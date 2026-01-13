/**
 * @file game.c
 * @brief Game logic implementation
 */

#include "game.h"
#include "camera.h"
#include "input.h"
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
    bool up;
    bool down;
} key_actions = {0};

bool game_init(game_state_t* game, camera_t* camera, voxel_world_t* world) {
    if (!game || !camera || !world) {
        ESP_LOGE(TAG, "Null parameters in game_init");
        return false;
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

    // WASD movement
    switch (key) {
        case IOTCRAFT_KEY_W:
            key_actions.forward = pressed;
            break;
        case IOTCRAFT_KEY_S:
            key_actions.backward = pressed;
            break;
        case IOTCRAFT_KEY_A:
            key_actions.left = pressed;
            break;
        case IOTCRAFT_KEY_D:
            key_actions.right = pressed;
            break;

        // Arrow keys for rotation
        case IOTCRAFT_KEY_LEFT:
            key_actions.rotate_left = pressed;
            break;
        case IOTCRAFT_KEY_RIGHT:
            key_actions.rotate_right = pressed;
            break;

        // Space/Shift for vertical movement
        case IOTCRAFT_KEY_SPACE:
            key_actions.up = pressed;
            break;
        case IOTCRAFT_KEY_LEFT_SHIFT:
        case IOTCRAFT_KEY_RIGHT_SHIFT:
            key_actions.down = pressed;
            break;

        // Quit on ESC
        case IOTCRAFT_KEY_ESCAPE:
            if (pressed) {
                game->running = false;
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
