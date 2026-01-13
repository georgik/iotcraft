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
 * @brief Initialize network connectivity
 *
 * This function initializes Ethernet (or WiFi in the future)
 * and waits for IP connection.
 *
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t network_init(void);

/**
 * @brief Check if network is connected
 * @return true if connected, false otherwise
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
