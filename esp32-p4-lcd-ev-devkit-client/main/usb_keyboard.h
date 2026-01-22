/**
 * @file usb_keyboard.h
 * @brief USB HID keyboard driver
 */

#pragma once

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Initialize USB HID keyboard driver
 * @return true on success, false on failure
 */
bool usb_keyboard_init(void);

/**
 * @brief Check if USB keyboard is connected
 * @return true if keyboard is connected, false otherwise
 */
bool usb_keyboard_is_connected(void);

/**
 * @brief Deinitialize USB HID keyboard driver
 */
void usb_keyboard_deinit(void);

#ifdef __cplusplus
}
#endif
