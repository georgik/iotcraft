/**
 * @file device_manager.c
 * @brief Device manager implementation for IoT devices
 */

#include "device_manager.h"
#include "iotcraft_mqtt.h"
#include "world.h"
#include "esp_log.h"
#include <string.h>
#include <stdio.h>

static const char* TAG = "DeviceManager";

// Device storage
static iot_device_t g_devices[MAX_DEVICES];
static int g_device_count = 0;

// Blink timing
static float g_blink_timer = 0.0f;
static const float BLINK_INTERVAL = 2000.0f;  // 2000ms (2 second) blink cycle
static bool g_current_blink_state = false;  // Current blink phase (true=bright, false=dark)

void device_manager_init(void) {
    memset(g_devices, 0, sizeof(g_devices));
    g_device_count = 0;
    ESP_LOGI(TAG, "Device manager initialized");
}

iot_device_t* device_manager_get_or_create(const char* device_id) {
    // Try to find existing device
    iot_device_t* existing = device_manager_find(device_id);
    if (existing) {
        return existing;
    }

    // Create new device
    if (g_device_count >= MAX_DEVICES) {
        ESP_LOGW(TAG, "Max devices reached, cannot add: %s", device_id);
        return NULL;
    }

    iot_device_t* device = &g_devices[g_device_count];
    strncpy(device->id, device_id, sizeof(device->id) - 1);
    device->id[sizeof(device->id) - 1] = '\0';
    device->type = DEVICE_TYPE_UNKNOWN;
    device->state = DEVICE_STATE_OFFLINE;
    device->x = 0.0f;
    device->y = 0.0f;
    device->z = 0.0f;
    device->is_visible = false;
    device->is_blinking = false;

    g_device_count++;
    ESP_LOGI(TAG, "Created device: %s (total: %d)", device_id, g_device_count);
    return device;
}

iot_device_t* device_manager_find(const char* device_id) {
    for (int i = 0; i < g_device_count; i++) {
        if (strcmp(g_devices[i].id, device_id) == 0) {
            return &g_devices[i];
        }
    }
    return NULL;
}

void device_manager_update_position(iot_device_t* device, float x, float y, float z) {
    if (!device) return;

    device->x = x;
    device->y = y;
    device->z = z;
    device->is_visible = true;

    ESP_LOGI(TAG, "Device %s position updated: (%.1f, %.1f, %.1f)",
             device->id, x, y, z);

    // Place initial block in world (dark stone = off state)
    // NOTE: This assumes world pointer is available globally or we need to pass it
    // For now, we'll place the block when update() is first called
}

void device_manager_set_blinking(iot_device_t* device, bool blinking) {
    if (!device) return;

    device->is_blinking = blinking;
    device->state = blinking ? DEVICE_STATE_BLINKING : DEVICE_STATE_ONLINE;

    ESP_LOGI(TAG, "Device %s blink state: %s",
             device->id, blinking ? "ON" : "OFF");
}

void device_manager_update(voxel_world_t* world, float delta_time) {
    // Update blink timer
    static bool last_blink_state = false;
    g_blink_timer += delta_time;
    bool blink_state = (g_blink_timer < BLINK_INTERVAL / 2.0f);

    // Reset blink timer if cycle complete
    if (g_blink_timer >= BLINK_INTERVAL) {
        g_blink_timer = 0.0f;
    }

    // Check if blink state changed
    bool blink_changed = (blink_state != last_blink_state);
    if (blink_changed) {
        last_blink_state = blink_state;
    }

    // Store global blink state for renderer access
    g_current_blink_state = blink_state;

    // Update all devices
    for (int i = 0; i < g_device_count; i++) {
        iot_device_t* device = &g_devices[i];

        if (!device->is_visible || device->type != DEVICE_TYPE_LAMP) {
            continue;
        }

        // Calculate device position in voxel coordinates
        int32_t vx = (int32_t)device->x;
        int32_t vy = (int32_t)device->y;
        int32_t vz = (int32_t)device->z;

        // Determine block type based on blink state
        block_type_t block_type = BLOCK_STONE;  // Default: dark stone (off)

        if (device->is_blinking && blink_state) {
            block_type = BLOCK_QUARTZ;  // Light quartz (on)
        }

        // Only update when needed to minimize race conditions
        // 1. When blink state changes (every ~500ms for blinking devices)
        // 2. When device first becomes visible (initial placement)
        static bool device_initialized[MAX_DEVICES] = {false};

        if (blink_changed && device->is_blinking) {
            // Blink state changed - update block and send MQTT command
            world_set_block(world, vx, vy, vz, block_type);

            // Send MQTT command to actual lamp to turn ON/OFF
            const char* mqtt_state = blink_state ? "ON" : "OFF";
            iotcraft_mqtt_send_blink_command(device->id, mqtt_state);

            ESP_LOGI(TAG, "Blink state changed for %s: %s (sent via MQTT)",
                     device->id, mqtt_state);
        } else if (!device_initialized[i]) {
            // First time seeing this device - place initial block
            world_set_block(world, vx, vy, vz, block_type);
            device_initialized[i] = true;

            // Send initial state to lamp
            if (device->is_blinking && blink_state) {
                iotcraft_mqtt_send_blink_command(device->id, "ON");
            } else {
                iotcraft_mqtt_send_blink_command(device->id, "OFF");
            }
        }
    }
}

const iot_device_t* device_manager_get_all(int* count) {
    if (count) {
        *count = g_device_count;
    }
    return g_devices;
}

int device_manager_blink_all(bool blinking) {
    int count = 0;

    for (int i = 0; i < g_device_count; i++) {
        iot_device_t* device = &g_devices[i];

        if (device->type == DEVICE_TYPE_LAMP) {
            device_manager_set_blinking(device, blinking);

            // Send MQTT command
            iotcraft_mqtt_send_blink_command(device->id, blinking ? "ON" : "OFF");
            count++;
        }
    }

    ESP_LOGI(TAG, "Sent blink %s command to %d lamp(s)",
             blinking ? "ON" : "OFF", count);

    return count;
}

bool device_manager_is_blinking_lamp(int32_t x, int32_t y, int32_t z) {
    for (int i = 0; i < g_device_count; i++) {
        const iot_device_t* device = &g_devices[i];

        if (device->type == DEVICE_TYPE_LAMP &&
            device->is_visible &&
            device->is_blinking &&
            (int32_t)device->x == x &&
            (int32_t)device->y == y &&
            (int32_t)device->z == z) {
            return g_current_blink_state;  // Bright during "on" phase
        }
    }
    return false;
}
