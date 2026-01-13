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

static const char* TAG = "USBKeyboard";

// State tracking
static bool g_keyboard_connected = false;
static TaskHandle_t g_usb_host_task_handle = NULL;
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
        return;
    }

    // HID boot report format: modifier (1 byte) + reserved (1 byte) + 6 key codes
    static uint8_t prev_keys[6] = {0};

    // Process each key slot
    for (int i = 0; i < 6; i++) {
        uint8_t key_code = data[2 + i];

        // Key released (was in previous report but not in current)
        if (prev_keys[i] > 0 && !key_found(data + 2, prev_keys[i], 6)) {
            input_update_key(prev_keys[i], false);
            ESP_LOGV(TAG, "Key released: %d", prev_keys[i]);
        }

        // Key pressed (is in current report but was not in previous)
        if (key_code > 0 && !key_found(prev_keys, key_code, 6)) {
            input_update_key(key_code, true);
            ESP_LOGV(TAG, "Key pressed: %d", key_code);
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
        return;
    }

    switch (event) {
    case HID_HOST_DRIVER_EVENT_CONNECTED:
        ESP_LOGI(TAG, "HID Device connected: protocol %d", dev_params.proto);

        if (dev_params.proto == HID_PROTOCOL_KEYBOARD) {
            const hid_host_device_config_t dev_config = {
                .callback = hid_host_interface_callback,
                .callback_arg = NULL
            };

            ret = hid_host_device_open(hid_device_handle, &dev_config);
            if (ret == ESP_OK) {
                // Set boot protocol
                hid_class_request_set_protocol(hid_device_handle, HID_REPORT_PROTOCOL_BOOT);
                hid_class_request_set_idle(hid_device_handle, 0, 0);

                ret = hid_host_device_start(hid_device_handle);
                if (ret == ESP_OK) {
                    g_keyboard_connected = true;
                    ESP_LOGI(TAG, "USB Keyboard connected and ready");
                }
            }
        }
        break;

    default:
        break;
    }
}

/**
 * @brief USB host library task
 */
static void usb_host_task(void *arg)
{
    const usb_host_config_t host_config = {
        .skip_phy_setup = false,
        .intr_flags = ESP_INTR_FLAG_LEVEL1,
    };

    esp_err_t ret = usb_host_install(&host_config);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to install USB host: %d", ret);
        vTaskDelete(NULL);
        return;
    }

    ESP_LOGI(TAG, "USB host installed");

    // Process USB host events
    while (true) {
        uint32_t event_flags;
        ret = usb_host_lib_handle_events(portMAX_DELAY, &event_flags);

        if (ret != ESP_OK) {
            ESP_LOGE(TAG, "USB host lib error: %d", ret);
            break;
        }

        // Check if no more clients
        if (event_flags & USB_HOST_LIB_EVENT_FLAGS_NO_CLIENTS) {
            usb_host_device_free_all();
            break;
        }
    }

    // Cleanup
    vTaskDelay(pdMS_TO_TICKS(10));
    usb_host_uninstall();
    g_usb_host_task_handle = NULL;
    vTaskDelete(NULL);
}

/**
 * @brief HID event processing task
 */
static void hid_event_task(void *arg)
{
    ESP_LOGI(TAG, "HID event task started");

    while (true) {
        app_event_t evt;

        if (xQueueReceive(g_app_event_queue, &evt, pdMS_TO_TICKS(100)) == pdTRUE) {
            if (evt.type == APP_EVENT_HID_HOST) {
                process_hid_device_event(evt.hid_host_device.handle,
                                       evt.hid_host_device.event,
                                       evt.hid_host_device.arg);
            }
        }

        // Check if task should exit
        if (g_usb_host_task_handle == NULL) {
            break;
        }
    }

    ESP_LOGI(TAG, "HID event task exiting");
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

    // Create USB host task
    BaseType_t ret = xTaskCreatePinnedToCore(
        usb_host_task,
        "usb_host",
        4096,
        NULL,
        5,
        &g_usb_host_task_handle,
        0
    );

    if (ret != pdPASS) {
        ESP_LOGE(TAG, "Failed to create USB host task");
        vQueueDelete(g_app_event_queue);
        return false;
    }

    // Wait for USB host to initialize
    vTaskDelay(pdMS_TO_TICKS(100));

    // Configure HID host driver
    const hid_host_driver_config_t hid_host_driver_config = {
        .create_background_task = false,
        .task_priority = 5,
        .stack_size = 4096,
        .core_id = 0,
        .callback = hid_host_device_callback,
        .callback_arg = NULL
    };

    esp_err_t err = hid_host_install(&hid_host_driver_config);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to install HID host: %d", err);
        // Cleanup will happen when USB host task detects no clients
        return false;
    }

    // Create HID event processing task
    ret = xTaskCreatePinnedToCore(
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
        return false;
    }

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
