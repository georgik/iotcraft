/**
 * @file brightness.h
 * @brief Display brightness control using PWM
 */

#pragma once

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Initialize brightness control
 *
 * Sets up PWM on GPIO26 for LCD backlight control
 *
 * @return true on success, false on failure
 */
bool brightness_init(void);

/**
 * @brief Set brightness level
 *
 * @param percent Brightness percentage (0-100)
 * @return true on success, false on failure
 */
bool brightness_set(uint8_t percent);

/**
 * @brief Increase brightness
 *
 * Increases brightness by step percent (clamped to 100)
 *
 * @param step Step size in percent (default: 10)
 * @return New brightness level (0-100)
 */
uint8_t brightness_increase(uint8_t step);

/**
 * @brief Decrease brightness
 *
 * Decreases brightness by step percent (clamped to 0)
 *
 * @param step Step size in percent (default: 10)
 * @return New brightness level (0-100)
 */
uint8_t brightness_decrease(uint8_t step);

/**
 * @brief Get current brightness level
 *
 * @return Current brightness percentage (0-100)
 */
uint8_t brightness_get(void);

#ifdef __cplusplus
}
#endif
