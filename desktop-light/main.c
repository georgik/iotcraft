/**
 * @file main.c
 * @brief IotCraft Desktop Simulator - Main Entry Point
 *
 * Desktop simulator for IotCraft 3D renderer using Raylib
 * Shares 100% of rendering code with ESP32-P4 version
 */

#define __DESKTOP_BUILD__ 1

#include <stdio.h>
#include <stdlib.h>
#include <signal.h>
#include <unistd.h>
#include <math.h>
#include <string.h>

// Raylib (desktop)
#include "raylib.h"

// Local
#include "cli_options.h"

// Console overlay
#include "console.h"
#include "console_commands.h"

// Shared components (from ESP32 version)
#include "iotcraft_types.h"
#include "camera.h"
#include "world.h"
#include "renderer.h"
#include "game.h"
#include "input.h"
#include "world_template.h"
#include "trig_lut.h"
#include "texture_loader.h"

#define TAG "IotCraftDesktop"

// Flag to control main loop
static volatile sig_atomic_t g_running = 1;
static camera_t g_camera = {0};  // Global camera for multi-screenshot rotation

// Signal handler for graceful shutdown
void signal_handler(int sig) {
    if (g_running) {
        printf("\n[%s] Received signal %d, shutting down...\n", TAG, sig);
        g_running = 0;
    }
}

/**
 * @brief Display chessboard test pattern
 * @param duration_ms How long to display (in milliseconds)
 */
static void display_chessboard_test(int duration_ms) {
    const int tileSize = 10;
    int frames = 0;
    int total_frames = duration_ms / 50;  // 50ms per frame = 20 FPS

    printf("[%s] Starting chessboard test (%d frames, %d ms)\n", TAG, total_frames, duration_ms);

    while (frames < total_frames && g_running) {
        BeginDrawing();

        // Draw 10x10 pixel chessboard pattern
        for (int y = 0; y < GetScreenHeight(); y += tileSize) {
            for (int x = 0; x < GetScreenWidth(); x += tileSize) {
                // Alternate black and white tiles
                int tileX = x / tileSize;
                int tileY = y / tileSize;
                Color color = ((tileX + tileY) % 2 == 0) ? BLACK : WHITE;
                DrawRectangle(x, y, tileSize, tileSize, color);
            }
        }

        // Draw test info overlay
        DrawText("DISPLAY TEST", 10, 10, 20, RED);
        DrawText(TextFormat("Frame: %d/%d", frames, total_frames), 10, 35, 15, RED);
        DrawText("10x10px Chessboard", 10, 55, 12, DARKGREEN);

        EndDrawing();

        frames++;
        usleep(50000);  // 50ms = 20 FPS
    }

    printf("[%s] Chessboard test complete (%d frames rendered)\n", TAG, frames);
}

/**
 * @brief Run the 3D renderer
 * @param options CLI options
 * @return 0 on success, -1 on error
 */
