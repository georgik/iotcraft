/**
 * @file input.c
 * @brief Input handling implementation
 */

#include "input.h"
#include <string.h>
#include <esp_log.h>
#include <freertos/FreeRTOS.h>
#include <freertos/task.h>

static const char* TAG = "Input";

// Global input state
static input_state_t g_input_state = {0};

// Track currently pressed keys for release detection
static bool g_currently_pressed[256] = {false};

bool input_init(void) {
    memset(&g_input_state, 0, sizeof(input_state_t));
    memset(g_currently_pressed, 0, sizeof(g_currently_pressed));
    g_input_state.last_update = 0;

    ESP_LOGI(TAG, "Input system initialized");
    return true;
}

const input_state_t* input_get_state(void) {
    return &g_input_state;
}

bool input_is_key_pressed(iotcraft_key_code_t key) {
    if (key >= 256) {
        return false;
    }
    return g_input_state.keys[key].pressed;
}

void input_update_key(iotcraft_key_code_t key, bool pressed) {
    if (key >= 256) {
        return;
    }

    // Only update if state changed
    if (g_input_state.keys[key].pressed != pressed) {
        g_input_state.keys[key].pressed = pressed;
        g_input_state.keys[key].timestamp = xTaskGetTickCount();
        g_input_state.last_update = xTaskGetTickCount();

        // Log all key presses (including F-keys) for debugging
        if (pressed) {
            ESP_LOGI(TAG, "Key pressed: %d (0x%02x)", key, key);
        } else {
            ESP_LOGV(TAG, "Key released: %d", key);
        }
    }
}

void input_poll(void) {
    // USB HID callbacks will update the state automatically
    // This function can be used for auto-release of keys

    // Auto-release all keys (simple approach for HID boot protocol)
    // A more sophisticated approach would track actual key state
    for (int i = 0; i < 256; i++) {
        if (g_input_state.keys[i].pressed) {
            // Mark for potential release (will be re-pressed if still in HID report)
            g_currently_pressed[i] = false;
        }
    }

    vTaskDelay(pdMS_TO_TICKS(10));  // 100Hz polling
}

/**
 * @brief Process HID keyboard report (for use by USB driver)
 *
 * This function handles the key state tracking properly
 */
void input_process_hid_report(const uint8_t *report, uint16_t length)
{
    if (!report || length < 8) {
        return;
    }

    // First, clear all non-modifier keys
    for (int i = 0; i < 256; i++) {
        if (i != IOTCRAFT_KEY_LEFT_CTRL &&
            i != IOTCRAFT_KEY_LEFT_SHIFT &&
            i != IOTCRAFT_KEY_LEFT_ALT &&
            i != IOTCRAFT_KEY_RIGHT_SHIFT) {
            g_currently_pressed[i] = false;
        }
    }

    // Process modifier keys (byte 0)
    // Bit 0: Left Ctrl, Bit 1: Left Shift, Bit 2: Left Alt, Bit 3: Left GUI
    // Bit 4: Right Ctrl, Bit 5: Right Shift, Bit 6: Right Alt, Bit 7: Right GUI
    uint8_t modifiers = report[0];
    g_currently_pressed[IOTCRAFT_KEY_LEFT_CTRL] = (modifiers & 0x01) != 0;
    g_currently_pressed[IOTCRAFT_KEY_LEFT_SHIFT] = (modifiers & 0x02) != 0;
    g_currently_pressed[IOTCRAFT_KEY_LEFT_ALT] = (modifiers & 0x04) != 0;
    // g_currently_pressed[IOTCRAFT_KEY_LEFT_GUI] = (modifiers & 0x08) != 0;  // Not defined
    g_currently_pressed[IOTCRAFT_KEY_RIGHT_CTRL] = (modifiers & 0x10) != 0;
    g_currently_pressed[IOTCRAFT_KEY_RIGHT_SHIFT] = (modifiers & 0x20) != 0;
    // g_currently_pressed[IOTCRAFT_KEY_RIGHT_ALT] = (modifiers & 0x40) != 0;  // Not defined
    // g_currently_pressed[IOTCRAFT_KEY_RIGHT_GUI] = (modifiers & 0x80) != 0;  // Not defined

    // Mark regular keys in this report as pressed (bytes 2-7)
    for (int i = 2; i < 8 && i < length; i++) {
        if (report[i] != 0) {
            g_currently_pressed[report[i]] = true;
        }
    }

    // Update input state based on currently pressed keys
    for (int i = 0; i < 256; i++) {
        input_update_key(i, g_currently_pressed[i]);
    }
}
