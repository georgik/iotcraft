/**
 * @file usb_keyboard.c
 * @brief USB HID keyboard driver implementation
 *
 * Based on OpenTyrian's ESP32 HID keyboard implementation
 */

#include "usb_keyboard.h"
#include "input.h"
#include "esp_log.h"
#include "esp_err.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/event_groups.h"
#include "freertos/queue.h"
#include "usb/usb_host.h"
#include "usb/hid_host.h"
#include "usb/hid_usage_keyboard.h"
#include "bsp/esp32_p4_function_ev_board.h"  // For BSP USB host functions
#include <string.h>

static const char* TAG = "USBKeyboard";

// State tracking
static bool g_keyboard_connected = false;
static bool g_hid_host_installed = false;  // Track if HID host is installed
static TaskHandle_t g_hid_event_task_handle = NULL;
static QueueHandle_t g_app_event_queue = NULL;

// Event types
typedef enum {
    APP_EVENT_HID_HOST = 0
} app_event_type_t;

// Event structure
typedef struct {
    app_event_type_t type;
    struct {
        hid_host_device_handle_t handle;
        hid_host_driver_event_t event;
        void *arg;
    } hid_host_device;
} app_event_t;

// Forward declarations
static void hid_host_device_callback(hid_host_device_handle_t hid_device_handle,
                                     const hid_host_driver_event_t event,
                                     void *arg);
static void hid_host_interface_callback(hid_host_device_handle_t hid_device_handle,
                                        const hid_host_interface_event_t event,
                                        void *arg);

/**
 * @brief Check if a key is in the array
 */
static inline bool key_found(const uint8_t *keys, uint8_t key, unsigned int length)
{
    for (unsigned int i = 0; i < length; i++) {
        if (keys[i] == key) {
            return true;
        }
    }
    return false;
}

/**
 * @brief Process HID keyboard input report
 */
static void hid_keyboard_report_callback(const uint8_t *const data, int length)
{
    if (length < 8) {
        ESP_LOGW(TAG, "Short HID report: %d bytes (expected 8)", length);
        return;
    }

    // HID boot report format: modifier (1 byte) + reserved (1 byte) + 6 key codes
    static uint8_t prev_keys[6] = {0};

    // Log first report only for confirmation
    static bool first_report_logged = false;
    if (!first_report_logged) {
        ESP_LOGI(TAG, "Receiving keyboard input: mod=%02x keys=[%02x %02x %02x %02x %02x %02x]",
                 data[0], data[2], data[3], data[4], data[5], data[6], data[7]);
        first_report_logged = true;
    }

    // Process each key slot
    for (int i = 0; i < 6; i++) {
        uint8_t key_code = data[2 + i];

        // Key released (was in previous report but not in current)
        if (prev_keys[i] > 0 && !key_found(data + 2, prev_keys[i], 6)) {
            input_update_key(prev_keys[i], false);
            // ESP_LOGD(TAG, "Key released: %d (0x%02x)", prev_keys[i], prev_keys[i]);
        }

        // Key pressed (is in current report but was not in previous)
        if (key_code > 0 && !key_found(prev_keys, key_code, 6)) {
            input_update_key(key_code, true);
            // ESP_LOGD(TAG, "Key pressed: %d (0x%02x)", key_code, key_code);
        }
    }

    // Copy current keys to previous
    memcpy(prev_keys, data + 2, 6);
}

/**
 * @brief HID interface callback (called when data is received)
 */
static void hid_host_interface_callback(hid_host_device_handle_t hid_device_handle,
                                        const hid_host_interface_event_t event,
                                        void *arg)
{
    uint8_t data[64] = {0};
    size_t data_length = 0;
    hid_host_dev_params_t dev_params;

    esp_err_t ret = hid_host_device_get_params(hid_device_handle, &dev_params);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to get device params: %d", ret);
        return;
    }

    switch (event) {
    case HID_HOST_INTERFACE_EVENT_INPUT_REPORT:
        ret = hid_host_device_get_raw_input_report_data(hid_device_handle,
                                                          data,
                                                          sizeof(data),
                                                          &data_length);
        if (ret == ESP_OK) {
            if (dev_params.proto == HID_PROTOCOL_KEYBOARD) {
                hid_keyboard_report_callback(data, data_length);
            }
        }
        break;

    case HID_HOST_INTERFACE_EVENT_DISCONNECTED:
        ESP_LOGI(TAG, "HID Keyboard disconnected");
        g_keyboard_connected = false;
        hid_host_device_close(hid_device_handle);
        break;

    case HID_HOST_INTERFACE_EVENT_TRANSFER_ERROR:
        ESP_LOGW(TAG, "HID Keyboard transfer error");
        break;

    default:
        break;
    }
}

