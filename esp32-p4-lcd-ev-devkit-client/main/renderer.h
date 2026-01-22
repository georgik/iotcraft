/**
 * @file renderer.h
 * @brief Raycasting renderer for voxel world
 */

#pragma once

#include "iotcraft_types.h"
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Visible voxel structure for multi-core rendering
 */
typedef struct {
    int32_t x, y, z;
    block_type_t block;
    float depth;
} visible_voxel_t;

/**
 * @brief Thread-safe voxel buffer for multi-core rendering
 */
typedef struct {
    visible_voxel_t voxels[2048];  // Per-core voxel buffer (reduced from 4096)
    int count;                     // Number of voxels collected
} voxel_buffer_t;

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

/**
 * @brief Check if wireframe mode is currently enabled
 * @return true if wireframe mode is enabled, false otherwise
 */
bool renderer_is_wireframe_enabled(void);

/**
 * @brief Toggle debug block mode (F5 - displays redstone block for visual debugging)
 * @param new_mode If true, enable debug block; if false, disable. If -1, toggle current state
 * @return Current debug block mode state
 */
bool renderer_toggle_debug_block(int new_mode);

/**
 * @brief Collect voxels in world space range (for multi-core rendering)
 * @param renderer Renderer context
 * @param buffer Output voxel buffer
 * @param x_min, x_max X-axis world space bounds (exclusive per-core split)
 */
void renderer_collect_voxels_parallel(renderer_t* renderer, voxel_buffer_t* buffer,
                                      int32_t x_min, int32_t x_max);

/**
 * @brief Sort voxels in buffer by depth (for multi-core rendering)
 * @param buffer Voxel buffer to sort
 * @param cam_x, cam_y, cam_z Camera position for distance calculation
 */
void renderer_sort_voxel_buffer(voxel_buffer_t* buffer,
                                 float cam_x, float cam_y, float cam_z);

/**
 * @brief Render voxels from buffer
 * @param renderer Renderer context
 * @param buffer Voxel buffer to render
 */
void renderer_render_voxel_buffer(renderer_t* renderer, const voxel_buffer_t* buffer);

#ifdef __cplusplus
}
#endif
