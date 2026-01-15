/**
 * @file fixed_point.h
 * @brief Fixed-point arithmetic for 3D rendering optimization
 *
 * Uses 16.16 format: 16 bits for integer part, 16 bits for fractional part
 * Range: [-32768.99998, 32767.99998]
 * Precision: 1/65536 ≈ 0.000015
 *
 * This eliminates slow floating-point operations on RISC-V 32-bit IMACF,
 * which has no hardware FPU and must emulate floats in software.
 *
 * CRITICAL FUNCTIONS PLACED IN IRAM FOR MAXIMUM PERFORMANCE
 */

#ifndef FIXED_POINT_H
#define FIXED_POINT_H

#include <stdint.h>
#include <math.h>
#include "esp_log.h"

// Fixed-point type: 16.16 format (signed 32-bit)
typedef int32_t fixed_t;

// Fixed-point constants
#define FIXED_SHIFT 16
#define FIXED_ONE (1 << FIXED_SHIFT)  // 1.0 in fixed-point
#define FIXED_HALF (FIXED_ONE >> 1)   // 0.5 in fixed-point

// Helper macros
#define FIXED_FROM_INT(x) ((fixed_t)((x) << FIXED_SHIFT))
#define FIXED_TO_INT(x) ((x) >> FIXED_SHIFT)
#define FIXED_FROM_FLOAT(x) ((fixed_t)((x) * FIXED_ONE))
#define FIXED_TO_FLOAT(x) ((float)(x) / (float)FIXED_ONE)

// OPTIMIZATION: Place critical fixed-point math in IRAM
// These functions are called 50,000+ times per frame
#ifdef __ESP32_P4__
#define IRAM_FN __attribute__((section(".iram.text")))
#else
#define IRAM_FN
#endif

// Fixed-point multiplication: (a * b) >> 16
// Called ~50,000 times per frame - MUST BE IN IRAM
static inline fixed_t IRAM_FN fixed_mul(fixed_t a, fixed_t b) {
    return (fixed_t)(((int64_t)a * (int64_t)b) >> FIXED_SHIFT);
}

// Fixed-point division: (a << 16) / b
// Called ~5,000 times per frame
static inline fixed_t IRAM_FN fixed_div(fixed_t a, fixed_t b) {
    if (b == 0) {
        return 0;
    }
    return (fixed_t)(((int64_t)a << FIXED_SHIFT) / b);
}

// Fixed-point square root
// Uses Newton-Raphson method
static inline fixed_t fixed_sqrt(fixed_t x) {
    if (x <= 0) return 0;

    // Initial guess: convert to float, sqrt, convert back
    float f = FIXED_TO_FLOAT(x);
    float sqrt_f = sqrtf(f);
    return FIXED_FROM_FLOAT(sqrt_f);
}

// Fixed-point absolute value
static inline fixed_t fixed_abs(fixed_t x) {
    return (x < 0) ? -x : x;
}

// Fixed-point minimum
static inline fixed_t fixed_min(fixed_t a, fixed_t b) {
    return (a < b) ? a : b;
}

// Fixed-point maximum
static inline fixed_t fixed_max(fixed_t a, fixed_t b) {
    return (a > b) ? a : b;
}

// Fixed-point clamp
static inline fixed_t IRAM_FN fixed_clamp(fixed_t x, fixed_t min_val, fixed_t max_val) {
    if (x < min_val) return min_val;
    if (x > max_val) return max_val;
    return x;
}

// Fixed-point comparison with tolerance
static inline bool IRAM_FN fixed_eq(fixed_t a, fixed_t b, fixed_t epsilon) {
    return fixed_abs(a - b) <= epsilon;
}

// Fixed-point sine/cosine using LUT
// These must match the trig_lut.c implementation
extern float IRAM_FN cosf_fast(float angle);
extern float IRAM_FN sinf_fast(float angle);

// CRITICAL: Fixed-point trig functions called ~32,000 times per frame
// Placed in IRAM for maximum performance
static inline fixed_t IRAM_FN fixed_cos(fixed_t angle) {
    float f = FIXED_TO_FLOAT(angle);
    return FIXED_FROM_FLOAT(cosf_fast(f));
}

static inline fixed_t IRAM_FN fixed_sin(fixed_t angle) {
    float f = FIXED_TO_FLOAT(angle);
    return FIXED_FROM_FLOAT(sinf_fast(f));
}

// Fixed-point floor (already integer - just shift)
static inline fixed_t fixed_floor(fixed_t x) {
    return x & 0xFFFF0000;  // Clear fractional bits
}

// Debug helper
static inline void fixed_print(const char* label, fixed_t x) {
    ESP_LOGI("FixedPoint", "%s = %.4f (0x%08x)", label, FIXED_TO_FLOAT(x), (uint32_t)x);
}

#endif // FIXED_POINT_H