/**
 * @brief HID device event callback (connection events)
 */
static void hid_host_device_callback(hid_host_device_handle_t hid_device_handle,
                                     const hid_host_driver_event_t event,
                                     void *arg)
{
    // Send event to queue
    app_event_t evt = {
        .type = APP_EVENT_HID_HOST,
        .hid_host_device.handle = hid_device_handle,
        .hid_host_device.event = event,
        .hid_host_device.arg = arg
    };

    if (g_app_event_queue) {
        xQueueSend(g_app_event_queue, &evt, 0);
    }
}

/**
 * @brief Process HID device event
 */
static void process_hid_device_event(hid_host_device_handle_t hid_device_handle,
                                     const hid_host_driver_event_t event,
                                     void *arg)
{
    hid_host_dev_params_t dev_params;
    esp_err_t ret = hid_host_device_get_params(hid_device_handle, &dev_params);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to get device params: %d", ret);
        return;
    }

    ESP_LOGI(TAG, "HID device event: %d, protocol: %d, subclass: %d",
             event, dev_params.proto, dev_params.sub_class);

    switch (event) {
    case HID_HOST_DRIVER_EVENT_CONNECTED:
        ESP_LOGI(TAG, "HID Device connected: protocol %d, subclass %d",
                 dev_params.proto, dev_params.sub_class);

        if (dev_params.proto == HID_PROTOCOL_KEYBOARD) {
            ESP_LOGI(TAG, "Opening keyboard device...");
            const hid_host_device_config_t dev_config = {
                .callback = hid_host_interface_callback,
                .callback_arg = NULL
            };

            ret = hid_host_device_open(hid_device_handle, &dev_config);
            if (ret == ESP_OK) {
                ESP_LOGI(TAG, "Keyboard device opened, starting...");

                // NOTE: Skip hid_class_request_set_protocol() and hid_class_request_set_idle()
                // These optional commands can cause 5-second timeouts if keys are pressed during init.
                // The keyboard works perfectly without them - most modern keyboards default to boot protocol.

                ret = hid_host_device_start(hid_device_handle);
                if (ret == ESP_OK) {
                    g_keyboard_connected = true;
                    ESP_LOGI(TAG, "✓ USB Keyboard connected and ready!");
                } else {
                    ESP_LOGE(TAG, "Failed to start keyboard device: %d", ret);
                }
            } else {
                ESP_LOGE(TAG, "Failed to open keyboard device: %d", ret);
            }
        } else {
            ESP_LOGI(TAG, "Ignoring non-keyboard HID device (protocol %d)", dev_params.proto);
        }
        break;

    default:
        ESP_LOGD(TAG, "Unhandled HID event: %d", event);
        break;
    }
}

/**
 * @brief HID event processing task
 *
 * This task has two responsibilities:
 * 1. Call hid_host_handle_events() to process HID driver events
 * 2. Process events from the queue (device connect/disconnect/input reports)
 */
