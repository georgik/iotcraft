#include <string.h>
#include <stdio.h>
#include "esp_log.h"
#include "esp_wifi.h"
#include "esp_netif.h"
#include "esp_event.h"
#include "nvs_flash.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/event_groups.h"
#include "mqtt_client.h"
#include "esp_mac.h"
#include "esp_timer.h"

static const char *TAG = "SWIFT_WRAPPER";

// WiFi configuration
#define WIFI_SSID "IOTCRAFT_DEMO"
#define WIFI_PASSWORD "demo123456"
#define WIFI_CONNECTED_BIT BIT0
#define WIFI_FAIL_BIT BIT1
#define ESP_MAXIMUM_RETRY 5

static EventGroupHandle_t s_wifi_event_group;
static int s_retry_num = 0;
static bool wifi_initialized = false;
static bool mqtt_initialized = false;
static esp_mqtt_client_handle_t mqtt_client = NULL;

// LED control callback function pointer
static void (*led_control_callback)(bool is_on) = NULL;

// MQTT broker configuration
#define MQTT_BROKER_URI "mqtt://192.168.4.1:1883"

// WiFi event handler
static void event_handler(void* arg, esp_event_base_t event_base, int32_t event_id, void* event_data)
{
    if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_START) {
        esp_wifi_connect();
    } else if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_DISCONNECTED) {
        if (s_retry_num < ESP_MAXIMUM_RETRY) {
            esp_wifi_connect();
            s_retry_num++;
            ESP_LOGI(TAG, "retry to connect to the AP");
        } else {
            xEventGroupSetBits(s_wifi_event_group, WIFI_FAIL_BIT);
        }
        ESP_LOGI(TAG,"connect to the AP fail");
    } else if (event_base == IP_EVENT && event_id == IP_EVENT_STA_GOT_IP) {
        ip_event_got_ip_t* event = (ip_event_got_ip_t*) event_data;
        ESP_LOGI(TAG, "got ip:" IPSTR, IP2STR(&event->ip_info.ip));
        s_retry_num = 0;
        xEventGroupSetBits(s_wifi_event_group, WIFI_CONNECTED_BIT);
    }
}

