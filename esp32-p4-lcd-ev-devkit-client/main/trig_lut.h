/**
 * @file trig_lut.h
 * @brief Fast trigonometric lookup tables
 */

#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Lookup table resolution (degrees)
#define TRIG_LUT_RESOLUTION 360
#define TRIG_LUT_SCALE (2 * 3.14159265359f / 360.0f)  // Degrees to radians

/**
 * @brief Initialize trigonometric lookup tables
 * Call once during startup
 */
void trig_lut_init(void);

/**
 * @brief Fast sine lookup (degrees)
 * @param degrees Angle in degrees (can be negative, wraps automatically)
 * @return Sine value (-1.0 to 1.0)
 */
float sin_fast(float degrees);

/**
 * @brief Fast cosine lookup (degrees)
 * @param degrees Angle in degrees (can be negative, wraps automatically)
 * @return Cosine value (-1.0 to 1.0)
 */
float cos_fast(float degrees);

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
