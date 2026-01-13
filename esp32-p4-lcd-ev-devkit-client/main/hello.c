/**
 * @file hello.c
 * @brief IotCraft ESP32-P4 Client - Main entry point
 */

#include "raylib.h"
#include "board_init.h"
#include "esp_raylib_port.h"
#include "esp_lcd_panel_io.h"
#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_task_wdt.h"
#include <inttypes.h>
#include <math.h>

// IotCraft components
#include "iotcraft_types.h"
#include "world.h"
#include "camera.h"
#include "renderer.h"
#include "input.h"
#include "game.h"
#include "usb_keyboard.h"
#include "iotcraft_mqtt.h"
#include "network_init.h"
#include "world_template.h"

#define TAG "IotCraftClient"
#define RAYLIB_TASK_STACK_SIZE (160 * 1024)  // 160KB stack for renderer (increased from 128KB)
#define RENDER_WIDTH 320
#define RENDER_HEIGHT 240

// Global state
static voxel_world_t g_world;
static camera_t g_camera;
static renderer_t g_renderer;
static game_state_t g_game;

// Convert block_type_t to string for MQTT messages
static const char* block_type_to_string(block_type_t type) {
    switch (type) {
        case BLOCK_AIR: return "air";
        case BLOCK_GRASS: return "grass";
        case BLOCK_DIRT: return "dirt";
        case BLOCK_STONE: return "stone";
        case BLOCK_QUARTZ: return "quartz";
        case BLOCK_GLASS: return "glass";
        case BLOCK_TERRACOTTA: return "terracotta";
        case BLOCK_WATER: return "water";
        default: return "unknown";
    }
}

