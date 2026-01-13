/**
 * @file iotcraft_types.h
 * @brief Core type definitions for IotCraft ESP32-P4 client
 */

#pragma once

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Block types matching desktop client
 */
typedef enum {
    BLOCK_AIR = 0,
    BLOCK_GRASS,
    BLOCK_DIRT,
    BLOCK_STONE,
    BLOCK_QUARTZ,
    BLOCK_GLASS,
    BLOCK_TERRACOTTA,
    BLOCK_WATER,
    BLOCK_COUNT
} block_type_t;

/**
 * @brief Voxel block in the world
 */
typedef struct {
    int32_t x, y, z;
    block_type_t type;
} voxel_t;

/**
 * @brief Camera structure for first-person view
 */
typedef struct {
    float x, y, z;           // Position
    float yaw, pitch;        // Rotation angles (radians)
    float fov;               // Field of view (radians)
} camera_t;

/**
 * @brief Voxel world storage
 */
typedef struct {
    voxel_t* voxels;
    int32_t count;
    int32_t capacity;
} voxel_world_t;

/**
 * @brief Rendering context
 */
typedef struct {
    uint16_t* framebuffer;   // RGB565 framebuffer
    int32_t width;            // Rendering resolution width
    int32_t height;           // Rendering resolution height
    camera_t* camera;
    voxel_world_t* world;
} renderer_t;

#ifdef __cplusplus
}
#endif
