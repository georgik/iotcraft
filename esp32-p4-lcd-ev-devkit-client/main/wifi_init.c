/**
 * @file wifi_init.c
 * @brief WiFi connectivity implementation using ESP-Hosted
 */

#include "wifi_init.h"
#include "esp_log.h"
#include "esp_wifi.h"
#include "esp_event.h"
#include "esp_netif.h"
#include "freertos/FreeRTOS.h"
#include "freertos/event_groups.h"
#include <string.h>

// ESP-Hosted includes (commented out until build is fixed)
// #include "esp_hosted.h"
// #include "esp_hosted_rpc.h"

static const char* TAG = "WiFi";

// Default WiFi credentials for IoTCraft network
#define DEFAULT_WIFI_SSID     "iotcraft"
#define DEFAULT_WIFI_PASSWORD "iotcraft123"

// WiFi configuration
#define WIFI_CONNECTED_BIT BIT0
#define WIFI_FAIL_BIT      BIT1

static EventGroupHandle_t s_wifi_event_group;
static wifi_got_ip_callback_t s_got_ip_callback = NULL;
static esp_netif_t* s_sta_netif = NULL;
static int s_retry_num = 0;
static bool s_connected = false;
static char s_ip_address[16] = {0};

#define MAX_RETRY 5

// WiFi event handler
static void wifi_event_handler(void* arg, esp_event_base_t event_base,
                                int32_t event_id, void* event_data) {
    if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_START) {
        esp_wifi_connect();
    } else if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_DISCONNECTED) {
        if (s_retry_num < MAX_RETRY) {
            esp_wifi_connect();
            s_retry_num++;
            ESP_LOGI(TAG, "Retry connecting to AP");
        } else {
            xEventGroupSetBits(s_wifi_event_group, WIFI_FAIL_BIT);
        }
        s_connected = false;
        ESP_LOGI(TAG, "Connection to AP failed");
    } else if (event_base == IP_EVENT && event_id == IP_EVENT_STA_GOT_IP) {
        ip_event_got_ip_t* event = (ip_event_got_ip_t*) event_data;
        snprintf(s_ip_address, sizeof(s_ip_address), IPSTR, IP2STR(&event->ip_info.ip));

        ESP_LOGI(TAG, "Got IP:" IPSTR, IP2STR(&event->ip_info.ip));
        s_retry_num = 0;
        s_connected = true;
        xEventGroupSetBits(s_wifi_event_group, WIFI_CONNECTED_BIT);

        // Invoke callback if registered
        if (s_got_ip_callback != NULL) {
            s_got_ip_callback(s_ip_address);
        }
    }
}

int wifi_init(const char* ssid, const char* password) {
    // Use default credentials if none provided
    if (!ssid || strlen(ssid) == 0) {
        ssid = DEFAULT_WIFI_SSID;
    }
    if (!password || strlen(password) == 0) {
        password = DEFAULT_WIFI_PASSWORD;
    }

    ESP_LOGI(TAG, "Initializing WiFi using ESP-Hosted...");
    ESP_LOGI(TAG, "Target SSID: %s", ssid);

    // Create event group
    s_wifi_event_group = xEventGroupCreate();
    if (!s_wifi_event_group) {
        ESP_LOGE(TAG, "Failed to create event group");
        return ESP_FAIL;
    }

    // Initialize TCP/IP stack
    ESP_ERROR_CHECK(esp_netif_init());

    // Create default event loop
    ESP_ERROR_CHECK(esp_event_loop_create_default());

    // TODO: Initialize ESP-Hosted (when includes are fixed)
    // ESP_LOGI(TAG, "Initializing ESP-Hosted...");
    // esp_err_t ret = esp_hosted_init();
    // if (ret != ESP_OK) {
    //     ESP_LOGE(TAG, "Failed to initialize ESP-Hosted: %s", esp_err_to_name(ret));
    //     return ret;
    // }
    // ESP_LOGI(TAG, "ESP-Hosted initialized successfully");

    // Create station netif
    s_sta_netif = esp_netif_create_default_wifi_sta();
    if (!s_sta_netif) {
        ESP_LOGE(TAG, "Failed to create station netif");
        return ESP_FAIL;
    }

    // Initialize WiFi with default config
    wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
    ESP_ERROR_CHECK(esp_wifi_init(&cfg));

    // Register event handlers
    ESP_ERROR_CHECK(esp_event_handler_register(WIFI_EVENT, ESP_EVENT_ANY_ID, &wifi_event_handler, NULL));
    ESP_ERROR_CHECK(esp_event_handler_register(IP_EVENT, IP_EVENT_STA_GOT_IP, &wifi_event_handler, NULL));

    // Set WiFi mode to station
    ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_STA));

    // Configure WiFi
    wifi_config_t wifi_config = {
        .sta = {
            .threshold.authmode = WIFI_AUTH_WPA2_PSK,
        },
    };

    strncpy((char*)wifi_config.sta.ssid, ssid, sizeof(wifi_config.sta.ssid) - 1);
    strncpy((char*)wifi_config.sta.password, password, sizeof(wifi_config.sta.password) - 1);

    ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_STA, &wifi_config));

    // Start WiFi (non-blocking)
    ESP_LOGI(TAG, "Starting WiFi, connecting to %s...", ssid);
    ESP_ERROR_CHECK(esp_wifi_start());

    ESP_LOGI(TAG, "WiFi started, waiting for connection...");
    return ESP_OK;
}

void wifi_set_got_ip_callback(wifi_got_ip_callback_t callback) {
    s_got_ip_callback = callback;
}

bool wifi_is_connected(void) {
    return s_connected && (strlen(s_ip_address) > 0);
}

int wifi_get_ip(char* buf, size_t buf_size) {
    if (!wifi_is_connected()) {
        return ESP_ERR_INVALID_STATE;
    }

    strncpy(buf, s_ip_address, buf_size - 1);
    buf[buf_size - 1] = '\0';
    return ESP_OK;
}
