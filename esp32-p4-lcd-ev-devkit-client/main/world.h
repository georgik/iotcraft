/**
 * @file world.h
 * @brief Voxel world management
 */

#pragma once

#include "iotcraft_types.h"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Initialize a voxel world
 * @param world World structure to initialize
 * @return true on success, false on failure
 */
bool world_init(voxel_world_t* world);

/**
 * @brief Free world resources
 * @param world World to free
 */
void world_free(voxel_world_t* world);

/**
 * @brief Get block type at position
 * @param world World to query
 * @param x X coordinate
 * @param y Y coordinate
 * @param z Z coordinate
 * @return Block type at position (BLOCK_AIR if no block)
 */
block_type_t world_get_block(const voxel_world_t* world, int32_t x, int32_t y, int32_t z);

/**
 * @brief Set block at position
 * @param world World to modify
 * @param x X coordinate
 * @param y Y coordinate
 * @param z Z coordinate
 * @param type Block type to set
 * @return true on success, false on failure
 */
bool world_set_block(voxel_world_t* world, int32_t x, int32_t y, int32_t z, block_type_t type);

/**
 * @brief Generate test terrain (flat grass plane)
 * @param world World to populate
 */
void world_generate_test_terrain(voxel_world_t* world);

/**
 * @brief Get block that camera is looking at
 * @param world World to query
 * @param camera Camera position and direction
 * @param target_x Output X coordinate of targeted block
 * @param target_y Output Y coordinate of targeted block
 * @param target_z Output Z coordinate of targeted block
 * @return true if a block is targeted, false otherwise
 */
bool world_get_target_block(const voxel_world_t* world, const camera_t* camera,
                             int32_t* target_x, int32_t* target_y, int32_t* target_z);

/**
 * @brief Get empty position adjacent to targeted block (for placement)
 * @param world World to query
 * @param camera Camera position and direction
 * @param place_x Output X coordinate for placement
 * @param place_y Output Y coordinate for placement
 * @param place_z Output Z coordinate for placement
 * @return true if a valid position is found, false otherwise
 */
bool world_get_place_position(const voxel_world_t* world, const camera_t* camera,
                               int32_t* place_x, int32_t* place_y, int32_t* place_z);

#ifdef __cplusplus
}
#endif