// MQTT event handler
static void mqtt_event_handler(void *handler_args, esp_event_base_t base, int32_t event_id, void *event_data)
{
    esp_mqtt_event_handle_t event = event_data;
    esp_mqtt_client_handle_t client = event->client;
    int msg_id;
    
    switch ((esp_mqtt_event_id_t)event_id) {
    case MQTT_EVENT_CONNECTED:
        ESP_LOGI(TAG, "MQTT_EVENT_CONNECTED");
        // Get device ID for topic subscriptions
        uint8_t mac[6];
        esp_read_mac(mac, ESP_MAC_WIFI_STA);
        char device_id[32];
        snprintf(device_id, sizeof(device_id), "esp32c6-%02x%02x%02x%02x%02x%02x", 
                 mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
        
        // Subscribe to device-specific light control topic
        char light_topic[64];
        snprintf(light_topic, sizeof(light_topic), "home/%s/light", device_id);
        msg_id = esp_mqtt_client_subscribe(client, light_topic, 1);
        ESP_LOGI(TAG, "subscribed to %s, msg_id=%d", light_topic, msg_id);
        
        // Subscribe to device position update topic
        char position_topic[64];
        snprintf(position_topic, sizeof(position_topic), "home/%s/position/set", device_id);
        msg_id = esp_mqtt_client_subscribe(client, position_topic, 1);
        ESP_LOGI(TAG, "subscribed to %s, msg_id=%d", position_topic, msg_id);
        
        // Send device announcement with full IoTCraft format including location
        char device_announce[512];
        snprintf(device_announce, sizeof(device_announce), 
                "{\"device_id\":\"%s\",\"device_type\":\"lamp\",\"state\":\"online\",\"location\":{\"x\":1.0,\"y\":0.5,\"z\":2.0}}", 
                device_id);
        
        msg_id = esp_mqtt_client_publish(client, "devices/announce", device_announce, 0, 1, 0);
        ESP_LOGI(TAG, "sent device announce successful, msg_id=%d", msg_id);
        break;
        
    case MQTT_EVENT_DISCONNECTED:
        ESP_LOGI(TAG, "MQTT_EVENT_DISCONNECTED");
        break;
        
    case MQTT_EVENT_SUBSCRIBED:
        ESP_LOGI(TAG, "MQTT_EVENT_SUBSCRIBED, msg_id=%d", event->msg_id);
        break;
        
    case MQTT_EVENT_UNSUBSCRIBED:
        ESP_LOGI(TAG, "MQTT_EVENT_UNSUBSCRIBED, msg_id=%d", event->msg_id);
        break;
        
    case MQTT_EVENT_PUBLISHED:
        ESP_LOGI(TAG, "MQTT_EVENT_PUBLISHED, msg_id=%d", event->msg_id);
        break;
        
    case MQTT_EVENT_DATA:
        ESP_LOGI(TAG, "MQTT_EVENT_DATA");
        printf("TOPIC=%.*s\r\n", event->topic_len, event->topic);
        printf("DATA=%.*s\r\n", event->data_len, event->data);
        
        // Handle MQTT commands based on topic patterns
        if (strstr(event->topic, "/light") != NULL) {
            // Handle light control commands
            if (strncmp(event->data, "ON", event->data_len) == 0) {
                ESP_LOGI(TAG, "💡 Light command: ON");
                // Call Swift LED control callback if registered
                if (led_control_callback != NULL) {
                    led_control_callback(true);
                }
            } else if (strncmp(event->data, "OFF", event->data_len) == 0) {
                ESP_LOGI(TAG, "🔹 Light command: OFF");
                // Call Swift LED control callback if registered
                if (led_control_callback != NULL) {
                    led_control_callback(false);
                }
            }
        } else if (strstr(event->topic, "/position/set") != NULL) {
            // Handle position update commands
            ESP_LOGI(TAG, "📍 Position update received: %.*s", event->data_len, event->data);
            // Position handling will be processed by Swift if needed
        }
        break;
        
    case MQTT_EVENT_ERROR:
        ESP_LOGI(TAG, "MQTT_EVENT_ERROR");
        if (event->error_handle->error_type == MQTT_ERROR_TYPE_TCP_TRANSPORT) {
            ESP_LOGI(TAG, "Last errno string (%s)", strerror(event->error_handle->esp_transport_sock_errno));
        }
        break;
        
    default:
        ESP_LOGI(TAG, "Other event id:%d", event->event_id);
        break;
    }
}

// Swift callable functions
int nvs_init(void)
{
    esp_err_t ret = nvs_flash_init();
    if (ret == ESP_ERR_NVS_NO_FREE_PAGES || ret == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_ERROR_CHECK(nvs_flash_erase());
        ret = nvs_flash_init();
    }
    ESP_LOGI(TAG, "NVS initialized");
    return (ret == ESP_OK) ? 0 : -1;
}

int wifi_init(void)
{
    if (wifi_initialized) {
        return 0;
    }
    
    ESP_ERROR_CHECK(esp_netif_init());
    ESP_ERROR_CHECK(esp_event_loop_create_default());
    esp_netif_create_default_wifi_sta();
    
    wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
    ESP_ERROR_CHECK(esp_wifi_init(&cfg));
    
    s_wifi_event_group = xEventGroupCreate();
    
    esp_event_handler_instance_t instance_any_id;
    esp_event_handler_instance_t instance_got_ip;
    ESP_ERROR_CHECK(esp_event_handler_instance_register(WIFI_EVENT, ESP_EVENT_ANY_ID, &event_handler, NULL, &instance_any_id));
    ESP_ERROR_CHECK(esp_event_handler_instance_register(IP_EVENT, IP_EVENT_STA_GOT_IP, &event_handler, NULL, &instance_got_ip));
    
    wifi_initialized = true;
    ESP_LOGI(TAG, "WiFi initialized");
    return 0;
}

int wifi_connect(const char* ssid, const char* password)
{
    if (!wifi_initialized) {
        if (wifi_init() != 0) {
            return -1;
        }
    }
    
    wifi_config_t wifi_config = {
        .sta = {
            .threshold.authmode = WIFI_AUTH_WPA2_PSK,
            .pmf_cfg = {
                .capable = true,
                .required = false
            },
        },
    };
    
    // Use provided SSID and password, or defaults
    if (ssid != NULL) {
        strncpy((char*)wifi_config.sta.ssid, ssid, sizeof(wifi_config.sta.ssid) - 1);
    } else {
        strcpy((char*)wifi_config.sta.ssid, WIFI_SSID);
    }
    
    if (password != NULL) {
        strncpy((char*)wifi_config.sta.password, password, sizeof(wifi_config.sta.password) - 1);
    } else {
        strcpy((char*)wifi_config.sta.password, WIFI_PASSWORD);
    }
    
    ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_STA));
    ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_STA, &wifi_config));
    ESP_ERROR_CHECK(esp_wifi_start());
    
    ESP_LOGI(TAG, "WiFi connecting to %s", wifi_config.sta.ssid);
    
    // Wait for either the connection to succeed or fail
    EventBits_t bits = xEventGroupWaitBits(s_wifi_event_group,
            WIFI_CONNECTED_BIT | WIFI_FAIL_BIT,
            pdFALSE,
            pdFALSE,
            portMAX_DELAY);
    
    if (bits & WIFI_CONNECTED_BIT) {
        ESP_LOGI(TAG, "connected to ap SSID:%s", wifi_config.sta.ssid);
        return 0;
    } else if (bits & WIFI_FAIL_BIT) {
        ESP_LOGI(TAG, "Failed to connect to SSID:%s", wifi_config.sta.ssid);
        return -1;
    } else {
        ESP_LOGE(TAG, "UNEXPECTED EVENT");
        return -1;
    }
}

