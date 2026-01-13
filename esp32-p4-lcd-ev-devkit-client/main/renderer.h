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
 * @brief Render a range of columns (for multi-core rendering)
 * @param renderer Renderer context
 * @param start_col First column to render (inclusive)
 * @param end_col Last column to render (exclusive)
 */
void renderer_render_columns(renderer_t* renderer, int32_t start_col, int32_t end_col);

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

/**
 * @brief Toggle wireframe rendering mode
 * @param new_mode If true, enable wireframe; if false, disable. If -1, toggle current state
 * @return Current wireframe mode state
 */
bool renderer_toggle_wireframe(int new_mode);

#ifdef __cplusplus
}
#endif
