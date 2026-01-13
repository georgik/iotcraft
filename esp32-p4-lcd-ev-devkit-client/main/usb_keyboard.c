/**
 * @file usb_keyboard.c
 * @brief USB HID keyboard driver implementation
 *
 * NOTE: Full USB keyboard support requires integration with ESP USB HID host
 * event system and device enumeration. This is a placeholder for future implementation.
 *
 * For now, the client uses auto-rotation demo mode.
 */

#include "usb_keyboard.h"
#include "esp_log.h"
#include "usb/hid_host.h"

static const char* TAG = "USBKeyboard";

static bool g_keyboard_connected = false;

bool usb_keyboard_init(void)
{
    ESP_LOGI(TAG, "Initializing USB HID keyboard...");
    ESP_LOGI(TAG, "Note: Full USB keyboard support pending HID event integration");

    // Install HID host driver
    esp_err_t ret = hid_host_install(NULL);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to install HID host: %d", ret);
        return false;
    }

    ESP_LOGI(TAG, "USB HID host installed (using auto-rotation demo mode)");
    return true;
}

bool usb_keyboard_is_connected(void)
{
    // Always return false for now - using auto-rotation demo
    return false;
}

void usb_keyboard_deinit(void)
{
    hid_host_uninstall();
    g_keyboard_connected = false;
    ESP_LOGI(TAG, "USB HID keyboard deinitialized");
}
