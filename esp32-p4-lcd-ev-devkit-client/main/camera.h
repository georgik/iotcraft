/**
 * @file camera.h
 * @brief Camera management for first-person view
 */

#pragma once

#include "iotcraft_types.h"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Initialize camera with default position
 * @param camera Camera to initialize
 */
void camera_init(camera_t* camera);

/**
 * @brief Move camera forward/backward
 * @param camera Camera to move
 * @param delta Distance to move (positive = forward, negative = backward)
 */
void camera_move_forward(camera_t* camera, float delta);

/**
 * @brief Move camera left/right (strafe)
 * @param camera Camera to move
 * @param delta Distance to move (positive = right, negative = left)
 */
void camera_move_strafe(camera_t* camera, float delta);

/**
 * @brief Rotate camera
 * @param camera Camera to rotate
 * @param yaw_delta Yaw rotation (radians)
 * @param pitch_delta Pitch rotation (radians)
 */
void camera_rotate(camera_t* camera, float yaw_delta, float pitch_delta);

#ifdef __cplusplus
}
#endif