static int run_renderer(const cli_options_t* options) {
    printf("[%s] Initializing 3D renderer...\n", TAG);

    // Initialize console overlay
    printf("[%s] Initializing console overlay...\n", TAG);
    console_init();
    console_register_builtin_commands();
    console_show();

    // Show welcome message
    console_log(LOG_LEVEL_INFO, "SYSTEM", "IoTCraft Desktop Simulator");
    console_log(LOG_LEVEL_INFO, "SYSTEM", "Press F3 or ` to toggle console");
    console_log(LOG_LEVEL_INFO, "SYSTEM", "Type 'help' for available commands");

    // Initialize global camera (only on first call)
    static bool camera_initialized = false;
    if (!camera_initialized) {
        camera_init(&g_camera);
        camera_initialized = true;

        // Use CLI camera position if provided, otherwise use defaults
        if (!isnan(options->cam_x)) {
            g_camera.x = options->cam_x;
            g_camera.y = options->cam_y;
            g_camera.z = options->cam_z;
        } else {
            g_camera.x = -15.0f;
            g_camera.y = 2.0f;
            g_camera.z = 0.0f;
        }

        if (!isnan(options->cam_yaw)) {
            g_camera.yaw = options->cam_yaw;
            g_camera.pitch = options->cam_pitch;
        } else {
            g_camera.yaw = -0.78f;  // -45 degrees
            g_camera.pitch = -0.17f;  // -10 degrees
        }
    }

    // Initialize world
    voxel_world_t world = {0};
    if (!world_init(&world)) {
        fprintf(stderr, "[%s] Failed to initialize world\n", TAG);
        return -1;
    }

    // Load medieval world template
    printf("[%s] Loading medieval world template...\n", TAG);
    if (!world_load_medieval_template(&world, &g_camera)) {
        fprintf(stderr, "[%s] Failed to load world template\n", TAG);
        world_free(&world);
        return -1;
    }
    printf("[%s] World loaded: %d blocks\n", TAG, world.count);

    // Initialize trigonometry lookup tables
    printf("[%s] Initializing trigonometry lookup tables...\n", TAG);
    trig_lut_init();

    // Initialize texture system
    printf("[%s] Initializing texture system...\n", TAG);
    esp_err_t tex_ret = texture_loader_init();
    if (tex_ret == ESP_OK) {
        bool using_sd = texture_loader_using_sdcard();
        printf("[%s] Texture system initialized (%s)\n", TAG,
               using_sd ? "SD card" : "embedded fallback");
        console_log(LOG_LEVEL_INFO, "TEXTURES", "%s textures",
                    using_sd ? "SD card" : "embedded");
    } else {
        printf("[%s] WARNING: Texture system initialization failed (using embedded textures)\n", TAG);
    }

    // Initialize renderer
    static renderer_t renderer = {0};
    if (!renderer_init(&renderer, options->width, options->height, &g_camera, &world)) {
        fprintf(stderr, "[%s] Failed to initialize renderer\n", TAG);
        world_free(&world);
        return -1;
    }

    // Set global renderer reference for debug block placement
    extern renderer_t* g_global_renderer;
    g_global_renderer = &renderer;

    printf("[%s] Renderer initialized: %dx%d\n", TAG, options->width, options->height);

    // Enable wireframe mode if requested
    if (options->wireframe) {
        printf("[%s] Enabling wireframe mode...\n", TAG);
        renderer_toggle_wireframe(1);
    }

    printf("[%s] Starting rendering loop...\n", TAG);

    // Create persistent texture for framebuffer display
    Texture2D fb_texture = {0};
    Image fb_image = {
        .data = NULL,
        .width = options->width,
        .height = options->height,
        .mipmaps = 1,
        .format = PIXELFORMAT_UNCOMPRESSED_R8G8B8A8
    };
    fb_image.data = (unsigned char*)malloc(options->width * options->height * 4);
    fb_texture = LoadTextureFromImage(fb_image);

    // Rendering loop
    int frameCounter = 0;
    float auto_rotate = g_camera.yaw;  // Start from initial yaw
    int max_frames = options->duration_seconds * 20;  // 20 FPS
    bool keys_pressed = false;  // Track if user interacted

    while (frameCounter < max_frames && g_running) {
        // Check if window close button was clicked
        if (WindowShouldClose()) {
            g_running = 0;
            break;
        }

        // Update console animation
        console_update();

        // Handle console toggle key
        if (IsKeyPressed(KEY_F3) || IsKeyPressed(KEY_GRAVE)) {
            console_toggle();
        }

        // Handle diagnostic keys
        if (IsKeyPressed(KEY_F5)) {
            // Toggle debug block mode
            printf("[%s] F5 pressed - toggling debug block mode\n", TAG);
            renderer_toggle_debug_block(-1);
        }

        if (IsKeyPressed(KEY_F6)) {
            // Toggle wireframe mode
            printf("[%s] F6 pressed - toggling wireframe mode\n", TAG);
            renderer_toggle_wireframe(-1);
        }

        // Handle keyboard input
        if (options->interactive) {
            float move_speed = 0.5f;
            float rot_speed = 0.05f;
            bool any_key = false;

            // Movement
            if (IsKeyDown(KEY_W)) {
                g_camera.x += cosf(g_camera.yaw) * move_speed;
                g_camera.z += sinf(g_camera.yaw) * move_speed;
                any_key = true;
            }
            if (IsKeyDown(KEY_S)) {
                g_camera.x -= cosf(g_camera.yaw) * move_speed;
                g_camera.z -= sinf(g_camera.yaw) * move_speed;
                any_key = true;
            }
            if (IsKeyDown(KEY_A)) {
                g_camera.x += cosf(g_camera.yaw - 1.57f) * move_speed;
                g_camera.z += sinf(g_camera.yaw - 1.57f) * move_speed;
                any_key = true;
            }
            if (IsKeyDown(KEY_D)) {
                g_camera.x += cosf(g_camera.yaw + 1.57f) * move_speed;
                g_camera.z += sinf(g_camera.yaw + 1.57f) * move_speed;
                any_key = true;
            }
            if (IsKeyDown(KEY_E)) {  // Up (replaces Space)
                g_camera.y += move_speed;
                any_key = true;
            }
            if (IsKeyDown(KEY_Q)) {  // Down (replaces Shift)
                g_camera.y -= move_speed;
                any_key = true;
            }

            // Rotation
            if (IsKeyDown(KEY_LEFT)) {
                g_camera.yaw -= rot_speed;
                any_key = true;
            }
            if (IsKeyDown(KEY_RIGHT)) {
                g_camera.yaw += rot_speed;
                any_key = true;
            }
            if (IsKeyDown(KEY_UP)) {
                g_camera.pitch += rot_speed;
                any_key = true;
            }
            if (IsKeyDown(KEY_DOWN)) {
                g_camera.pitch -= rot_speed;
                any_key = true;
            }

            if (any_key) {
                keys_pressed = true;
            }
        }

        // Auto-rotate camera only if no user interaction
        if (!keys_pressed && !options->interactive) {
            auto_rotate += 0.05f;  // 0.05 radians per frame
            g_camera.yaw = auto_rotate;
        }

        // Extend max frames if user is interacting
        if (keys_pressed && frameCounter >= max_frames - 1) {
            max_frames += 20;  // Add another second
        }

        // Render frame to internal framebuffer
        renderer_render_frame(&renderer);

        // Get the rendered framebuffer
        const uint16_t* fb = renderer_get_framebuffer(&renderer);
        int32_t fb_width, fb_height;
        renderer_get_dimensions(&renderer, &fb_width, &fb_height);

        // Draw framebuffer to screen
        BeginDrawing();
        ClearBackground(BLACK);

        // Convert RGB565 framebuffer to RGBA and update texture
        if (fb && fb_width > 0 && fb_height > 0) {
            unsigned char* pixels = (unsigned char*)fb_image.data;

            // DEBUG: Check first few pixels on frame 20
            static int debug_frame = 0;
            debug_frame++;
            if (debug_frame == 20) {
                printf("[%s] DEBUG: Frame 20 framebuffer dump:\n", TAG);
                printf("  fb[0] = 0x%04x\n", fb[0]);
                printf("  fb[1] = 0x%04x\n", fb[1]);
                printf("  fb[%d] = 0x%04x (diagonal)\n", fb_width + 1, fb[fb_width + 1]);
                printf("  fb[%d] = 0x%04x (middle)\n", (fb_height/2) * fb_width + fb_width/2,
                       fb[(fb_height/2) * fb_width + fb_width/2]);
            }

            for (int32_t i = 0; i < fb_width * fb_height; i++) {
                uint16_t pixel = fb[i];

                // Extract RGB565 components
                uint8_t r5 = (pixel >> 11) & 0x1F;
                uint8_t g6 = (pixel >> 5) & 0x3F;
                uint8_t b5 = pixel & 0x1F;

                // Convert to 8-bit
                uint8_t r8 = (r5 << 3) | (r5 >> 2);
                uint8_t g8 = (g6 << 2) | (g6 >> 4);
                uint8_t b8 = (b5 << 3) | (b5 >> 2);

                // Write RGBA
                pixels[i * 4 + 0] = r8;
                pixels[i * 4 + 1] = g8;
                pixels[i * 4 + 2] = b8;
                pixels[i * 4 + 3] = 255;
            }

            // Update texture and draw
            UpdateTexture(fb_texture, fb_image.data);
            DrawTexture(fb_texture, 0, 0, WHITE);
        }

        // Draw debug overlay
        if (options->verbose || options->interactive) {
            DrawText(TextFormat("Frame: %d/%d", frameCounter, max_frames), 10, 10, 15, WHITE);
            DrawText(TextFormat("Blocks: %d", world.count), 10, 25, 15, WHITE);
            DrawText(TextFormat("Pos: (%.1f, %.1f, %.1f)", g_camera.x, g_camera.y, g_camera.z), 10, 40, 15, WHITE);
            DrawText(TextFormat("Yaw: %.2f Pitch: %.2f", g_camera.yaw, g_camera.pitch), 10, 55, 15, WHITE);

            if (options->interactive) {
                DrawText("WASD=Move Q/E=Up/Down Arrows=Look", 10, 75, 12, YELLOW);
            }

            // Draw horizon line to see where middle is
            DrawLine(0, GetScreenHeight()/2, GetScreenWidth(), GetScreenHeight()/2, RED);
        }

        // Render console overlay (after everything else)
        // Desktop build passes NULL for framebuffer (uses Raylib drawing)
        console_render(NULL, 0, 0);

        EndDrawing();

        frameCounter++;

        if (frameCounter % 30 == 0) {
            printf("[%s] Frame %d: %dx%d, %d blocks\n", TAG, frameCounter,
                   options->width, options->height, world.count);
        }

        // Frame rate control (20 FPS)
        usleep(50000);  // 50ms
    }

    printf("[%s] Rendering complete: %d frames\n", TAG, frameCounter);

    // Cleanup
    UnloadTexture(fb_texture);
    UnloadImage(fb_image);
    renderer_free(&renderer);
    world_free(&world);

    return 0;
}

