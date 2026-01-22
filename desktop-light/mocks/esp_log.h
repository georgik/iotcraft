/**
 * @file esp_log_mock.h
 * @brief Mock ESP-IDF logging functions for desktop simulator
 */

#ifndef ESP_LOG_MOCK_H
#define ESP_LOG_MOCK_H

#include <stdio.h>

// ESP log levels
typedef enum {
    ESP_LOG_NONE,
    ESP_LOG_ERROR,
    ESP_LOG_WARN,
    ESP_LOG_INFO,
    ESP_LOG_DEBUG,
    ESP_LOG_VERBOSE
} esp_log_level_t;

// Mock macros - map to printf with colors
#define ESP_LOGE(tag, fmt, ...) \
    printf("\033[31m[E] %s: " fmt "\033[0m\n", tag, ##__VA_ARGS__)

#define ESP_LOGW(tag, fmt, ...) \
    printf("\033[33m[W] %s: " fmt "\033[0m\n", tag, ##__VA_ARGS__)

#define ESP_LOGI(tag, fmt, ...) \
    printf("[I] %s: " fmt "\n", tag, ##__VA_ARGS__)

#define ESP_LOGD(tag, fmt, ...) \
    printf("[D] %s: " fmt "\n", tag, ##__VA_ARGS__)

#define ESP_LOGV(tag, fmt, ...) \
    printf("[V] %s: " fmt "\n", tag, ##__VA_ARGS__)

// Also mock esp_log.h if included
#ifdef ESP_LOG_H
#undef ESP_LOG_H
#endif
#define ESP_LOG_H

#endif // ESP_LOG_MOCK_H
