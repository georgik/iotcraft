/**
 * @file world_template.h
 * @brief World template parser for loading pre-built worlds
 */

#pragma once

#include "iotcraft_types.h"
#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Parse and load a world template
 * @param template_data World template script (medieval.txt format)
 * @param world World to populate
 * @param camera Camera to position (optional, can be NULL)
 * @return true on success, false on failure
 */
bool world_load_template(const char* template_data, voxel_world_t* world, camera_t* camera);

/**
 * @brief Load embedded medieval world template
 * @param world World to populate
 * @param camera Camera to position (optional, can be NULL)
 * @return true on success, false on failure
 */
bool world_load_medieval_template(voxel_world_t* world, camera_t* camera);

#ifdef __cplusplus
}
#endif
