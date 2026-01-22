/**
 * @file texture_loader.h
 * @brief Texture loading from SD card or embedded fallback
 */

#pragma once

#include <stdbool.h>
#include <stdint.h>
#include "iotcraft_types.h"

#ifdef __cplusplus
extern "C" {
#endif

// ESP-IDF compatibility for desktop simulator
#if !defined(__DESKTOP_BUILD__) && !defined(__ESP32_P4__)
    // Real ESP-IDF platform
    #include "esp_err.h"
#elif defined(__ESP32_P4__)
    // ESP32-P4 platform
    #include "esp_err.h"
#else
    // Desktop simulator - include ppa_helper.h for esp_err_t definition
    #include "ppa_helper.h"
#endif

/**
 * @brief Initialize texture system
 * @return ESP_OK on success
 *
 * Attempts to load textures from SD card.
 * Falls back to embedded textures if SD card not available.
 */
esp_err_t texture_loader_init(void);

/**
 * @brief Check if using SD card textures
 * @return true if textures loaded from SD card, false if embedded
 */
bool texture_loader_using_sdcard(void);

/**
 * @brief Get texture data for a block type
 * @param type Block type
 * @return Pointer to texture data (256 pixels = 512 bytes for 16x16)
 *
 * Returns pointer to RGB565 texture data.
 * Always valid - falls back to embedded textures if needed.
 */
const uint16_t* texture_get(block_type_t type);

/**
 * @brief Get texture size in pixels (width = height)
 * @return Texture size (e.g., 16 for 16x16 textures)
 */
int texture_get_size(void);

/**
 * @brief Load embedded textures (fallback when SD card not available)
 */
void texture_loader_use_embedded(void);

#ifdef __cplusplus
}
#endif
