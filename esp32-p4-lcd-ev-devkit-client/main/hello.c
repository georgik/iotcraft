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
#include "trig_lut.h"
#include "device_manager.h"
#include "texture_loader.h"

// Console overlay
#include "console.h"
#include "console_commands.h"

#define TAG "IotCraftClient"
#define RAYLIB_TASK_STACK_SIZE (160 * 1024)  // 160KB stack for main renderer task
#define CORE1_RENDER_STACK_SIZE (4 * 1024)    // 4KB stack for Core 1 render task (only collects & sorts voxels, no deep call stacks)
#define RENDER_WIDTH 512    // Half resolution (4x fewer pixels = 4x faster rendering)
#define RENDER_HEIGHT 300   // Half resolution (will be scaled to 1024x600 by display)
#define DISPLAY_WIDTH 1024  // Physical display size
#define DISPLAY_HEIGHT 600  // Physical display size

// Global state
static voxel_world_t g_world;
static camera_t g_camera;
static renderer_t g_renderer;
static game_state_t g_game;

// Multi-core rendering synchronization
static SemaphoreHandle_t g_render_start_sem = NULL;  // Signals Core 1 to start
static SemaphoreHandle_t g_render_done_sem = NULL;   // Signals Core 0 that Core 1 finished
static volatile bool g_core1_ready = false;
static TaskHandle_t g_core1_task_handle = NULL;
static voxel_buffer_t g_core1_voxel_buffer;  // Core 1's voxel buffer (right world space)

// Callback for when network IP is acquired
static void on_got_ip(const char* ip) {
    ESP_LOGI(TAG, "✓ Network connected! IP: %s", ip);

    // Log to console for diagnostics
    console_log(LOG_LEVEL_INFO, "NETWORK", "IP: %s", ip);
    console_log(LOG_LEVEL_INFO, "NETWORK", "Connecting to MQTT broker...");

    // Initialize MQTT client when network is ready
    ESP_LOGI(TAG, "Initializing MQTT client...");
    esp_err_t mqtt_ret = iotcraft_mqtt_init("test-world", NULL);
    if (mqtt_ret == ESP_OK) {
        ESP_LOGI(TAG, "✓ MQTT client initialized (connecting in background)");
        console_log(LOG_LEVEL_INFO, "NETWORK", "MQTT init: connecting...");
    } else {
        ESP_LOGW(TAG, "✗ MQTT client initialization failed (continuing without multiplayer)");
        console_log(LOG_LEVEL_ERROR, "NETWORK", "MQTT init failed!");
    }
}

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

// ============================================================
// MULTI-CORE RENDERING: Core 1 Task (Right World Space)
// ============================================================
/**
 * @brief Core 1 rendering task - collects and sorts voxels in right world space
 * @param pvParameter Unused
 *
 * This task runs on Core 1 and processes voxels where x >= camera_x
 * It uses world-space splitting to maintain painter's algorithm correctness
 */
