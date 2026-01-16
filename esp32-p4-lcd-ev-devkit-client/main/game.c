/**
 * @file game.c
 * @brief Game logic implementation
 */

#include "game.h"
#include "camera.h"
#include "input.h"
#include "brightness.h"
#include "console.h"
#include <esp_log.h>
#include <math.h>

static const char* TAG = "Game";

// Key state tracking
static struct {
    bool forward;
    bool backward;
    bool left;
    bool right;
    bool rotate_left;
    bool rotate_right;
    bool rotate_up;
    bool rotate_down;
    bool up;
    bool down;
} key_actions = {0};

bool game_init(game_state_t* game, camera_t* camera, voxel_world_t* world) {
    if (!game || !camera || !world) {
        ESP_LOGE(TAG, "Null parameters in game_init");
        return false;
    }

    // Initialize brightness control
    if (!brightness_init()) {
        ESP_LOGW(TAG, "Failed to initialize brightness control");
    }

    game->camera = camera;
    game->world = world;
    game->config.move_speed = 5.0f;       // 5 blocks per second
    game->config.rotate_speed = 2.0f;     // 2 radians per second
    game->config.mouse_sensitivity = 0.1f;
    game->running = true;

    ESP_LOGI(TAG, "Game initialized");
    return true;
}

void game_handle_key(game_state_t* game, iotcraft_key_code_t key, bool pressed) {
    if (!game) {
        return;
    }

    // WASD movement (corrected based on actual behavior)
    switch (key) {
        case IOTCRAFT_KEY_W:
            key_actions.forward = pressed;  // Should move forward
            break;
        case IOTCRAFT_KEY_S:
            key_actions.backward = pressed;  // Should move backward
            break;
        case IOTCRAFT_KEY_A:
            key_actions.left = pressed;  // Should strafe left
            break;
        case IOTCRAFT_KEY_D:
            key_actions.right = pressed;  // Should strafe right
            break;

        // Arrow keys for rotation (swapped: LEFT now rotates right, RIGHT rotates left)
        case IOTCRAFT_KEY_LEFT:
            key_actions.rotate_right = pressed;
            break;
        case IOTCRAFT_KEY_RIGHT:
            key_actions.rotate_left = pressed;
            break;
        case IOTCRAFT_KEY_UP:
            key_actions.rotate_up = pressed;  // Tilt camera up
            break;
        case IOTCRAFT_KEY_DOWN:
            key_actions.rotate_down = pressed;  // Tilt camera down
            break;

        // Q/E for vertical movement (altitude control)
        case IOTCRAFT_KEY_Q:
            key_actions.up = pressed;
            break;
        case IOTCRAFT_KEY_E:
            key_actions.down = pressed;
            break;

        // ESC - on desktop it quits, on ESP32-P4 it's handled by console overlay
        // We don't set running=false here because ESP32-P4 should never exit
        case IOTCRAFT_KEY_ESCAPE:
            // ESC is handled in hello.c for console control
            // On ESP32-P4, we ignore it here (no quit)
            break;

        // N/M for brightness control
        case IOTCRAFT_KEY_N:
            if (pressed) {
                brightness_decrease(10);  // Decrease by 10%
            }
            break;
        case IOTCRAFT_KEY_M:
            if (pressed) {
                brightness_increase(10);  // Increase by 10%
            }
            break;

        // F3 for console toggle
        case IOTCRAFT_KEY_F3:
            if (pressed) {
                console_toggle();
            }
            break;

        default:
            break;
    }
}

void game_update(game_state_t* game, float delta_time) {
    if (!game || !game->camera) {
        return;
    }

    float move_delta = game->config.move_speed * delta_time;
    float rotate_delta = game->config.rotate_speed * delta_time;

    // Forward/backward movement
    if (key_actions.forward) {
        camera_move_forward(game->camera, move_delta);
    }
    if (key_actions.backward) {
        camera_move_forward(game->camera, -move_delta);
    }

    // Strafe left/right
    if (key_actions.left) {
        camera_move_strafe(game->camera, -move_delta);
    }
    if (key_actions.right) {
        camera_move_strafe(game->camera, move_delta);
    }

    // Rotation
    if (key_actions.rotate_left) {
        camera_rotate(game->camera, -rotate_delta, 0.0f);
    }
    if (key_actions.rotate_right) {
        camera_rotate(game->camera, rotate_delta, 0.0f);
    }

    // Pitch (look up/down)
    if (key_actions.rotate_up) {
        camera_rotate(game->camera, 0.0f, rotate_delta);
    }
    if (key_actions.rotate_down) {
        camera_rotate(game->camera, 0.0f, -rotate_delta);
    }

    // Vertical movement
    if (key_actions.up) {
        game->camera->y += move_delta;
    }
    if (key_actions.down) {
        game->camera->y -= move_delta;
    }
}

void game_shutdown(game_state_t* game) {
    if (!game) {
        return;
    }

    game->running = false;
    game->camera = NULL;
    game->world = NULL;

    ESP_LOGI(TAG, "Game shutdown");
}
