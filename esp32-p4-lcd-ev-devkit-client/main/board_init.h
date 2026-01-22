/**
 * @file board_init.h
 * @brief Board initialization interface
 */

#pragma once

#include "esp_err.h"
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Initialize display hardware and raylib port layer
 *
 * This function handles board-specific initialization including:
 * - Creating esp_lcd panel/io handles
 * - Initializing backlight
 * - Configuring raylib port layer
 * - Registering display with port
 *
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t board_init_display(void);

/**
 * @brief Push RGB565 framebuffer to display
 * @param framebuffer Pointer to RGB565 pixel data
 * @param width Framebuffer width in pixels
 * @param height Framebuffer height in pixels
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t board_display_push_frame(const uint16_t* framebuffer, int width, int height);

/**
 * @brief Push framebuffer to display with hardware scaling
 * @param framebuffer Source framebuffer (RGB565)
 * @param src_width Source width
 * @param src_height Source height
 * @param dst_width Destination width (display size)
 * @param dst_height Destination height (display size)
 * @return ESP_OK on success
 */
esp_err_t board_display_push_frame_scaled(const uint16_t* framebuffer,
                                          int src_width, int src_height,
                                          int dst_width, int dst_height);

#ifdef __cplusplus
}
#endif
