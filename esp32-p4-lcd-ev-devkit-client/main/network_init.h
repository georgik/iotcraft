/**
 * @file network_init.h
 * @brief Network connectivity initialization (Ethernet/WiFi)
 */

#pragma once

#include <stdbool.h>
#include "esp_err.h"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Callback function type for IP acquisition events
 * @param ip Acquired IP address string (e.g., "192.168.1.100")
 */
typedef void (*network_got_ip_callback_t)(const char* ip);

/**
 * @brief Initialize network connectivity (non-blocking)
 *
 * This function initializes Ethernet and returns immediately.
 * IP acquisition happens in background. Use network_set_got_ip_callback()
 * to be notified when IP is acquired.
 *
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t network_init(void);

/**
 * @brief Set callback for IP acquisition event
 *
 * This callback will be invoked when Ethernet acquires an IP address via DHCP.
 * The callback is invoked from the Ethernet event handler context.
 *
 * @param callback Function to call when IP is acquired (NULL to disable)
 */
void network_set_got_ip_callback(network_got_ip_callback_t callback);

/**
 * @brief Check if network is connected
 * @return true if connected and has IP, false otherwise
 */
bool network_is_connected(void);

/**
 * @brief Get IP address as string
 * @param buf Buffer to store IP string
 * @param buf_size Buffer size
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t network_get_ip(char* buf, size_t buf_size);

#ifdef __cplusplus
}
#endif
