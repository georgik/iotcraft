/**
 * @file game.h
 * @brief Game logic and controller integration
 */

#pragma once

#include "iotcraft_types.h"
#include "input.h"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Game configuration
 */
typedef struct {
    float move_speed;        // Movement speed (blocks per second)
    float rotate_speed;      // Rotation speed (radians per second)
    float mouse_sensitivity; // Mouse sensitivity
} game_config_t;

/**
 * @brief Game state
 */
typedef struct {
    camera_t* camera;
    voxel_world_t* world;
    game_config_t config;
    bool running;
} game_state_t;

/**
 * @brief Initialize game
 * @param game Game state to initialize
 * @param camera Camera to use
 * @param world World to use
 * @return true on success, false on failure
 */
bool game_init(game_state_t* game, camera_t* camera, voxel_world_t* world);

/**
 * @brief Update game logic (call every frame)
 * @param game Game state to update
 * @param delta_time Time since last update (seconds)
 */
void game_update(game_state_t* game, float delta_time);

/**
 * @brief Handle keyboard input for camera movement
 * @param game Game state
 * @param key Key code
 * @param pressed true if pressed, false if released
 */
void game_handle_key(game_state_t* game, iotcraft_key_code_t key, bool pressed);

/**
 * @brief Shutdown game
 * @param game Game state to shutdown
 */
void game_shutdown(game_state_t* game);

#ifdef __cplusplus
}
#endif
