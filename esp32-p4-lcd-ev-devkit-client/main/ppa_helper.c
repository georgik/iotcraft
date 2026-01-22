/**
 * @file ppa_helper.c
 * @brief PPA (Pixel-Processing Accelerator) helper implementation
 */

#include "ppa_helper.h"

// Desktop simulator - define esp_err_t if not available (must be before functions)
#if !defined(__ESP32_P4__) && !defined(IDF_TARGET_ESP32P4)
#ifndef ESP_OK
#define ESP_OK 0
typedef int esp_err_t;
#endif
#endif

#include <stdlib.h>
#include <string.h>
#include <inttypes.h>

#if defined(__ESP32_P4__) || defined(IDF_TARGET_ESP32P4)
#include "esp_log.h"
#include "esp_heap_caps.h"
#else
#define heap_caps_malloc(size, caps) malloc(size)
#define heap_caps_free(ptr) free(ptr)
#define MALLOC_CAP_DMA 0

// Desktop stub functions
static inline esp_err_t ppa_register_client(void* config, ppa_client_handle_t* handle) {
    (void)config;
    (void)handle;
    return ESP_OK;
}

static inline esp_err_t ppa_unregister_client(ppa_client_handle_t handle) {
    (void)handle;
    return ESP_OK;
}

static inline esp_err_t ppa_do_scale_rotate_mirror(ppa_client_handle_t handle, void* config) {
    (void)handle;
    (void)config;
    return ESP_OK;
}

typedef struct {
    int oper_type;
    int max_pending_trans_num;
    int data_burst_length;
} ppa_client_config_t;
#endif

static const char* TAG = "PPAScaler";

bool ppa_scaler_init(ppa_scaler_t* scaler, int32_t final_width, int32_t final_height, int32_t scale_div) {
    if (!scaler || scale_div < 1) {
        ESP_LOGE(TAG, "Invalid parameters");
        return false;
    }

    memset(scaler, 0, sizeof(ppa_scaler_t));

    scaler->final_width = final_width;
    scaler->final_height = final_height;
    scaler->small_width = final_width / scale_div;
    scaler->small_height = final_height / scale_div;

    ESP_LOGI(TAG, "Initializing PPA scaler: %" PRId32 "x%" PRId32 " -> %" PRId32 "x%" PRId32 " (scale factor: %" PRId32 ")",
             scaler->small_width, scaler->small_height,
             scaler->final_width, scaler->final_height, scale_div);

    // Check if we should use hardware scaling
    // Only enable on ESP32-P4 (not on desktop simulator)
#ifdef __ESP32_P4__
    scaler->use_hardware_scaling = true;
    ESP_LOGI(TAG, "Hardware PPA scaling enabled");
#else
    scaler->use_hardware_scaling = false;
    ESP_LOGI(TAG, "Hardware PPA scaling disabled (desktop mode - using stubs)");
#endif

    // Allocate final buffer (always needed)
    size_t final_size = final_width * final_height * sizeof(uint16_t);
    scaler->final_buffer = (uint16_t*)heap_caps_malloc(final_size, MALLOC_CAP_DMA);
    if (!scaler->final_buffer) {
        ESP_LOGE(TAG, "Failed to allocate final buffer (%zu bytes)", final_size);
        return false;
    }

    // Allocate small buffer (for low-resolution rendering)
    if (scaler->use_hardware_scaling) {
        size_t small_size = scaler->small_width * scaler->small_height * sizeof(uint16_t);
        scaler->small_buffer = (uint16_t*)heap_caps_malloc(small_size, MALLOC_CAP_DMA);
        if (!scaler->small_buffer) {
            ESP_LOGE(TAG, "Failed to allocate small buffer (%zu bytes)", small_size);
            heap_caps_free(scaler->final_buffer);
            return false;
        }

        // Initialize PPA client (hardware only)
        ppa_client_config_t ppa_config = {
            .oper_type = PPA_OPERATION_SRM,
            .max_pending_trans_num = 1,
            .data_burst_length = PPA_DATA_BURST_LENGTH_128
        };

        esp_err_t ret = ppa_register_client(&ppa_config, &scaler->ppa_client);
        if (ret != ESP_OK) {
            ESP_LOGE(TAG, "Failed to register PPA client: %s", esp_err_to_name(ret));
            heap_caps_free(scaler->small_buffer);
            heap_caps_free(scaler->final_buffer);
            return false;
        }

        ESP_LOGI(TAG, "PPA client registered successfully");
    } else {
        // Desktop mode: use final buffer directly (no scaling)
        scaler->small_buffer = scaler->final_buffer;
        // In desktop mode, small dimensions equal final dimensions
        scaler->small_width = final_width;
        scaler->small_height = final_height;
    }

    scaler->initialized = true;
    return true;
}