int mqtt_client_init(void)
{
    if (mqtt_initialized) {
        return 0;
    }
    
    esp_mqtt_client_config_t mqtt_cfg = {
        .broker.address.uri = MQTT_BROKER_URI,
    };
    
    mqtt_client = esp_mqtt_client_init(&mqtt_cfg);
    if (mqtt_client == NULL) {
        ESP_LOGE(TAG, "Failed to initialize MQTT client");
        return -1;
    }
    
    ESP_ERROR_CHECK(esp_mqtt_client_register_event(mqtt_client, ESP_EVENT_ANY_ID, mqtt_event_handler, NULL));
    
    mqtt_initialized = true;
    ESP_LOGI(TAG, "MQTT client initialized");
    return 0;
}

int mqtt_client_start(void)
{
    if (!mqtt_initialized) {
        if (mqtt_client_init() != 0) {
            return -1;
        }
    }
    
    esp_err_t ret = esp_mqtt_client_start(mqtt_client);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to start MQTT client: %s", esp_err_to_name(ret));
        return -1;
    }
    
    ESP_LOGI(TAG, "MQTT client started");
    return 0;
}

int mqtt_publish(const char* topic, const char* payload)
{
    if (mqtt_client == NULL || !mqtt_initialized) {
        ESP_LOGE(TAG, "MQTT client not initialized");
        return -1;
    }
    
    int msg_id = esp_mqtt_client_publish(mqtt_client, topic, payload, 0, 1, 0);
    if (msg_id == -1) {
        ESP_LOGE(TAG, "Failed to publish MQTT message");
        return -1;
    }
    
    ESP_LOGI(TAG, "Published to %s: %s (msg_id=%d)", topic, payload, msg_id);
    return 0;
}

void generate_device_id(char* buffer, int buffer_size)
{
    if (buffer == NULL || buffer_size < 20) {
        return;
    }
    
    uint8_t mac[6];
    esp_read_mac(mac, ESP_MAC_WIFI_STA);
    snprintf(buffer, buffer_size, "esp32c6-%02x%02x%02x%02x%02x%02x", 
             mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
}

// Additional wrapper function for device announcement publishing
int mqtt_publish_device_announcement(void)
{
    if (mqtt_client == NULL || !mqtt_initialized) {
        ESP_LOGE(TAG, "MQTT client not initialized");
        return -1;
    }
    
    uint8_t mac[6];
    esp_read_mac(mac, ESP_MAC_WIFI_STA);
    char device_id[32];
    snprintf(device_id, sizeof(device_id), "esp32c6-%02x%02x%02x%02x%02x%02x", 
             mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
    
    char device_announce[512];
    snprintf(device_announce, sizeof(device_announce), 
            "{\"device_id\":\"%s\",\"device_type\":\"lamp\",\"state\":\"online\",\"location\":{\"x\":1.0,\"y\":0.5,\"z\":2.0}}", 
            device_id);
    
    int msg_id = esp_mqtt_client_publish(mqtt_client, "devices/announce", device_announce, 0, 1, 0);
    if (msg_id == -1) {
        ESP_LOGE(TAG, "Failed to publish device announcement");
        return -1;
    }
    
    ESP_LOGI(TAG, "Device announcement published: %s (msg_id=%d)", device_id, msg_id);
    return 0;
}

// Register LED control callback from Swift
void register_led_control_callback(void (*callback)(bool is_on))
{
    led_control_callback = callback;
    ESP_LOGI(TAG, "LED control callback registered");
}

void delay_ms(unsigned int ms)
{
    vTaskDelay(pdMS_TO_TICKS(ms));
}

unsigned long get_millis(void)
{
    return (unsigned long)(esp_timer_get_time() / 1000ULL);
}