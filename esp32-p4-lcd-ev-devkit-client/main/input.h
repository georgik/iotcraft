/**
 * @file input.h
 * @brief Input handling for USB keyboard
 */

#pragma once

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Key codes (simplified subset of HID usage)
 * Named with IOTCRAFT_ prefix to avoid conflicts with Raylib
 */
typedef enum {
    IOTCRAFT_KEY_UNKNOWN = 0,
    IOTCRAFT_KEY_A = 4,
    IOTCRAFT_KEY_B = 5,
    IOTCRAFT_KEY_C = 6,
    IOTCRAFT_KEY_D = 7,
    IOTCRAFT_KEY_E = 8,
    IOTCRAFT_KEY_F = 9,
    IOTCRAFT_KEY_G = 10,
    IOTCRAFT_KEY_H = 11,
    IOTCRAFT_KEY_I = 12,
    IOTCRAFT_KEY_J = 13,
    IOTCRAFT_KEY_K = 14,
    IOTCRAFT_KEY_L = 15,
    IOTCRAFT_KEY_M = 16,
    IOTCRAFT_KEY_N = 17,
    IOTCRAFT_KEY_O = 18,
    IOTCRAFT_KEY_P = 19,
    IOTCRAFT_KEY_Q = 20,
    IOTCRAFT_KEY_R = 21,
    IOTCRAFT_KEY_S = 22,
    IOTCRAFT_KEY_T = 23,
    IOTCRAFT_KEY_U = 24,
    IOTCRAFT_KEY_V = 25,
    IOTCRAFT_KEY_W = 26,
    IOTCRAFT_KEY_X = 27,
    IOTCRAFT_KEY_Y = 28,
    IOTCRAFT_KEY_Z = 29,
    IOTCRAFT_KEY_1 = 30,
    IOTCRAFT_KEY_2 = 31,
    IOTCRAFT_KEY_3 = 32,
    IOTCRAFT_KEY_4 = 33,
    IOTCRAFT_KEY_5 = 34,
    IOTCRAFT_KEY_6 = 35,
    IOTCRAFT_KEY_7 = 36,
    IOTCRAFT_KEY_8 = 37,
    IOTCRAFT_KEY_9 = 38,
    IOTCRAFT_KEY_0 = 39,
    IOTCRAFT_KEY_ENTER = 40,
    IOTCRAFT_KEY_ESCAPE = 41,
    IOTCRAFT_KEY_BACKSPACE = 42,
    IOTCRAFT_KEY_TAB = 43,
    IOTCRAFT_KEY_SPACE = 44,
    IOTCRAFT_KEY_MINUS = 45,
    IOTCRAFT_KEY_EQUALS = 46,
    IOTCRAFT_KEY_LEFT_BRACKET = 47,
    IOTCRAFT_KEY_RIGHT_BRACKET = 48,
    IOTCRAFT_KEY_BACKSLASH = 49,
    IOTCRAFT_KEY_SEMICOLON = 51,
    IOTCRAFT_KEY_QUOTE = 52,
    IOTCRAFT_KEY_GRAVE = 53,
    IOTCRAFT_KEY_COMMA = 54,
    IOTCRAFT_KEY_PERIOD = 55,
    IOTCRAFT_KEY_SLASH = 56,
    IOTCRAFT_KEY_CAPS_LOCK = 57,
    IOTCRAFT_KEY_F1 = 58,
    IOTCRAFT_KEY_F2 = 59,
    IOTCRAFT_KEY_F3 = 60,
    IOTCRAFT_KEY_F4 = 61,
    IOTCRAFT_KEY_F5 = 62,
    IOTCRAFT_KEY_F6 = 63,
    IOTCRAFT_KEY_F7 = 64,
    IOTCRAFT_KEY_F8 = 65,
    IOTCRAFT_KEY_F9 = 66,
    IOTCRAFT_KEY_F10 = 67,
    IOTCRAFT_KEY_F11 = 68,
    IOTCRAFT_KEY_F12 = 69,
    IOTCRAFT_KEY_PRINT_SCREEN = 70,
    IOTCRAFT_KEY_SCROLL_LOCK = 71,
    IOTCRAFT_KEY_PAUSE = 72,
    IOTCRAFT_KEY_INSERT = 73,
    IOTCRAFT_KEY_HOME = 74,
    IOTCRAFT_KEY_PAGE_UP = 75,
    IOTCRAFT_KEY_DELETE = 76,
    IOTCRAFT_KEY_END = 77,
    IOTCRAFT_KEY_PAGE_DOWN = 78,
    IOTCRAFT_KEY_RIGHT = 79,
    IOTCRAFT_KEY_LEFT = 80,
    IOTCRAFT_KEY_DOWN = 81,
    IOTCRAFT_KEY_UP = 82,
    IOTCRAFT_KEY_NUM_LOCK = 83,
    IOTCRAFT_KEY_LEFT_SHIFT = 224,
    IOTCRAFT_KEY_LEFT_CTRL = 225,
    IOTCRAFT_KEY_LEFT_ALT = 226,
    IOTCRAFT_KEY_RIGHT_SHIFT = 227,
} iotcraft_key_code_t;

/**
 * @brief Key state
 */
typedef struct {
    bool pressed;
    uint32_t timestamp;  // When the key was last pressed
} key_state_t;

/**
 * @brief Input state structure
 */
typedef struct {
    key_state_t keys[256];  // State of all possible keys
    uint32_t last_update;
} input_state_t;

/**
 * @brief Initialize USB HID input system
 * @return true on success, false on failure
 */
bool input_init(void);

/**
 * @brief Get current input state
 * @return Pointer to input state (read-only)
 */
const input_state_t* input_get_state(void);

/**
 * @brief Check if a key is currently pressed
 * @param key Key code to check
 * @return true if key is pressed, false otherwise
 */
bool input_is_key_pressed(iotcraft_key_code_t key);

/**
 * @brief Update input state (called from USB HID callback)
 * @param key Key code
 * @param pressed true if key was pressed, false if released
 */
void input_update_key(iotcraft_key_code_t key, bool pressed);

/**
 * @brief Poll for input updates (non-blocking)
 */
void input_poll(void);

/**
 * @brief Process HID keyboard report (internal use by USB driver)
 * @param report HID report data
 * @param length Report length
 */
void input_process_hid_report(const uint8_t *report, uint16_t length);

#ifdef __cplusplus
}
#endif