static void core1_render_task(void *pvParameter) {
    ESP_LOGI(TAG, "Core 1 render task started (3D voxel renderer - right world space)");
    g_core1_ready = true;

    // Wait for render requests from Core 0
    while (true) {
        // Wait for start signal from Core 0
        if (xSemaphoreTake(g_render_start_sem, portMAX_DELAY) == pdTRUE) {
            // Get camera position for world-space split
            int32_t cam_x = (int32_t)floorf(g_renderer.camera->x);

            // Collect voxels in right world space (x >= cam_x)
            renderer_collect_voxels_parallel(&g_renderer, &g_core1_voxel_buffer, cam_x, INT32_MAX);

            // Sort by depth (far to near) using fixed-point arithmetic
            renderer_sort_voxel_buffer(&g_core1_voxel_buffer,
                                      g_renderer.camera->x,
                                      g_renderer.camera->y,
                                      g_renderer.camera->z);

            // Signal completion to Core 0
            xSemaphoreGive(g_render_done_sem);
        }
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

    // Initialize camera FIRST (sets FOV and defaults)
    camera_init(&g_camera);

    if (!world_load_medieval_template(&g_world, &g_camera)) {
        ESP_LOGE(TAG, "Failed to load medieval template, using fallback");
        // Fallback to simple terrain if template fails
        world_generate_test_terrain(&g_world);
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

    // Check 5x5x10 area around camera (including below)
    int blocks_found = 0;
    for (int32_t dx = -2; dx <= 2; dx++) {
        for (int32_t dz = -2; dz <= 2; dz++) {
            for (int32_t dy = -5; dy <= 5; dy++) {
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
    ESP_LOGI(TAG, "  Total blocks found in 5x5x10 area: %d", blocks_found);
    ESP_LOGI(TAG, "");

    // ============================================================
    // TEXTURE LOADING: Initialize texture system
    // ============================================================
    ESP_LOGI(TAG, "Initializing texture system...");
    esp_err_t tex_ret = texture_loader_init();
    if (tex_ret == ESP_OK) {
        bool using_sd = texture_loader_using_sdcard();
        ESP_LOGI(TAG, "✓ Texture system initialized (%s)",
                 using_sd ? "SD card" : "embedded fallback");
        // Note: Don't use console_log here - console not initialized yet
        // console_log() will be available after console_init() below
    } else {
        ESP_LOGW(TAG, "✗ Texture system initialization failed (using embedded textures)");
    }

    // Initialize renderer
    if (!renderer_init(&g_renderer, RENDER_WIDTH, RENDER_HEIGHT, &g_camera, &g_world)) {
        ESP_LOGE(TAG, "Failed to initialize renderer");
        world_free(&g_world);
        vTaskDelete(NULL);
        return;
    }

    // Set global renderer reference for debug block placement
    {
        extern renderer_t* g_global_renderer;
        g_global_renderer = &g_renderer;
    }

    // Initialize game
    if (!game_init(&g_game, &g_camera, &g_world)) {
        ESP_LOGE(TAG, "Failed to initialize game");
        renderer_free(&g_renderer);
        world_free(&g_world);
        vTaskDelete(NULL);
        return;
    }

    // ============================================================
    // CONSOLE OVERLAY: Initialize after renderer is ready
    // ============================================================
    ESP_LOGI(TAG, "Initializing console overlay...");
    console_init();
    console_register_builtin_commands();
    // Console is hidden by default - press F3 to show it

    // Show welcome message (will be visible when console is opened)
    console_log(LOG_LEVEL_INFO, "SYSTEM", "IoTCraft ESP32-P4 Client");
    console_log(LOG_LEVEL_INFO, "SYSTEM", "Press F3 to toggle console");
    console_log(LOG_LEVEL_INFO, "SYSTEM", "Press F4 to toggle blink");
    console_log(LOG_LEVEL_INFO, "SYSTEM", "Press ESC to close console");
    console_log(LOG_LEVEL_INFO, "SYSTEM", "Type 'help' for commands");
    // ============================================================

    // ============================================================
    // DEVICE MANAGER: Initialize IoT device tracking
    // ============================================================
    ESP_LOGI(TAG, "Initializing device manager...");
    device_manager_init();
    // ============================================================

    ESP_LOGI(TAG, "IotCraft client initialized successfully");
    ESP_LOGI(TAG, "Controls: WASD=Move (Spectator mode), Arrows=Look, Q=Fly Up, E=Fly Down");
    ESP_LOGI(TAG, "Debug: 'C'=Chessboard test, 'H'=Toggle help, ESC=Exit");

    // Initialize trigonometric lookup tables for fast rendering
    trig_lut_init();

    // ============================================================
    // MULTI-CORE RENDERING: Initialize
    // ============================================================
    ESP_LOGI(TAG, "Initializing multi-core rendering...");

    // Create binary semaphores for synchronization
    g_render_start_sem = xSemaphoreCreateBinary();
    if (!g_render_start_sem) {
        ESP_LOGE(TAG, "Failed to create render start semaphore");
        renderer_free(&g_renderer);
        world_free(&g_world);
        vTaskDelete(NULL);
        return;
    }

    g_render_done_sem = xSemaphoreCreateBinary();
    if (!g_render_done_sem) {
        ESP_LOGE(TAG, "Failed to create render done semaphore");
        vSemaphoreDelete(g_render_start_sem);
        renderer_free(&g_renderer);
        world_free(&g_world);
        vTaskDelete(NULL);
        return;
    }
    ESP_LOGI(TAG, "Render semaphores created");

    // Check available heap before creating Core 1 task
    size_t free_heap = esp_get_free_heap_size();
    size_t min_free_heap = esp_get_minimum_free_heap_size();
    ESP_LOGI(TAG, "Free heap before Core 1 task: %zu bytes (min: %zu bytes)",
             free_heap, min_free_heap);

    // ============================================================
    // ENABLED: Multi-core rendering (dual-core 3D voxel renderer)
    // ============================================================
    // Create Core 1 rendering task (right world space)
    ESP_LOGI(TAG, "Creating Core 1 render task with %d KB stack...",
             CORE1_RENDER_STACK_SIZE / 1024);

    BaseType_t task_ret = xTaskCreatePinnedToCore(
        core1_render_task,           // Task function
        "core1_render",              // Task name
        CORE1_RENDER_STACK_SIZE,     // Stack size
        NULL,                        // Parameters
        5,                           // Priority (same as main task)
        &g_core1_task_handle,        // Task handle
        1                            // Core ID (run on Core 1)
    );

    if (task_ret != pdPASS) {
        ESP_LOGE(TAG, "Failed to create Core 1 render task (ret=%d)", task_ret);
        ESP_LOGE(TAG, "  Available heap: %zu bytes", esp_get_free_heap_size());
        ESP_LOGE(TAG, "  Required stack: %d bytes", CORE1_RENDER_STACK_SIZE);
        vSemaphoreDelete(g_render_start_sem);
        vSemaphoreDelete(g_render_done_sem);
        renderer_free(&g_renderer);
        world_free(&g_world);
        vTaskDelete(NULL);
        return;
    }

    // Wait for Core 1 task to be ready
    int wait_count = 0;
    while (!g_core1_ready && wait_count < 100) {
        vTaskDelay(pdMS_TO_TICKS(10));
    }

    if (!g_core1_ready) {
        ESP_LOGE(TAG, "Core 1 render task failed to initialize");
        vSemaphoreDelete(g_render_start_sem);
        vSemaphoreDelete(g_render_done_sem);
        renderer_free(&g_renderer);
        world_free(&g_world);
        vTaskDelete(NULL);
        return;
    }
    // ============================================================

    ESP_LOGI(TAG, "Dual-core 3D renderer initialized (Core 0: left space, Core 1: right space)");


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

    ESP_LOGI(TAG, "Running in networked mode (Ethernet + MQTT)");

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

    // Simple font for debug HUD (5x7 digits)
    static const char digit_font[10][35] = {
        // 0
        {0,1,1,1,0,
         1,0,0,0,1,
         1,0,0,0,1,
         1,0,0,0,1,
         1,0,0,0,1,
         1,0,0,0,1,
         0,1,1,1,0},
        // 1
        {0,0,1,0,0,
         0,1,1,0,0,
         0,0,1,0,0,
         0,0,1,0,0,
         0,0,1,0,0,
         0,0,1,0,0,
         0,1,1,1,0},
        // 2
        {0,1,1,1,0,
         1,0,0,0,1,
         0,0,0,0,1,
         0,0,0,1,0,
         0,0,1,0,0,
         0,1,0,0,0,
         1,1,1,1,1},
        // 3
        {0,1,1,1,0,
         1,0,0,0,1,
         0,0,0,0,1,
         0,0,1,1,0,
         0,0,0,0,1,
         1,0,0,0,1,
         0,1,1,1,0},
        // 4
        {0,0,0,1,0,
         0,0,1,1,0,
         0,1,0,1,0,
         1,0,0,1,0,
         1,1,1,1,1,
         0,0,0,1,0,
         0,0,0,1,0},
        // 5
        {1,1,1,1,1,
         1,0,0,0,0,
         1,1,1,1,0,
         0,0,0,0,1,
         0,0,0,0,1,
         1,0,0,0,1,
         0,1,1,1,0},
        // 6
        {0,1,1,1,0,
         1,0,0,0,0,
         1,1,1,1,0,
         1,0,0,0,1,
         1,0,0,0,1,
         1,0,0,0,1,
         0,1,1,1,0},
        // 7
        {1,1,1,1,1,
         0,0,0,0,1,
         0,0,0,1,0,
         0,0,1,0,0,
         0,0,1,0,0,
         0,0,1,0,0,
         0,0,1,0,0},
        // 8
        {0,1,1,1,0,
         1,0,0,0,1,
         1,0,0,0,1,
         0,1,1,1,0,
         1,0,0,0,1,
         1,0,0,0,1,
         0,1,1,1,0},
        // 9
        {0,1,1,1,0,
         1,0,0,0,1,
         1,0,0,0,1,
         0,1,1,1,1,
         0,0,0,0,1,
         1,0,0,0,1,
         0,1,1,1,0}
    };

    // Helper function to draw integer on framebuffer
    auto void draw_int(uint16_t* fb, int32_t fb_width, int32_t fb_height,
                       int x, int y, int value, uint16_t color) {
        char buf[16];
        int len = 0;
        int tmp = value;

        // Handle negative
        if (value < 0) {
            tmp = -value;
        }

        // Convert to string (reversed)
        do {
            buf[len++] = '0' + (tmp % 10);
            tmp /= 10;
        } while (tmp > 0);

        if (value < 0) {
            buf[len++] = '-';
        }

        // Draw digits (in reverse order)
        int draw_x = x;
        for (int i = len - 1; i >= 0; i--) {
            int digit = buf[i] - '0';
            if (digit >= 0 && digit <= 9) {
                const char* glyph = digit_font[digit];
                for (int py = 0; py < 7; py++) {
                    for (int px = 0; px < 5; px++) {
                        if (glyph[py * 5 + px]) {
                            int draw_x_pos = draw_x + px;
                            int draw_y_pos = y + py;
                            if (draw_x_pos < fb_width && draw_y_pos < fb_height) {
                                fb[draw_y_pos * fb_width + draw_x_pos] = color;
                            }
                        }
                    }
                }
            }
            draw_x += 6;  // 5 pixels + 1 space
        }
    }

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

        // Update console overlay
        console_update();

        // Update device manager (spawns/despawns IoT devices)
        device_manager_update(&g_world, delta_time * 1000.0f);

        // Poll input (check for key presses)
        input_poll();

        // Update game key states from USB keyboard
        if (usb_keyboard_is_connected()) {
            // Map USB keyboard input to game actions
            // Log which keys are being pressed (only when any key is active)
            bool any_key = false;
            if (input_is_key_pressed(IOTCRAFT_KEY_W) ||
                input_is_key_pressed(IOTCRAFT_KEY_A) ||
                input_is_key_pressed(IOTCRAFT_KEY_S) ||
                input_is_key_pressed(IOTCRAFT_KEY_D) ||
                input_is_key_pressed(IOTCRAFT_KEY_LEFT) ||
                input_is_key_pressed(IOTCRAFT_KEY_RIGHT) ||
                input_is_key_pressed(IOTCRAFT_KEY_UP) ||
                input_is_key_pressed(IOTCRAFT_KEY_DOWN)) {
                any_key = true;
            }

            static int key_log_count = 0;
            if (any_key && ++key_log_count >= 10) {  // Log every 10th frame with keys pressed (2x per second)
                const char *keys = "";
                if (input_is_key_pressed(IOTCRAFT_KEY_W)) keys = "W";
                else if (input_is_key_pressed(IOTCRAFT_KEY_S)) keys = "S";
                else if (input_is_key_pressed(IOTCRAFT_KEY_A)) keys = "A";
                else if (input_is_key_pressed(IOTCRAFT_KEY_D)) keys = "D";
                else if (input_is_key_pressed(IOTCRAFT_KEY_LEFT)) keys = "LEFT";
                else if (input_is_key_pressed(IOTCRAFT_KEY_RIGHT)) keys = "RIGHT";
                else if (input_is_key_pressed(IOTCRAFT_KEY_UP)) keys = "UP";
                else if (input_is_key_pressed(IOTCRAFT_KEY_DOWN)) keys = "DOWN";
                else keys = "MULTIPLE";

                ESP_LOGI(TAG, "Processing key: %s | Camera: (%.1f, %.1f, %.1f) yaw=%.2f",
                         keys, g_camera.x, g_camera.y, g_camera.z, g_camera.yaw);
                key_log_count = 0;
            }

            game_handle_key(&g_game, IOTCRAFT_KEY_W, input_is_key_pressed(IOTCRAFT_KEY_W));
            game_handle_key(&g_game, IOTCRAFT_KEY_S, input_is_key_pressed(IOTCRAFT_KEY_S));
            game_handle_key(&g_game, IOTCRAFT_KEY_A, input_is_key_pressed(IOTCRAFT_KEY_A));
            game_handle_key(&g_game, IOTCRAFT_KEY_D, input_is_key_pressed(IOTCRAFT_KEY_D));
            game_handle_key(&g_game, IOTCRAFT_KEY_LEFT, input_is_key_pressed(IOTCRAFT_KEY_LEFT));
            game_handle_key(&g_game, IOTCRAFT_KEY_RIGHT, input_is_key_pressed(IOTCRAFT_KEY_RIGHT));
            game_handle_key(&g_game, IOTCRAFT_KEY_UP, input_is_key_pressed(IOTCRAFT_KEY_UP));
            game_handle_key(&g_game, IOTCRAFT_KEY_DOWN, input_is_key_pressed(IOTCRAFT_KEY_DOWN));
            game_handle_key(&g_game, IOTCRAFT_KEY_Q, input_is_key_pressed(IOTCRAFT_KEY_Q));
            game_handle_key(&g_game, IOTCRAFT_KEY_E, input_is_key_pressed(IOTCRAFT_KEY_E));
            game_handle_key(&g_game, IOTCRAFT_KEY_N, input_is_key_pressed(IOTCRAFT_KEY_N));
            game_handle_key(&g_game, IOTCRAFT_KEY_M, input_is_key_pressed(IOTCRAFT_KEY_M));
            game_handle_key(&g_game, IOTCRAFT_KEY_F3, input_is_key_pressed(IOTCRAFT_KEY_F3));
            game_handle_key(&g_game, IOTCRAFT_KEY_F5, input_is_key_pressed(IOTCRAFT_KEY_F5));
            game_handle_key(&g_game, IOTCRAFT_KEY_F6, input_is_key_pressed(IOTCRAFT_KEY_F6));

            // Handle console controls
            // ESC closes console if it's visible
            static bool esc_was_pressed = false;
            bool esc_is_pressed = input_is_key_pressed(IOTCRAFT_KEY_ESCAPE);
            if (esc_is_pressed && !esc_was_pressed) {
                if (console_is_visible()) {
                    console_hide();
                }
                // ESC never quits on ESP32-P4 - it only controls console
            }
            esc_was_pressed = esc_is_pressed;

            // Toggle wireframe mode with F1
            static bool f1_was_pressed = false;
            bool f1_is_pressed = input_is_key_pressed(IOTCRAFT_KEY_F1);
            if (f1_is_pressed && !f1_was_pressed) {
                renderer_toggle_wireframe(-1);  // Toggle
            }
            f1_was_pressed = f1_is_pressed;

            // Toggle blink mode with F4
            static bool f4_was_pressed = false;
            static bool blink_mode = false;
            bool f4_is_pressed = input_is_key_pressed(IOTCRAFT_KEY_F4);
            if (f4_is_pressed && !f4_was_pressed) {
                blink_mode = !blink_mode;
                int count = device_manager_blink_all(blink_mode);
                console_log(LOG_LEVEL_INFO, "BLINK", "Mode %s (%d devices)",
                           blink_mode ? "ON" : "OFF", count);
                ESP_LOGI(TAG, "Blink mode: %s (%d devices)", blink_mode ? "ON" : "OFF", count);
            }
            f4_was_pressed = f4_is_pressed;
        }

        // Handle debug keys (Raylib interface for 'H', 'E', 'C')
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
            // Only log occasionally (every 10 seconds)
            static int no_kbd_log_count = 0;
            if (++no_kbd_log_count >= 200) {  // 200 frames = ~10 seconds at 20 FPS
                ESP_LOGI(TAG, "No keyboard detected - camera stationary at yaw=%.2f", g_camera.yaw);
                no_kbd_log_count = 0;
            }

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

        // ============================================================
        // TRUE 3D RENDERING (shared with desktop-light)
        // ============================================================
#ifdef __ESP32_P4__
        // Check if wireframe mode is enabled
        if (renderer_is_wireframe_enabled()) {
            // Wireframe mode: use single-core renderer_render_frame
            // which has the wireframe rendering logic
            renderer_render_frame(&g_renderer);
        } else {
            // Normal mode: Use multi-core rendering for performance
            int32_t cam_x = (int32_t)floorf(g_renderer.camera->x);

            // Collect left voxels (x < cam_x)
            voxel_buffer_t left_buffer;
            renderer_collect_voxels_parallel(&g_renderer, &left_buffer, INT32_MIN, cam_x - 1);

            // Signal Core 1 to start collecting right voxels
            xSemaphoreGive(g_render_start_sem);

            // Sort left voxels while Core 1 collects (parallel execution)
            renderer_sort_voxel_buffer(&left_buffer,
                                      g_renderer.camera->x,
                                      g_renderer.camera->y,
                                      g_renderer.camera->z);

            // Clear framebuffer
            renderer_clear(&g_renderer, 0x0000);  // Black background

            // Render left voxels
            renderer_render_voxel_buffer(&g_renderer, &left_buffer);

            // Wait for Core 1 to finish collecting and sorting right voxels
            xSemaphoreTake(g_render_done_sem, portMAX_DELAY);

            // Render right voxels (collected and sorted by Core 1)
            renderer_render_voxel_buffer(&g_renderer, &g_core1_voxel_buffer);
        }
#else
        // Single-core rendering: Use original function
        renderer_render_frame(&g_renderer);
#endif
        // ============================================================

        // Get framebuffer and dimensions
        const uint16_t* fb = renderer_get_framebuffer(&g_renderer);
        int32_t fb_width, fb_height;
        renderer_get_dimensions(&g_renderer, &fb_width, &fb_height);

        // Writable framebuffer pointer for console overlay
        uint16_t* fb_writable = NULL;

        // Draw debug text to verify display is working
        // If we see text but no walls, it's a renderer bug, not display timing
        if (fb && fb_width > 0 && fb_height > 0) {
            // Cast away const to draw debug overlay
            fb_writable = (uint16_t*)fb;

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

            // Draw minimap in top-right corner (2D top-down view)
            const int minimap_size = 64;  // 64x64 pixel minimap
            const int minimap_x = fb_width - minimap_size - 5;
            const int minimap_y = 25;  // Below the "F" indicator

            // Scale factor: how many world units per minimap pixel
            const float minimap_scale = 2.0f;  // 2 world units = 1 minimap pixel

            // Camera position in minimap coordinates
            const int cam_minimap_x = minimap_size / 2;
            const int cam_minimap_y = minimap_size / 2;

            // Draw minimap background (semi-transparent dark)
            for (int my = 0; my < minimap_size; my++) {
                for (int mx = 0; mx < minimap_size; mx++) {
                    int draw_x = minimap_x + mx;
                    int draw_y = minimap_y + my;
                    if (draw_x < fb_width && draw_y < fb_height) {
                        fb_writable[draw_y * fb_width + draw_x] = 0x0000;  // Black
                    }
                }
            }

            // Draw blocks on minimap
            for (int my = 0; my < minimap_size; my++) {
                for (int mx = 0; mx < minimap_size; mx++) {
                    // Convert minimap pixel to world coordinates
                    float world_offset_x = (mx - cam_minimap_x) * minimap_scale;
                    float world_offset_z = (my - cam_minimap_y) * minimap_scale;

                    int32_t world_x = (int32_t)floorf(g_camera.x + world_offset_x);
                    int32_t world_z = (int32_t)floorf(g_camera.z + world_offset_z);

                    // Check for blocks at this position (search from bottom up)
                    bool found_block = false;
                    for (int32_t wy = 30; wy >= 0; wy--) {
                        block_type_t block = world_get_block(&g_world, world_x, wy, world_z);
                        if (block != BLOCK_AIR) {
                            // Color based on block type
                            uint16_t block_color;
                            switch (block) {
                                case BLOCK_GRASS: block_color = 0x07E0; break;  // Green
                                case BLOCK_DIRT:  block_color = 0x6A44; break;  // Brown
                                case BLOCK_STONE: block_color = 0xFFFF; break;  // White
                                case BLOCK_QUARTZ: block_color = 0xFFFF; break; // White
                                default: block_color = 0x8888; break;  // Gray
                            }

                            // Draw pixel on minimap
                            int draw_x = minimap_x + mx;
                            int draw_y = minimap_y + my;
                            if (draw_x < fb_width && draw_y < fb_height) {
                                fb_writable[draw_y * fb_width + draw_x] = block_color;
                            }
                            found_block = true;
                            break;
                        }
                    }
                }
            }

            // Draw camera direction arrow on minimap
            const int arrow_len = 8;
            const float dir_x = cosf(g_camera.yaw);
            const float dir_z = sinf(g_camera.yaw);
            for (int i = 0; i < arrow_len; i++) {
                int ax = cam_minimap_x + (int)(dir_x * i);
                int ay = cam_minimap_y + (int)(dir_z * i);
                if (ax >= 0 && ax < minimap_size && ay >= 0 && ay < minimap_size) {
                    int draw_x = minimap_x + ax;
                    int draw_y = minimap_y + ay;
                    if (draw_x < fb_width && draw_y < fb_height) {
                        fb_writable[draw_y * fb_width + draw_x] = 0xF800;  // Red arrow
                    }
                }
            }

            // Draw minimap border
            for (int i = 0; i < minimap_size; i++) {
                // Top border
                if (minimap_y < fb_height) {
                    fb_writable[minimap_y * fb_width + (minimap_x + i)] = 0xFFFF;  // White
                }
                // Bottom border
                if (minimap_y + minimap_size - 1 < fb_height) {
                    fb_writable[(minimap_y + minimap_size - 1) * fb_width + (minimap_x + i)] = 0xFFFF;
                }
                // Left border
                if (minimap_x < fb_width) {
                    fb_writable[(minimap_y + i) * fb_width + minimap_x] = 0xFFFF;
                }
                // Right border
                if (minimap_x + minimap_size - 1 < fb_width) {
                    fb_writable[(minimap_y + i) * fb_width + (minimap_x + minimap_size - 1)] = 0xFFFF;
                }
            }

            // Draw debug HUD (bottom-left corner)
            int hud_y = fb_height - 60;  // Start from bottom
            int hud_x = 10;
            uint16_t hud_color = 0xFFFF;  // White

            // Position: X, Y, Z
            draw_int(fb_writable, fb_width, fb_height, hud_x, hud_y, (int)g_camera.x, hud_color);
            hud_y += 10;
            draw_int(fb_writable, fb_width, fb_height, hud_x, hud_y, (int)g_camera.y, hud_color);
            hud_y += 10;
            draw_int(fb_writable, fb_width, fb_height, hud_x, hud_y, (int)g_camera.z, hud_color);
            hud_y += 10;

            // Yaw (multiply by 100 to show 2 decimal places)
            int yaw_scaled = (int)(g_camera.yaw * 100.0f);
            draw_int(fb_writable, fb_width, fb_height, hud_x, hud_y, yaw_scaled, hud_color);
        }

        // Render console overlay (on top of everything else)
        // Pass framebuffer so console can draw directly to it
        console_render(fb_writable, fb_width, fb_height);

        // Push framebuffer to display with hardware scaling
        // Render at 512x300, scale to 1024x600 by display hardware (free performance boost!)
        if (fb && fb_width > 0 && fb_height > 0) {
            esp_err_t ret = board_display_push_frame_scaled(fb, fb_width, fb_height, DISPLAY_WIDTH, DISPLAY_HEIGHT);
            if (ret != ESP_OK) {
                // Only log errors occasionally to avoid spam
                if (frameCounter % 30 == 0) {
                    ESP_LOGW(TAG, "Failed to push scaled frame to display: %d", ret);
                }
            }
        }

        // Log every 30 frames to show progress (only if help enabled)
        if (show_help && frameCounter % 30 == 0) {
            ESP_LOGI(TAG, "Frame %d: %dx%d, %d blocks, pos:(%.1f,%.1f,%.1f) yaw:%.2f",
                     frameCounter, fb_width, fb_height, g_world.count,
                     g_camera.x, g_camera.y, g_camera.z, g_camera.yaw);

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

    // Console will be initialized in iotcraft_task after renderer is ready

    // ============================================================
    // NETWORK CONNECTIVITY: Initialize Ethernet BEFORE renderer
    // ============================================================
    // IMPORTANT: Network initialization must happen BEFORE creating
    // the renderer task because the EMAC driver needs to allocate
    // its RX task while there's still sufficient heap memory available.
    // The renderer allocates large buffers which would cause EMAC task
    // creation to fail if network is initialized after.
    // ============================================================

    ESP_LOGI(TAG, "Initializing network (Ethernet)...");
    network_set_got_ip_callback(on_got_ip);
    esp_err_t net_ret = network_init();
    if (net_ret == ESP_OK) {
        ESP_LOGI(TAG, "✓ Ethernet started (waiting for DHCP in background)");
    } else {
        ESP_LOGW(TAG, "✗ Ethernet initialization failed: %s (continuing without network)", esp_err_to_name(net_ret));
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