void iotcraft_task(void *pvParameter)
{
    ESP_LOGI(TAG, "Initializing IotCraft client...");

    // Query actual display dimensions from port
    uint16_t displayWidth = 320;
    uint16_t displayHeight = 240;
    esp_err_t ret = ray_port_get_dimensions(&displayWidth, &displayHeight);
    if (ret != ESP_OK) {
        ESP_LOGW(TAG, "Failed to get display dimensions, using defaults");
    }

    ESP_LOGI(TAG, "Display dimensions: %dx%d", displayWidth, displayHeight);
    ESP_LOGI(TAG, "Rendering at: %dx%d", RENDER_WIDTH, RENDER_HEIGHT);

    // CRITICAL: Initialize Raylib window (allocates buffers in PSRAM/IRAM properly)
    ESP_LOGI(TAG, "Initializing Raylib window...");
    InitWindow(displayWidth, displayHeight, "IotCraft 3D Client");
    ESP_LOGI(TAG, "Raylib window initialized successfully");

    // Initialize USB HID keyboard
    ESP_LOGI(TAG, "Initializing USB HID keyboard...");
    if (usb_keyboard_init()) {
        ESP_LOGI(TAG, "USB keyboard initialized (waiting for connection)");
    } else {
        ESP_LOGW(TAG, "USB keyboard initialization failed, continuing without keyboard");
    }

    // Initialize input system
    if (!input_init()) {
        ESP_LOGE(TAG, "Failed to initialize input system");
        vTaskDelete(NULL);
        return;
    }

    // Initialize world
    if (!world_init(&g_world)) {
        ESP_LOGE(TAG, "Failed to initialize world");
        vTaskDelete(NULL);
        return;
    }

    // Load medieval world template (castle, village, forest)
    ESP_LOGI(TAG, "Loading medieval world template...");
    if (!world_load_medieval_template(&g_world, &g_camera)) {
        ESP_LOGE(TAG, "Failed to load medieval template, using fallback");
        // Fallback to simple terrain if template fails
        world_generate_test_terrain(&g_world);
        camera_init(&g_camera);
    } else {
        ESP_LOGI(TAG, "Medieval world loaded successfully!");
    }

    // Debug: Check if there are actually blocks around the camera
    ESP_LOGI(TAG, "");
    ESP_LOGI(TAG, "WORLD DEBUG: Checking blocks near camera...");
    ESP_LOGI(TAG, "  Camera position: (%.1f, %.1f, %.1f)", g_camera.x, g_camera.y, g_camera.z);
    ESP_LOGI(TAG, "  Camera yaw: %.2f radians", g_camera.yaw);

    int32_t cam_x = (int32_t)floorf(g_camera.x);
    int32_t cam_y = (int32_t)floorf(g_camera.y);
    int32_t cam_z = (int32_t)floorf(g_camera.z);

    // Check 5x5 area around camera
    int blocks_found = 0;
    for (int32_t dx = -2; dx <= 2; dx++) {
        for (int32_t dz = -2; dz <= 2; dz++) {
            for (int32_t dy = 0; dy < 5; dy++) {
                int32_t check_x = cam_x + dx;
                int32_t check_y = cam_y + dy;
                int32_t check_z = cam_z + dz;
                block_type_t block = world_get_block(&g_world, check_x, check_y, check_z);
                if (block != BLOCK_AIR) {
                    blocks_found++;
                    if (blocks_found <= 10) {  // Log first 10 blocks
                        ESP_LOGI(TAG, "  Block at (%d, %d, %d): %d", check_x, check_y, check_z, block);
                    }
                }
            }
        }
    }
    ESP_LOGI(TAG, "  Total blocks found in 5x5x5 area: %d", blocks_found);
    ESP_LOGI(TAG, "");

    // Initialize renderer
    if (!renderer_init(&g_renderer, RENDER_WIDTH, RENDER_HEIGHT, &g_camera, &g_world)) {
        ESP_LOGE(TAG, "Failed to initialize renderer");
        world_free(&g_world);
        vTaskDelete(NULL);
        return;
    }

    // Initialize game
    if (!game_init(&g_game, &g_camera, &g_world)) {
        ESP_LOGE(TAG, "Failed to initialize game");
        renderer_free(&g_renderer);
        world_free(&g_world);
        vTaskDelete(NULL);
        return;
    }

    ESP_LOGI(TAG, "IotCraft client initialized successfully");
    ESP_LOGI(TAG, "Controls: WASD=Move, Arrows=Rotate, Space/Shift=Up/Down");
    ESP_LOGI(TAG, "Debug: 'C'=Chessboard test, 'H'=Toggle help, 'E'=Exit");

    // Debug flags
    bool show_chessboard = false;  // Disabled by default, press 'C' to enable
    bool show_help = true;
    bool chessboard_done = !show_chessboard;  // Skip if not enabled

    // ============================================================
    // DISPLAY PIPELINE TEST: Chessboard Pattern (OPTIONAL)
    // ============================================================
    // This verifies the Raylib display pipeline is working correctly
    // Using BeginDrawing()/EndDrawing() ensures proper MIPI DSI timing
    // Press 'C' key during startup to enable this test
    if (show_chessboard) {
        ESP_LOGI(TAG, "");
        ESP_LOGI(TAG, "========================================");
        ESP_LOGI(TAG, "DISPLAY TEST: Chessboard (5 seconds)");
        ESP_LOGI(TAG, "========================================");
        ESP_LOGI(TAG, "You should see a 10x10px black/white pattern");
        ESP_LOGI(TAG, "");
    } else {
        ESP_LOGI(TAG, "");
        ESP_LOGI(TAG, "Chessboard test disabled (press 'C' during startup to enable)");
        ESP_LOGI(TAG, "");
    }

    const int tileSize = 10;
    int testFrames = 0;
    const int testDuration = 5 * 20;  // 5 seconds at 20 FPS

    while (!chessboard_done && testFrames < testDuration) {
        // Check for 'C' key to enable chessboard test during startup
        if (testFrames == 0) {
            vTaskDelay(pdMS_TO_TICKS(100));  // Wait 100ms for initial key press
            input_poll();
            if (usb_keyboard_is_connected() && IsKeyPressed(KEY_C)) {
                show_chessboard = true;
                ESP_LOGI(TAG, "Chessboard test enabled via 'C' key");
            }
        }

        if (!show_chessboard) {
            chessboard_done = true;  // Skip the test
            break;
        }

        BeginDrawing();

        // Draw 10x10 pixel chessboard pattern
        for (int y = 0; y < g_renderer.height; y += tileSize) {
            for (int x = 0; x < g_renderer.width; x += tileSize) {
                // Alternate black and white tiles
                int tileX = x / tileSize;
                int tileY = y / tileSize;
                Color color = ((tileX + tileY) % 2 == 0) ? BLACK : WHITE;
                DrawRectangle(x, y, tileSize, tileSize, color);
            }
        }

        // Draw test info overlay
        DrawText("DISPLAY TEST", 10, 10, 20, RED);
        DrawText(TextFormat("Frame: %d/%d", testFrames, testDuration), 10, 35, 15, RED);
        DrawText("10x10px Chessboard", 10, 55, 12, DARKGREEN);

        EndDrawing();
        vTaskDelay(pdMS_TO_TICKS(50));  // 20 FPS
        testFrames++;
    }

    if (show_chessboard) {
        ESP_LOGI(TAG, "");
        ESP_LOGI(TAG, "Chessboard test complete!");
        ESP_LOGI(TAG, "If you saw the pattern, display pipeline is working.");
        ESP_LOGI(TAG, "========================================");
        ESP_LOGI(TAG, "");
    }

    // ============================================================
    // START 3D RENDERER
    // ============================================================
    ESP_LOGI(TAG, "");
    ESP_LOGI(TAG, "========================================");
    ESP_LOGI(TAG, "3D RENDERER STARTED");
    ESP_LOGI(TAG, "========================================");
    ESP_LOGI(TAG, "DEBUG: Check display for:");
    ESP_LOGI(TAG, "  - Top: RGBW color bars (R/G/B/W)");
    ESP_LOGI(TAG, "  - Top-right: White 'F' character");
    ESP_LOGI(TAG, "  - Center: 3D medieval world");
    ESP_LOGI(TAG, "");
    ESP_LOGI(TAG, "  See bars + 'F' but NO walls? -> Renderer bug");
    ESP_LOGI(TAG, "  NO bars or 'F'? -> Display timing bug");
    ESP_LOGI(TAG, "========================================");
    ESP_LOGI(TAG, "");

    // Add task to watchdog so it doesn't reset us
    esp_err_t wdt_ret = esp_task_wdt_add(NULL);
    if (wdt_ret != ESP_OK) {
        ESP_LOGW(TAG, "Failed to add task to WDT: %d (may crash in simulator)", wdt_ret);
    } else {
        ESP_LOGI(TAG, "Task added to watchdog successfully");
    }

    // TODO: Network and MQTT temporarily disabled for rendering testing
    // Initialize network (Ethernet)
    // ESP_LOGI(TAG, "Initializing network (Ethernet)...");
    // esp_err_t net_ret = network_init();
    // if (net_ret == ESP_OK) {
    //     char ip_str[16];
    //     if (network_get_ip(ip_str, sizeof(ip_str)) == ESP_OK) {
    //         ESP_LOGI(TAG, "Network ready! IP: %s", ip_str);
    //     }
    // } else {
    //     ESP_LOGW(TAG, "Network initialization failed: %d (continuing without network)", net_ret);
    // }

    // Initialize MQTT client for multiplayer/device interaction
    // ESP_LOGI(TAG, "Initializing MQTT client...");
    // esp_err_t mqtt_ret = iotcraft_mqtt_init("test-world", NULL);  // Use default broker
    // if (mqtt_ret == ESP_OK) {
    //     ESP_LOGI(TAG, "MQTT client initialized (connecting in background)");
    // } else {
    //     ESP_LOGW(TAG, "MQTT client initialization failed (continuing without multiplayer)");
    // }

    ESP_LOGI(TAG, "Network/MQTT disabled for testing - running in standalone mode");

    // Main rendering loop
    int frameCounter = 0;
    TickType_t last_time = xTaskGetTickCount();

    // Auto-movement for testing (when no keyboard connected)
    float auto_rotate = 0.0f;

    // Block interaction demo variables
    int demo_frame = 0;
    const int demo_cycle = 180;  // 6 seconds at 30 FPS
    block_type_t current_block_type = BLOCK_STONE;

    bool wdt_enabled = (wdt_ret == ESP_OK);  // Track if WDT is available

    while (g_game.running) {
        // Feed watchdog to prevent resets (only if we successfully added to WDT)
        if (wdt_enabled) {
            esp_task_wdt_reset();
        }

        // Yield to other tasks (important for simulator)
        vTaskDelay(pdMS_TO_TICKS(1));

        // Calculate delta time
        TickType_t current_time = xTaskGetTickCount();
        float delta_time = (current_time - last_time) / 1000.0f;  // Convert to seconds
        last_time = current_time;

        // Poll input (check for key presses)
        input_poll();

        // Handle debug keys
        if (usb_keyboard_is_connected()) {
            if (IsKeyPressed(KEY_H)) {
                show_help = !show_help;
                ESP_LOGI(TAG, "Help overlay: %s", show_help ? "ON" : "OFF");
            }
            if (IsKeyPressed(KEY_E)) {
                ESP_LOGI(TAG, "Exit requested via 'E' key");
                g_game.running = false;
                break;
            }
        }

        // Auto-rotate camera if no keyboard connected (for testing)
        if (!usb_keyboard_is_connected()) {
            // DISABLED: Keep camera stationary to debug raycasting
            // auto_rotate += delta_time * 0.2f;  // Slower rotation: 0.2 radians per second (was 0.5)
            // g_camera.yaw = auto_rotate;
            ESP_LOGI(TAG, "Auto-rotate DISABLED for debugging");
            ESP_LOGI(TAG, "Camera staying at yaw=%.2f", g_camera.yaw);

            // Stay in center but higher up for better view
            // Camera already at good position from template (tp -15 8 20)

            // Block interaction demo
            demo_frame++;
            int phase = (demo_frame / demo_cycle) % 4;

            if (phase == 0 && demo_frame % demo_cycle == 0) {
                // Place a block every 6 seconds
                int32_t px, py, pz;
                if (world_get_place_position(&g_world, &g_camera, &px, &py, &pz)) {
                    // Cycle through block types
                    static const block_type_t block_types[] = {
                        BLOCK_STONE, BLOCK_DIRT, BLOCK_QUARTZ,
                        BLOCK_GLASS, BLOCK_TERRACOTTA
                    };
                    current_block_type = block_types[(demo_frame / demo_cycle) % 5];
                    world_set_block(&g_world, px, py, pz, current_block_type);

                    // Publish block placement to MQTT (disabled for now)
                    // if (iotcraft_mqtt_is_connected()) {
                    //     iotcraft_mqtt_publish_block_placed(px, py, pz, block_type_to_string(current_block_type));
                    // }
                }
            } else if (phase == 2 && demo_frame % demo_cycle == 0) {
                // Remove a block every 6 seconds
                int32_t tx, ty, tz;
                if (world_get_target_block(&g_world, &g_camera, &tx, &ty, &tz)) {
                    world_set_block(&g_world, tx, ty, tz, BLOCK_AIR);

                    // Publish block removal to MQTT (disabled for now)
                    // if (iotcraft_mqtt_is_connected()) {
                    //     iotcraft_mqtt_publish_block_removed(tx, ty, tz);
                    // }
                }
            }
        }

        // Update game logic
        game_update(&g_game, delta_time);

        // Render a frame
        renderer_render_frame(&g_renderer);

        // Get framebuffer and dimensions
        const uint16_t* fb = renderer_get_framebuffer(&g_renderer);
        int32_t fb_width, fb_height;
        renderer_get_dimensions(&g_renderer, &fb_width, &fb_height);

        // Draw debug text to verify display is working
        // If we see text but no walls, it's a renderer bug, not display timing
        if (fb && fb_width > 0 && fb_height > 0) {
            // Cast away const to draw debug overlay
            uint16_t* fb_writable = (uint16_t*)fb;

            // Draw colored test bars at the top
            for (int x = 0; x < fb_width && x < 80; x++) {
                // Red bar (x: 0-20)
                if (x < 20) fb_writable[x] = 0xF800;           // RED
                // Green bar (x: 20-40)
                else if (x < 40) fb_writable[x] = 0x07E0;      // GREEN
                // Blue bar (x: 40-60)
                else if (x < 60) fb_writable[x] = 0x001F;      // BLUE
                // White bar (x: 60-80)
                else fb_writable[x] = 0xFFFF;                  // WHITE
            }

            // Draw vertical test bars on left side (to verify vertical rendering)
            for (int y = 0; y < fb_height && y < 80; y++) {
                uint16_t color;
                if (y < 20) color = 0xF800;      // RED
                else if (y < 40) color = 0x07E0; // GREEN
                else if (y < 60) color = 0x001F; // BLUE
                else color = 0xFFFF;              // WHITE
                fb_writable[y * fb_width + 0] = color;
            }

            // Draw frame counter in top-right corner
            if (frameCounter % 10 == 0) {
                int text_x = fb_width - 40;
                int text_y = 10;

                // Simple "F" for Frame indicator (5x7 pixels)
                const char frame_pattern[35] = {  // 5x7 "F"
                    1,1,1,1,1,
                    1,0,0,0,0,
                    1,1,1,0,0,
                    1,0,0,0,0,
                    1,0,0,0,0,
                    1,0,0,0,0,
                    1,0,0,0,0
                };

                for (int py = 0; py < 7; py++) {
                    for (int px = 0; px < 5; px++) {
                        if (frame_pattern[py * 5 + px]) {
                            int draw_x = text_x + px;
                            int draw_y = text_y + py;
                            if (draw_x < fb_width && draw_y < fb_height) {
                                fb_writable[draw_y * fb_width + draw_x] = 0xFFFF;  // White
                            }
                        }
                    }
                }
            }
        }

        // Push framebuffer to display
        if (fb && fb_width > 0 && fb_height > 0) {
            esp_err_t ret = board_display_push_frame(fb, fb_width, fb_height);
            if (ret != ESP_OK) {
                // Only log errors occasionally to avoid spam
                if (frameCounter % 30 == 0) {
                    ESP_LOGW(TAG, "Failed to push frame to display: %d", ret);
                }
            }
        }

        // Log every 30 frames to show progress (only if help enabled)
        if (show_help && frameCounter % 30 == 0) {
            ESP_LOGI(TAG, "Frame %d: %dx%d, %d blocks, pos:(%.1f,%.1f,%.1f) yaw:%.2f",
                     frameCounter, fb_width, fb_height, g_world.count,
                     g_camera.x, g_camera.y, g_camera.z, g_camera.yaw);

            // Verify framebuffer has non-sky colors (walls rendered)
            if (fb && fb_width > 0 && fb_height > 0) {
                uint16_t center_pixel = fb[(fb_height / 2) * fb_width + (fb_width / 2)];
                uint16_t sky_color = 0x867d;  // COLOR_SKY from renderer.c
                if (center_pixel != sky_color) {
                    ESP_LOGI(TAG, "  ✓ Center pixel: 0x%04x (NOT sky color - walls rendered!)", center_pixel);
                } else {
                    ESP_LOGW(TAG, "  ✗ Center pixel: 0x%04x (sky color - no walls at center)", center_pixel);
                }
            }
        }

        frameCounter++;

        // Frame rate control - slower for stability (20 FPS)
        vTaskDelay(pdMS_TO_TICKS(50));
    }

    // Cleanup (never reached in embedded mode)
    ESP_LOGI(TAG, "Shutting down...");
    CloseWindow();  // Properly cleanup Raylib resources
    renderer_free(&g_renderer);
    world_free(&g_world);
    vTaskDelete(NULL);
}

void app_main(void)
{
    ESP_LOGI(TAG, "Initializing board display...");

    // Initialize display hardware and port layer
    esp_err_t ret = board_init_display();
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to initialize display: %d", ret);
        return;
    }

    ESP_LOGI(TAG, "Creating IotCraft task with %dKB stack...",
             RAYLIB_TASK_STACK_SIZE / 1024);

    // Create dedicated task for IotCraft client
    // IMPORTANT: Run on Core 0 (same as app_main) to share Raylib context
    xTaskCreatePinnedToCore(
        iotcraft_task,            // Task function
        "iotcraft_task",          // Task name
        RAYLIB_TASK_STACK_SIZE,   // Stack size in bytes
        NULL,                     // Parameters
        5,                        // Priority
        NULL,                     // Task handle
        0                         // Core ID (run on core 0 - same as Raylib init)
    );
}
