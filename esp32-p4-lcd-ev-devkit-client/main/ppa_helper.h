/**
 * @file ppa_helper.h
 * @brief PPA (Pixel-Processing Accelerator) helper for hardware upscaling
 *
 * This module uses the ESP32-P4's PPA hardware to upscale the rendered image,
 * providing 3-4x speedup by rendering at lower resolution and scaling up.
 */

#ifndef PPA_HELPER_H
#define PPA_HELPER_H

#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <inttypes.h>

// Only include ESP-IDF headers on ESP32 platform
#if defined(__ESP32_P4__) || defined(IDF_TARGET_ESP32P4)
#include "esp_log.h"
#include "esp_err.h"
#include "driver/ppa.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#else
// Desktop simulator stubs (PPA not needed on desktop)
#ifndef ESP_LOGI
#define ESP_LOGI(tag, fmt, ...) printf("[%s] " fmt "\n", tag, ##__VA_ARGS__)
#define ESP_LOGE(tag, fmt, ...) printf("[%s] ERROR: " fmt "\n", tag, ##__VA_ARGS__)
#define ESP_LOGW(tag, fmt, ...) printf("[%s] WARNING: " fmt "\n", tag, ##__VA_ARGS__)
#endif
typedef int ppa_client_handle_t;
typedef enum { PPA_OPERATION_SRM } ppa_operation_t;
typedef enum { PPA_SRM_COLOR_MODE_RGB565 } ppa_srm_color_mode_t;
typedef enum { PPA_SRM_ROTATION_ANGLE_0 } ppa_srm_rotation_angle_t;
typedef enum { PPA_TRANS_MODE_BLOCKING } ppa_trans_mode_t;
typedef enum { PPA_DATA_BURST_LENGTH_128 } ppa_data_burst_length_t;
typedef enum { PPA_ALPHA_NO_CHANGE } ppa_alpha_update_mode_t;
// Desktop stub: define esp_err_t and esp_err_to_name only for desktop
#ifndef ESP_OK
#define ESP_OK 0
typedef int esp_err_t;
static inline const char* esp_err_to_name(esp_err_t code) { (void)code; return "unknown"; }
#endif
#endif

#ifdef __cplusplus
extern "C" {
#endif

// PPA scaling context
typedef struct {
    ppa_client_handle_t ppa_client;      // PPA client handle
    uint16_t* small_buffer;              // Low-resolution render buffer
    uint16_t* final_buffer;              // Final scaled buffer
    int32_t small_width;                 // Small buffer width
    int32_t small_height;                // Small buffer height
    int32_t final_width;                 // Final buffer width
    int32_t final_height;                // Final buffer height
    bool initialized;                    // Initialization flag
    bool use_hardware_scaling;           // Enable/disable hardware scaling
} ppa_scaler_t;

/**
 * @brief Initialize PPA scaler
 * @param scaler Scaler context to initialize
 * @param final_width Final output width (e.g., 320)
 * @param final_height Final output height (e.g., 240)
 * @param scale_div Scaling divisor (2 = half resolution, 4 = quarter resolution)
 * @return true if successful
 */
bool ppa_scaler_init(ppa_scaler_t* scaler, int32_t final_width, int32_t final_height, int32_t scale_div);

/**
 * @brief Get small render buffer for low-resolution rendering
 * @param scaler Scaler context
 * @return Pointer to small buffer (or final buffer if scaling disabled)
 */
uint16_t* ppa_scaler_get_render_buffer(ppa_scaler_t* scaler);

/**
 * @brief Scale the small buffer to final resolution using PPA hardware
 * @param scaler Scaler context
 * @return true if successful
 */
bool ppa_scaler_scale(ppa_scaler_t* scaler);

/**
 * @brief Get final buffer after scaling
 * @param scaler Scaler context
 * @return Pointer to final buffer
 */
uint16_t* ppa_scaler_get_final_buffer(ppa_scaler_t* scaler);

/**
 * @brief Free PPA scaler resources
 * @param scaler Scaler context to free
 */
void ppa_scaler_free(ppa_scaler_t* scaler);

#ifdef __cplusplus
}
#endif

#endif // PPA_HELPER_H
