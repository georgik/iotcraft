/**
 * @file iotcraft_mqtt.h
 * @brief IotCraft MQTT client for world synchronization and device interaction
 */

#pragma once

#include <stdbool.h>
#include <stdint.h>
#include "esp_err.h"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Initialize MQTT client
 * @param world_id World ID to connect to
 * @param broker_uri MQTT broker URI (e.g., "mqtt://localhost:1883")
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t iotcraft_mqtt_init(const char* world_id, const char* broker_uri);

/**
 * @brief Publish block placement event
 * @param x Block X coordinate
 * @param y Block Y coordinate
 * @param z Block Z coordinate
 * @param block_type Block type (as string, e.g., "stone", "grass")
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t iotcraft_mqtt_publish_block_placed(int32_t x, int32_t y, int32_t z, const char* block_type);

/**
 * @brief Publish block removal event
 * @param x Block X coordinate
 * @param y Block Y coordinate
 * @param z Block Z coordinate
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t iotcraft_mqtt_publish_block_removed(int32_t x, int32_t y, int32_t z);

/**
 * @brief Send device blink command (e.g., to redstone lamp on C6)
 * @param device_id Device ID (e.g., "c6-lamp-001")
 * @param state "ON" or "OFF"
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t iotcraft_mqtt_send_blink_command(const char* device_id, const char* state);

/**
 * @brief Check if MQTT client is connected
 * @return true if connected, false otherwise
 */
bool iotcraft_mqtt_is_connected(void);

/**
 * @brief Deinitialize MQTT client
 */
void iotcraft_mqtt_deinit(void);

#ifdef __cplusplus
}
#endif
