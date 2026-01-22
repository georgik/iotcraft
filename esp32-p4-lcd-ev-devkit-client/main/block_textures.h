/**
 * @file block_textures.h
 * @brief Procedural texture patterns for block types
 */

#pragma once

#include "iotcraft_types.h"
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define TEXTURE_SIZE 8  // 8x8 texture tiles

// Get texture color for a block type at given UV coordinates
uint16_t block_texture_get_pixel(block_type_t type, int u, int v);

// Get base color for a block type (fallback)
uint16_t block_type_get_color(block_type_t type);

#ifdef __cplusplus
}
#endif
