/**
 * @file wifi_init.h
 * @brief WiFi connectivity using ESP-Hosted (ESP32-C6)
 */

#pragma once

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Callback function type for WiFi IP acquisition events
 * @param ip Acquired IP address string (e.g., "192.168.1.100")
 */
typedef void (*wifi_got_ip_callback_t)(const char* ip);

/**
 * @brief Initialize WiFi using ESP-Hosted (non-blocking)
 *
 * This function initializes ESP-Hosted to communicate with the on-board
 * ESP32-C6 and starts WiFi connection. Returns immediately.
 * Use wifi_set_got_ip_callback() to be notified when IP is acquired.
 *
 * @param ssid WiFi SSID to connect to
 * @param password WiFi password
 * @return ESP_OK on success, error code otherwise
 */
int wifi_init(const char* ssid, const char* password);

/**
 * @brief Set callback for WiFi IP acquisition event
 *
 * This callback will be invoked when WiFi acquires an IP address via DHCP.
 *
 * @param callback Function to call when IP is acquired (NULL to disable)
 */
void wifi_set_got_ip_callback(wifi_got_ip_callback_t callback);

/**
 * @brief Check if WiFi is connected
 * @return true if connected and has IP, false otherwise
 */
bool wifi_is_connected(void);

/**
 * @brief Get current WiFi IP address
 * @param buf Buffer to store IP string
 * @param buf_size Size of buffer
 * @return ESP_OK on success, error code otherwise
 */
int wifi_get_ip(char* buf, size_t buf_size);

#ifdef __cplusplus
}
#endif
