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

// Shared components (from ESP32 version)
#include "iotcraft_types.h"
#include "camera.h"
#include "world.h"
#include "renderer.h"
#include "game.h"
#include "input.h"
#include "world_template.h"
#include "trig_lut.h"

#define TAG "IotCraftDesktop"

// Flag to control main loop
static volatile sig_atomic_t g_running = 1;

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

    // Initialize camera
    camera_t camera = {0};
    camera_init(&camera);

    // Use CLI camera position if provided, otherwise use defaults
    if (!isnan(options->cam_x)) {
        camera.x = options->cam_x;
        camera.y = options->cam_y;
        camera.z = options->cam_z;
    } else {
        camera.x = -15.0f;
        camera.y = 2.0f;
        camera.z = 0.0f;
    }

    if (!isnan(options->cam_yaw)) {
        camera.yaw = options->cam_yaw;
        camera.pitch = options->cam_pitch;
    } else {
        camera.yaw = -0.78f;  // -45 degrees
        camera.pitch = -0.17f;  // -10 degrees
    }

    // Initialize world
    voxel_world_t world = {0};
    if (!world_init(&world)) {
        fprintf(stderr, "[%s] Failed to initialize world\n", TAG);
        return -1;
    }

    // Load medieval world template
    printf("[%s] Loading medieval world template...\n", TAG);
    if (!world_load_medieval_template(&world, &camera)) {
        fprintf(stderr, "[%s] Failed to load world template\n", TAG);
        world_free(&world);
        return -1;
    }
    printf("[%s] World loaded: %d blocks\n", TAG, world.count);

    // Initialize trigonometry lookup tables
    printf("[%s] Initializing trigonometry lookup tables...\n", TAG);
    trig_lut_init();

    // Initialize renderer
    renderer_t renderer = {0};
    if (!renderer_init(&renderer, options->width, options->height, &camera, &world)) {
        fprintf(stderr, "[%s] Failed to initialize renderer\n", TAG);
        world_free(&world);
        return -1;
    }

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
    float auto_rotate = camera.yaw;  // Start from initial yaw
    int max_frames = options->duration_seconds * 20;  // 20 FPS
    bool keys_pressed = false;  // Track if user interacted

    while (frameCounter < max_frames && g_running) {
        // Handle keyboard input
        if (options->interactive || WindowShouldClose()) {
            float move_speed = 0.5f;
            float rot_speed = 0.05f;
            bool any_key = false;

            // Movement
            if (IsKeyDown(KEY_W)) {
                camera.x += cosf(camera.yaw) * move_speed;
                camera.z += sinf(camera.yaw) * move_speed;
                any_key = true;
            }
            if (IsKeyDown(KEY_S)) {
                camera.x -= cosf(camera.yaw) * move_speed;
                camera.z -= sinf(camera.yaw) * move_speed;
                any_key = true;
            }
            if (IsKeyDown(KEY_A)) {
                camera.x += cosf(camera.yaw - 1.57f) * move_speed;
                camera.z += sinf(camera.yaw - 1.57f) * move_speed;
                any_key = true;
            }
            if (IsKeyDown(KEY_D)) {
                camera.x += cosf(camera.yaw + 1.57f) * move_speed;
                camera.z += sinf(camera.yaw + 1.57f) * move_speed;
                any_key = true;
            }
            if (IsKeyDown(KEY_SPACE)) {
                camera.y += move_speed;
                any_key = true;
            }
            if (IsKeyDown(KEY_LEFT_SHIFT) || IsKeyDown(KEY_RIGHT_SHIFT)) {
                camera.y -= move_speed;
                any_key = true;
            }

            // Rotation
            if (IsKeyDown(KEY_LEFT)) {
                camera.yaw -= rot_speed;
                any_key = true;
            }
            if (IsKeyDown(KEY_RIGHT)) {
                camera.yaw += rot_speed;
                any_key = true;
            }
            if (IsKeyDown(KEY_UP)) {
                camera.pitch += rot_speed;
                any_key = true;
            }
            if (IsKeyDown(KEY_DOWN)) {
                camera.pitch -= rot_speed;
                any_key = true;
            }

            if (any_key) {
                keys_pressed = true;
            }
        }

        // Auto-rotate camera only if no user interaction
        if (!keys_pressed && !options->interactive) {
            auto_rotate += 0.05f;  // 0.05 radians per frame
            camera.yaw = auto_rotate;
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
            DrawText(TextFormat("Pos: (%.1f, %.1f, %.1f)", camera.x, camera.y, camera.z), 10, 40, 15, WHITE);
            DrawText(TextFormat("Yaw: %.2f Pitch: %.2f", camera.yaw, camera.pitch), 10, 55, 15, WHITE);

            if (options->interactive) {
                DrawText("WASD=Move Space/Shift=Up/Down Arrows=Look", 10, 75, 12, YELLOW);
            }

            // Draw horizon line to see where middle is
            DrawLine(0, GetScreenHeight()/2, GetScreenWidth(), GetScreenHeight()/2, RED);
        }

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

    // Save screenshot if requested
    if (options.screenshot_mode && g_running) {
        printf("\n[%s] Saving screenshot...\n", TAG);
        if (save_screenshot(options.screenshot_path) < 0) {
            CloseWindow();
            return 1;
        }
    }

    // Cleanup
    CloseWindow();
    printf("\n[%s] Exiting normally\n", TAG);

    return 0;
}