uint16_t* ppa_scaler_get_render_buffer(ppa_scaler_t* scaler) {
    if (!scaler || !scaler->initialized) {
        ESP_LOGE(TAG, "Scaler not initialized");
        return NULL;
    }

    return scaler->small_buffer;
}

bool ppa_scaler_scale(ppa_scaler_t* scaler) {
    if (!scaler || !scaler->initialized) {
        ESP_LOGE(TAG, "Scaler not initialized");
        return false;
    }

    if (!scaler->use_hardware_scaling) {
        // Desktop mode: buffers are the same, no scaling needed
        return true;
    }

#ifdef __ESP32_P4__
    // Use PPA hardware to scale small_buffer to final_buffer
    ppa_srm_oper_config_t srm_config = {
        .in = {
            .buffer = scaler->small_buffer,
            .pic_w = scaler->small_width,
            .pic_h = scaler->small_height,
            .block_w = scaler->small_width,
            .block_h = scaler->small_height,
            .block_offset_x = 0,
            .block_offset_y = 0,
            .srm_cm = PPA_SRM_COLOR_MODE_RGB565
        },
        .out = {
            .buffer = scaler->final_buffer,
            .buffer_size = scaler->final_width * scaler->final_height * sizeof(uint16_t),
            .pic_w = scaler->final_width,
            .pic_h = scaler->final_height,
            .block_offset_x = 0,
            .block_offset_y = 0,
            .srm_cm = PPA_SRM_COLOR_MODE_RGB565
        },
        .scale_x = (float)scaler->final_width / (float)scaler->small_width,
        .scale_y = (float)scaler->final_height / (float)scaler->small_height,
        .rotation_angle = PPA_SRM_ROTATION_ANGLE_0,
        .mirror_x = false,
        .mirror_y = false,
        .rgb_swap = false,
        .byte_swap = false,
        .alpha_update_mode = PPA_ALPHA_NO_CHANGE,
        .mode = PPA_TRANS_MODE_BLOCKING,  // Wait for completion
        .user_data = NULL
    };

    esp_err_t ret = ppa_do_scale_rotate_mirror(scaler->ppa_client, &srm_config);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "PPA scaling failed: %s", esp_err_to_name(ret));
        return false;
    }
#endif

    return true;
}

uint16_t* ppa_scaler_get_final_buffer(ppa_scaler_t* scaler) {
    if (!scaler || !scaler->initialized) {
        ESP_LOGE(TAG, "Scaler not initialized");
        return NULL;
    }

    return scaler->final_buffer;
}

void ppa_scaler_free(ppa_scaler_t* scaler) {
    if (!scaler || !scaler->initialized) {
        return;
    }

    if (scaler->use_hardware_scaling) {
#ifdef __ESP32_P4__
        // Unregister PPA client
        if (scaler->ppa_client) {
            ppa_unregister_client(scaler->ppa_client);
        }
#endif

        // Free small buffer (if different from final)
        if (scaler->small_buffer && scaler->small_buffer != scaler->final_buffer) {
            heap_caps_free(scaler->small_buffer);
        }
    }

    // Free final buffer
    if (scaler->final_buffer) {
        heap_caps_free(scaler->final_buffer);
    }

    memset(scaler, 0, sizeof(ppa_scaler_t));
}