/**
 * @brief Save screenshot to file
 * @param filename Output filename
 * @return 0 on success, -1 on error
 */
static int save_screenshot(const char* filename) {
    // Take screenshot from framebuffer
    Image image = LoadImageFromScreen();
    if (image.data == NULL) {
        fprintf(stderr, "[%s] Failed to capture screenshot\n", TAG);
        return -1;
    }

    // Export to PNG
    bool success = ExportImage(image, filename);
    if (!success) {
        fprintf(stderr, "[%s] Failed to save screenshot to %s\n", TAG, filename);
        UnloadImage(image);
        return -1;
    }

    printf("[%s] Screenshot saved: %s (%dx%d, %d KB)\n", TAG,
           filename, image.width, image.height, (int)(image.width * image.height * 4 / 1024));

    UnloadImage(image);
    return 0;
}

/**
 * @brief Main entry point
 */
int main(int argc, char* argv[])
{
    printf("\n");
    printf("╔════════════════════════════════════════════════════════════╗\n");
    printf("║                                                            ║\n");
    printf("║     IotCraft Desktop Simulator                            ║\n");
    printf("║     Desktop version of ESP32-P4 3D client                  ║\n");
    printf("║                                                            ║\n");
    printf("╚════════════════════════════════════════════════════════════╝\n");
    printf("\n");

    // Parse command-line options
    cli_options_t options;
    if (cli_parse_options(argc, argv, &options) < 0) {
        return 1;
    }

    if (options.verbose) {
        cli_print_defaults();
    }

    // Setup signal handlers
    signal(SIGINT, signal_handler);
    signal(SIGTERM, signal_handler);

    // Initialize window
    if (options.headless) {
        printf("[%s] Running in headless mode\n", TAG);
    }

    InitWindow(options.width, options.height, "IotCraft Desktop Simulator");
    SetTargetFPS(20);

    printf("[%s] Raylib initialized: %dx%d\n", TAG, options.width, options.height);

    // Chessboard test only if explicitly requested
    if (options.chessboard) {
        printf("\n[%s] ========================================\n", TAG);
        printf("[%s] PHASE 2: Chessboard Display Test\n", TAG);
        printf("[%s] ========================================\n", TAG);
        display_chessboard_test(5000);  // 5 seconds

        if (!g_running) {
            CloseWindow();
            return 0;
        }
    }

    // Run 3D renderer (Phase 3)
    printf("\n[%s] ========================================\n", TAG);
    printf("[%s] PHASE 3: 3D Renderer Test\n", TAG);
    printf("[%s] ========================================\n", TAG);

    if (run_renderer(&options) < 0) {
        CloseWindow();
        return 1;
    }

    // Save screenshot(s) if requested
    if (options.screenshot_mode && g_running) {
        if (options.screenshot_count == 1) {
            // Single screenshot mode (backward compatible)
            printf("\n[%s] Saving screenshot...\n", TAG);
            if (save_screenshot(options.screenshot_path) < 0) {
                CloseWindow();
                return 1;
            }
        } else {
            // Multi-screenshot mode with camera rotation
            printf("\n[%s] Taking %d screenshots with camera rotation...\n", TAG, options.screenshot_count);

            for (int shot = 0; shot < options.screenshot_count; shot++) {
                // Generate filename with shot number
                char shot_filename[512];
                const char* ext = strstr(options.screenshot_path, ".png");
                if (ext) {
                    size_t base_len = ext - options.screenshot_path;
                    snprintf(shot_filename, sizeof(shot_filename), "%.*s_%d%s",
                             (int)base_len, options.screenshot_path, shot + 1, ".png");
                } else {
                    snprintf(shot_filename, sizeof(shot_filename), "%s_%d.png",
                             options.screenshot_path, shot + 1);
                }

                // Save screenshot
                printf("[%s] Saving screenshot %d/%d: %s\n", TAG, shot + 1, options.screenshot_count, shot_filename);
                if (save_screenshot(shot_filename) < 0) {
                    CloseWindow();
                    return 1;
                }

                // Rotate camera for next shot (if not the last one)
                if (shot < options.screenshot_count - 1 && options.camera_rotate_yaw != 0.0f) {
                    // Rotate camera using camera_rotate function
                    camera_rotate(&g_camera, options.camera_rotate_yaw, 0.0f);

                    // Render just ONE frame to update the scene
                    // Don't use full duration - we just need one frame for the screenshot
                    cli_options_t single_frame_opts = options;
                    single_frame_opts.duration_seconds = 0.05f;  // Just 1 frame at 20 FPS
                    run_renderer(&single_frame_opts);

                    // Small delay
                    WaitTime(options.screenshot_interval);
                }
            }

            printf("[%s] All screenshots saved!\n", TAG);
        }
    }

    // Cleanup
    CloseWindow();
    printf("\n[%s] Exiting normally\n", TAG);

    return 0;
}
