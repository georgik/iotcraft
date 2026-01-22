/**
 * @file device_manager.h
 * @brief Device manager for IoT devices (lamps, etc.) in the voxel world
 */

#pragma once

#include <stdint.h>
#include <stdbool.h>
#include "iotcraft_types.h"

#ifdef __cplusplus
extern "C" {
#endif

// Maximum number of devices to track
#define MAX_DEVICES 32

/**
 * @brief Device types
 */
typedef enum {
    DEVICE_TYPE_UNKNOWN = 0,
    DEVICE_TYPE_LAMP,
    DEVICE_TYPE_DOOR,
    DEVICE_TYPE_COUNT
} device_type_t;

/**
 * @brief Device state
 */
typedef enum {
    DEVICE_STATE_OFFLINE = 0,
    DEVICE_STATE_ONLINE,
    DEVICE_STATE_BLINKING,
} device_state_t;

/**
 * @brief IoT device representation
 */
typedef struct {
    char id[64];              // Device ID (e.g., "c6-lamp-001")
    device_type_t type;       // Device type
    device_state_t state;     // Current state
    float x, y, z;            // Position in the world
    bool is_visible;          // Whether device is currently in the world
    bool is_blinking;         // Whether device is currently blinking
} iot_device_t;

/**
 * @brief Initialize device manager
 */
void device_manager_init(void);

/**
 * @brief Find or create a device by ID
 * @param device_id Device ID string
 * @return Pointer to device, or NULL if max devices reached
 */
iot_device_t* device_manager_get_or_create(const char* device_id);

/**
 * @brief Find a device by ID
 * @param device_id Device ID string
 * @return Pointer to device, or NULL if not found
 */
iot_device_t* device_manager_find(const char* device_id);

/**
 * @brief Update device position
 * @param device Device to update
 * @param x New X position
 * @param y New Y position
 * @param z New Z position
 */
void device_manager_update_position(iot_device_t* device, float x, float y, float z);

/**
 * @brief Set device blink state
 * @param device Device to update
 * @param blinking true to enable blinking, false to disable
 */
void device_manager_set_blinking(iot_device_t* device, bool blinking);

/**
 * @brief Update all devices (call every frame)
 * @param world World to spawn/despawn devices in
 * @param delta_time Time since last frame (ms)
 */
void device_manager_update(voxel_world_t* world, float delta_time);

/**
 * @brief Get all devices
 * @param count Output parameter for number of devices
 * @return Array of devices
 */
const iot_device_t* device_manager_get_all(int* count);

/**
 * @brief Send blink command to all lamp devices
 * @param blinking true to turn on, false to turn off
 * @return Number of devices commanded
 */
int device_manager_blink_all(bool blinking);

/**
 * @brief Check if a block at world position is a blinking lamp (should be rendered brighter)
 * @param x, y, z World coordinates
 * @return true if block is a blinking lamp in "on" phase, false otherwise
 */
bool device_manager_is_blinking_lamp(int32_t x, int32_t y, int32_t z);

#ifdef __cplusplus
}
#endif
