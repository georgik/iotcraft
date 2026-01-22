/**
 * @file sdcard_init.h
 * @brief SD card initialization for ESP32-P4
 */

#pragma once

#include <stdbool.h>
#include <stdint.h>

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
 * @brief Initialize SD card and mount filesystem
 * @return ESP_OK on success (or if SD card not available - this is not an error)
 *
 * Note: This function always succeeds, even if SD card is not present.
 * Use sdcard_is_available() to check if SD card was mounted.
 */
esp_err_t sdcard_init(void);

/**
 * @brief Check if SD card is available
 * @return true if SD card is mounted and ready, false otherwise
 */
bool sdcard_is_available(void);

/**
 * @brief Get SD card mount point
 * @return Mount point path (e.g., "/sdcard") or NULL if not mounted
 */
const char* sdcard_get_mount_point(void);

/**
 * @brief Unmount SD card filesystem
 * @return ESP_OK on success
 */
esp_err_t sdcard_deinit(void);

#ifdef __cplusplus
}
#endif