static void hid_event_task(void *arg)
{
    ESP_LOGI(TAG, "HID event task started, waiting for HID host installation...");

    int event_count = 0;
    int handle_count = 0;

    esp_err_t ret = ESP_OK;  // Initialize to avoid scope issues

    while (true) {
        // Only process HID events after the driver is installed
        if (g_hid_host_installed) {
            // Process HID host events - block until events are available
            // This is the KEY function that triggers all HID callbacks
            ret = hid_host_handle_events(portMAX_DELAY);
            if (ret == ESP_OK) {
                handle_count++;
                // Log first successful call
                if (handle_count == 1) {
                    ESP_LOGI(TAG, "✓ HID events are being processed!");
                }
                // Log every 5000 cycles (reduced from 100)
                if (handle_count % 5000 == 0) {
                    ESP_LOGI(TAG, "Processed %d HID event cycles", handle_count);
                }
            } else {
                ESP_LOGE(TAG, "hid_host_handle_events error: %d", ret);
                break;  // Exit on error
            }
        } else {
            // HID host not installed yet, wait a bit
            vTaskDelay(pdMS_TO_TICKS(10));
        }

        // Process events from our queue (non-blocking)
        app_event_t evt;
        while (xQueueReceive(g_app_event_queue, &evt, 0) == pdTRUE) {
            event_count++;
            if (event_count <= 2) {  // Log first 2 events only
                ESP_LOGI(TAG, "Received queued HID event #%d (type: %d)", event_count, evt.type);
            }

            if (evt.type == APP_EVENT_HID_HOST) {
                process_hid_device_event(evt.hid_host_device.handle,
                                       evt.hid_host_device.event,
                                       evt.hid_host_device.arg);
            }
        }

        // Check if task should exit
        // (No USB host task anymore, so we never exit unless there's an error)
        if (ret != ESP_OK && g_hid_host_installed) {
            break;
        }
    }

    ESP_LOGI(TAG, "HID event task exiting (processed %d queued events, %d handle cycles)",
             event_count, handle_count);
    g_hid_event_task_handle = NULL;
    vTaskDelete(NULL);
}

bool usb_keyboard_init(void)
{
    ESP_LOGI(TAG, "Initializing USB HID keyboard...");

    // Create event queue
    g_app_event_queue = xQueueCreate(10, sizeof(app_event_t));
    if (!g_app_event_queue) {
        ESP_LOGE(TAG, "Failed to create event queue");
        return false;
    }

    // IMPORTANT: Create HID event processing task BEFORE installing HID host
    // The task must be running to handle device connection events
    ESP_LOGI(TAG, "Creating HID event task...");
    BaseType_t ret = xTaskCreatePinnedToCore(
        hid_event_task,
        "hid_events",
        4096,
        NULL,
        5,
        &g_hid_event_task_handle,
        0
    );

    if (ret != pdPASS) {
        ESP_LOGE(TAG, "Failed to create HID event task");
        vQueueDelete(g_app_event_queue);
        return false;
    }

    // Give the event task time to start
    vTaskDelay(pdMS_TO_TICKS(100));

    // Use BSP to start USB host (this enables USB power!)
    // The BSP function handles USB power and host library initialization
    ESP_LOGI(TAG, "Starting BSP USB host (enables USB power)...");
    esp_err_t err = bsp_usb_host_start(BSP_USB_HOST_POWER_MODE_USB_DEV, false);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to start BSP USB host: %d", err);
        vQueueDelete(g_app_event_queue);
        g_app_event_queue = NULL;
        return false;
    }

    ESP_LOGI(TAG, "BSP USB host started successfully");

    // Now install HID host driver
    ESP_LOGI(TAG, "Installing HID host driver...");
    const hid_host_driver_config_t hid_host_driver_config = {
        .create_background_task = false,  // We handle events manually
        .task_priority = 5,
        .stack_size = 4096,
        .core_id = 0,
        .callback = hid_host_device_callback,
        .callback_arg = NULL
    };

    err = hid_host_install(&hid_host_driver_config);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to install HID host: %d", err);
        // Cleanup
        bsp_usb_host_stop();
        vQueueDelete(g_app_event_queue);
        g_app_event_queue = NULL;
        return false;
    }

    // Signal that HID host is installed (event task can now process events)
    g_hid_host_installed = true;
    ESP_LOGI(TAG, "HID host installed successfully");
    ESP_LOGI(TAG, "USB HID keyboard initialized (waiting for connection)");
    return true;
}

bool usb_keyboard_is_connected(void)
{
    return g_keyboard_connected;
}

void usb_keyboard_deinit(void)
{
    ESP_LOGI(TAG, "Deinitializing USB HID keyboard...");

    g_keyboard_connected = false;

    // Signal tasks to exit
    if (g_hid_event_task_handle) {
        vTaskDelay(pdMS_TO_TICKS(100));  // Give task time to exit
    }

    // Uninstall HID host
    hid_host_uninstall();

    // Cleanup queue
    if (g_app_event_queue) {
        vQueueDelete(g_app_event_queue);
        g_app_event_queue = NULL;
    }

    ESP_LOGI(TAG, "USB HID keyboard deinitialized");
}
