/**
 * @file camera.c
 * @brief Camera management implementation
 */

#include "camera.h"
#include <math.h>
#include <esp_log.h>

static const char* TAG = "Camera";

void camera_init(camera_t* camera) {
    if (!camera) {
        return;
    }

    // Start at origin, looking forward
    camera->x = 0.0f;
    camera->y = 1.5f;  // Eye level (1.5 blocks high)
    camera->z = 0.0f;
    camera->yaw = 0.0f;
    camera->pitch = 0.0f;
    camera->fov = 1.047f;  // ~60 degrees in radians

    ESP_LOGI(TAG, "Camera initialized at position (%.2f, %.2f, %.2f)",
             camera->x, camera->y, camera->z);
}

void camera_move_forward(camera_t* camera, float delta) {
    if (!camera) {
        return;
    }

    // Calculate forward direction from yaw angle
    float dx = cosf(camera->yaw) * delta;
    float dz = sinf(camera->yaw) * delta;

    camera->x += dx;
    camera->z += dz;
}

void camera_move_strafe(camera_t* camera, float delta) {
    if (!camera) {
        return;
    }

    // Calculate right direction (perpendicular to forward)
    float dx = sinf(camera->yaw) * delta;
    float dz = -cosf(camera->yaw) * delta;

    camera->x += dx;
    camera->z += dz;
}

void camera_rotate(camera_t* camera, float yaw_delta, float pitch_delta) {
    if (!camera) {
        return;
    }

    camera->yaw += yaw_delta;

    // Normalize yaw to 0-2PI range
    while (camera->yaw < 0.0f) {
        camera->yaw += 6.28318530718f;  // 2*PI
    }
    while (camera->yaw >= 6.28318530718f) {
        camera->yaw -= 6.28318530718f;
    }

    // Limit pitch to avoid gimbal lock
    camera->pitch += pitch_delta;
    if (camera->pitch > 1.047f) {   // +60 degrees
        camera->pitch = 1.047f;
    }
    if (camera->pitch < -1.047f) {  // -60 degrees
        camera->pitch = -1.047f;
    }
}
