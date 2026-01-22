/**
 * @file trig_lut.h
 * @brief Fast trigonometric lookup tables
 *
 * OPTIMIZATION: Critical lookup functions placed in IRAM
 * These are called ~32,000 times per frame (2 angles × 16,000 projections)
 */

#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Lookup table resolution (degrees)
#define TRIG_LUT_RESOLUTION 360
#define TRIG_LUT_SCALE (2 * 3.14159265359f / 360.0f)  // Degrees to radians

// IRAM attribute for ESP32-P4
#ifdef __ESP32_P4__
#define IRAM_FN __attribute__((section(".iram.text")))
#else
#define IRAM_FN
#endif

/**
 * @brief Initialize trigonometric lookup tables
 * Call once during startup
 */
void trig_lut_init(void);

/**
 * @brief Fast sine lookup (degrees)
 * @param degrees Angle in degrees (can be negative, wraps automatically)
 * @return Sine value (-1.0 to 1.0)
 *
 * CRITICAL: Called ~16,000 times per frame - Placed in IRAM
 */
float IRAM_FN sin_fast(float degrees);

/**
 * @brief Fast cosine lookup (degrees)
 * @param degrees Angle in degrees (can be negative, wraps automatically)
 * @return Cosine value (-1.0 to 1.0)
 *
 * CRITICAL: Called ~16,000 times per frame - Placed in IRAM
 */
float IRAM_FN cos_fast(float degrees);

/**
 * @brief Fast sine lookup (radians)
 * @param radians Angle in radians
 * @return Sine value (-1.0 to 1.0)
 */
float sinf_fast(float radians);

/**
 * @brief Fast cosine lookup (radians)
 * @param radians Angle in radians
 * @return Cosine value (-1.0 to 1.0)
 */
float cosf_fast(float radians);

#ifdef __cplusplus
}
#endif
