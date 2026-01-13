/**
 * @file trig_lut.c
 * @brief Fast trigonometric lookup tables implementation
 */

#include "trig_lut.h"
#include <math.h>
#include "esp_log.h"

static const char* TAG = "TrigLUT";

// Lookup tables (360 degrees = 2π radians)
static float sin_lut[TRIG_LUT_RESOLUTION];
static float cos_lut[TRIG_LUT_RESOLUTION];
static bool lut_initialized = false;

void trig_lut_init(void) {
    if (lut_initialized) {
        return;
    }

    ESP_LOGI(TAG, "Generating trigonometric lookup tables...");

    // Pre-compute sin/cos for every degree
    for (int i = 0; i < TRIG_LUT_RESOLUTION; i++) {
        float radians = (float)i * TRIG_LUT_SCALE;
        sin_lut[i] = sinf(radians);
        cos_lut[i] = cosf(radians);
    }

    lut_initialized = true;
    ESP_LOGI(TAG, "Trigonometric LUT ready: %d entries, %.1f KB",
             TRIG_LUT_RESOLUTION,
             (float)(sizeof(sin_lut) + sizeof(cos_lut)) / 1024.0f);
}

float sin_fast(float degrees) {
    if (!lut_initialized) {
        return sinf(degrees * 3.14159265359f / 180.0f);
    }

    // Normalize to 0-359 range
    int index = (int)degrees;
    index = index % TRIG_LUT_RESOLUTION;
    if (index < 0) {
        index += TRIG_LUT_RESOLUTION;
    }

    return sin_lut[index];
}

float cos_fast(float degrees) {
    if (!lut_initialized) {
        return cosf(degrees * 3.14159265359f / 180.0f);
    }

    // Normalize to 0-359 range
    int index = (int)degrees;
    index = index % TRIG_LUT_RESOLUTION;
    if (index < 0) {
        index += TRIG_LUT_RESOLUTION;
    }

    return cos_lut[index];
}

float sinf_fast(float radians) {
    // Convert radians to degrees and use lookup
    float degrees = radians * 180.0f / 3.14159265359f;
    return sin_fast(degrees);
}

float cosf_fast(float radians) {
    // Convert radians to degrees and use lookup
    float degrees = radians * 180.0f / 3.14159265359f;
    return cos_fast(degrees);
}
