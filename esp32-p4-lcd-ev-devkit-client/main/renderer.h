/**
 * @file renderer.h
 * @brief Raycasting renderer for voxel world
 */

#pragma once

#include "iotcraft_types.h"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Initialize renderer
 * @param renderer Renderer to initialize
 * @param width Rendering width
 * @param height Rendering height
 * @param camera Camera to use
 * @param world World to render
 * @return true on success, false on failure
 */
bool renderer_init(renderer_t* renderer, int32_t width, int32_t height,
                   camera_t* camera, voxel_world_t* world);

/**
 * @brief Free renderer resources
 * @param renderer Renderer to free
 */
void renderer_free(renderer_t* renderer);

/**
 * @brief Render a frame (raycasting)
 * @param renderer Renderer context
 */
void renderer_render_frame(renderer_t* renderer);

/**
 * @brief Clear framebuffer to solid color
 * @param renderer Renderer context
 * @param color RGB565 color
 */
void renderer_clear(renderer_t* renderer, uint16_t color);

/**
 * @brief Get framebuffer pointer for display
 * @param renderer Renderer context
 * @return Pointer to RGB565 framebuffer
 */
const uint16_t* renderer_get_framebuffer(const renderer_t* renderer);

/**
 * @brief Get framebuffer dimensions
 * @param renderer Renderer context
 * @param width Output width
 * @param height Output height
 */
void renderer_get_dimensions(const renderer_t* renderer, int32_t* width, int32_t* height);

#ifdef __cplusplus
}
#endif
