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

#ifdef __cplusplus
}
#endif
