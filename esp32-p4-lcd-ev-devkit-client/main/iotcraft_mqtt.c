/**
 * @file iotcraft_mqtt.c
 * @brief IotCraft MQTT client implementation
 */

#include "iotcraft_mqtt.h"
#include "mqtt_client.h"
#include "console/console.h"
#include "esp_log.h"
#include "esp_event.h"
#include <string.h>
#include <sys/time.h>
#include <inttypes.h>

static const char* TAG = "IotCraftMQTT";

#define MQTT_BROKER_URI "mqtt://192.168.4.1:1883"  // IoT Gateway (DHCP server)
#define PLAYER_ID "esp32-p4-client"
#define PLAYER_NAME "ESP32-P4"

static esp_mqtt_client_handle_t g_mqtt_client = NULL;
static char g_world_id[64] = {0};
static bool g_connected = false;

/**
 * @brief Get current timestamp in milliseconds
 */
static int64_t get_timestamp_ms(void) {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (int64_t)tv.tv_sec * 1000 + tv.tv_usec / 1000;
}

/**
 * @brief MQTT event handler
 */
static void mqtt_event_handler(void* handler_args, esp_event_base_t base, int32_t event_id, void* event_data) {
    esp_mqtt_event_handle_t event = event_data;

    switch ((esp_mqtt_event_id_t)event_id) {
        case MQTT_EVENT_CONNECTED:
            ESP_LOGI(TAG, "MQTT_EVENT_CONNECTED");
            console_log(LOG_LEVEL_INFO, "MQTT", "Connected to broker");

            // Subscribe to world block events
            char placed_topic[128];
            char removed_topic[128];
            snprintf(placed_topic, sizeof(placed_topic), "iotcraft/worlds/%s/state/blocks/placed", g_world_id);
            snprintf(removed_topic, sizeof(removed_topic), "iotcraft/worlds/%s/state/blocks/removed", g_world_id);

            esp_mqtt_client_subscribe(event->client, placed_topic, 1);
            esp_mqtt_client_subscribe(event->client, removed_topic, 1);

            ESP_LOGI(TAG, "Subscribed to world topics for %s", g_world_id);
            g_connected = true;
            break;

        case MQTT_EVENT_DISCONNECTED:
            ESP_LOGW(TAG, "MQTT_EVENT_DISCONNECTED");
            console_log(LOG_LEVEL_WARN, "MQTT", "Disconnected from broker");
            g_connected = false;
            break;

        case MQTT_EVENT_DATA:
            ESP_LOGI(TAG, "MQTT_EVENT_DATA: topic=%.*s", event->topic_len, event->topic);
            // Handle incoming block changes from other players
            // For now, just log - we'll implement full sync later
            break;

        case MQTT_EVENT_ERROR:
            ESP_LOGE(TAG, "MQTT_EVENT_ERROR");
            console_log(LOG_LEVEL_ERROR, "MQTT", "Connection error");
            if (event->error_handle->error_type == MQTT_ERROR_TYPE_TCP_TRANSPORT) {
                console_log(LOG_LEVEL_ERROR, "MQTT", "Transport error: %d",
                           event->error_handle->esp_transport_sock_errno);
            }
            break;

        default:
            break;
    }
}

esp_err_t iotcraft_mqtt_init(const char* world_id, const char* broker_uri) {
    if (g_mqtt_client) {
        ESP_LOGW(TAG, "MQTT client already initialized");
        return ESP_OK;
    }

    if (!world_id) {
        ESP_LOGE(TAG, "World ID is NULL");
        return ESP_ERR_INVALID_ARG;
    }

    // Store world ID
    strlcpy(g_world_id, world_id, sizeof(g_world_id));

    // Use provided URI or default
    const char* uri = broker_uri ? broker_uri : MQTT_BROKER_URI;

    ESP_LOGI(TAG, "Initializing MQTT client for world: %s", world_id);
    ESP_LOGI(TAG, "Broker URI: %s", uri);
    console_log(LOG_LEVEL_INFO, "MQTT", "Broker: %s", uri);

    // Configure MQTT client
    esp_mqtt_client_config_t mqtt_cfg = {
        .broker.address.uri = uri,
        .credentials = {
            .client_id = PLAYER_ID,
        },
        .session = {
            .last_will = {
                .topic = NULL,  // No LWT for now
                .msg = NULL,
                .qos = 1,
                .retain = 0,
            },
        },
        .network = {
            .timeout_ms = 5000,
        },
    };

    // Initialize MQTT client
    g_mqtt_client = esp_mqtt_client_init(&mqtt_cfg);
    if (!g_mqtt_client) {
        ESP_LOGE(TAG, "Failed to initialize MQTT client");
        return ESP_FAIL;
    }

    // Register event handler
    esp_err_t ret = esp_mqtt_client_register_event(
        g_mqtt_client,
        ESP_EVENT_ANY_ID,
        mqtt_event_handler,
        NULL
    );
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to register MQTT event handler: %d", ret);
        esp_mqtt_client_destroy(g_mqtt_client);
        g_mqtt_client = NULL;
        return ret;
    }

    // Start MQTT client
    ret = esp_mqtt_client_start(g_mqtt_client);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to start MQTT client: %d", ret);
        esp_mqtt_client_destroy(g_mqtt_client);
        g_mqtt_client = NULL;
        return ret;
    }

    ESP_LOGI(TAG, "MQTT client initialized successfully");
    return ESP_OK;
}

esp_err_t iotcraft_mqtt_publish_block_placed(int32_t x, int32_t y, int32_t z, const char* block_type) {
    if (!g_mqtt_client || !g_connected) {
        ESP_LOGW(TAG, "MQTT client not connected, skipping block placed event");
        return ESP_ERR_INVALID_STATE;
    }

    // Create JSON payload
    char payload[256];
    snprintf(payload, sizeof(payload),
             "{\"player_id\":\"%s\",\"player_name\":\"%s\",\"timestamp\":%lld,"
             "\"change\":{\"Placed\":{\"x\":%" PRId32 ",\"y\":%" PRId32 ",\"z\":%" PRId32 ",\"block_type\":\"%s\"}}}",
             PLAYER_ID, PLAYER_NAME, get_timestamp_ms(),
             x, y, z, block_type);

    // Create topic
    char topic[128];
    snprintf(topic, sizeof(topic), "iotcraft/worlds/%s/state/blocks/placed", g_world_id);

    // Publish
    int msg_id = esp_mqtt_client_publish(g_mqtt_client, topic, payload, 0, 1, 0);
    if (msg_id < 0) {
        ESP_LOGE(TAG, "Failed to publish block placed event");
        return ESP_FAIL;
    }

    ESP_LOGI(TAG, "Published block placed: (%d,%d,%d) type=%s", x, y, z, block_type);
    return ESP_OK;
}

esp_err_t iotcraft_mqtt_publish_block_removed(int32_t x, int32_t y, int32_t z) {
    if (!g_mqtt_client || !g_connected) {
        ESP_LOGW(TAG, "MQTT client not connected, skipping block removed event");
        return ESP_ERR_INVALID_STATE;
    }

    // Create JSON payload
    char payload[256];
    snprintf(payload, sizeof(payload),
             "{\"player_id\":\"%s\",\"player_name\":\"%s\",\"timestamp\":%lld,"
             "\"change\":{\"Removed\":{\"x\":%" PRId32 ",\"y\":%" PRId32 ",\"z\":%" PRId32 "}}}",
             PLAYER_ID, PLAYER_NAME, get_timestamp_ms(),
             x, y, z);

    // Create topic
    char topic[128];
    snprintf(topic, sizeof(topic), "iotcraft/worlds/%s/state/blocks/removed", g_world_id);

    // Publish
    int msg_id = esp_mqtt_client_publish(g_mqtt_client, topic, payload, 0, 1, 0);
    if (msg_id < 0) {
        ESP_LOGE(TAG, "Failed to publish block removed event");
        return ESP_FAIL;
    }

    ESP_LOGI(TAG, "Published block removed: (%d,%d,%d)", x, y, z);
    return ESP_OK;
}

esp_err_t iotcraft_mqtt_send_blink_command(const char* device_id, const char* state) {
    if (!g_mqtt_client || !g_connected) {
        ESP_LOGW(TAG, "MQTT client not connected, skipping blink command");
        return ESP_ERR_INVALID_STATE;
    }

    if (!device_id || !state) {
        ESP_LOGE(TAG, "Invalid device_id or state");
        return ESP_ERR_INVALID_ARG;
    }

    // Create topic: home/{device_id}/light
    char topic[128];
    snprintf(topic, sizeof(topic), "home/%s/light", device_id);

    // Publish command (payload is "ON" or "OFF")
    int msg_id = esp_mqtt_client_publish(g_mqtt_client, topic, state, strlen(state), 0, 0);
    if (msg_id < 0) {
        ESP_LOGE(TAG, "Failed to publish blink command");
        return ESP_FAIL;
    }

    ESP_LOGI(TAG, "Sent blink command to device %s: %s", device_id, state);
    return ESP_OK;
}

bool iotcraft_mqtt_is_connected(void) {
    return g_connected;
}

void iotcraft_mqtt_deinit(void) {
    if (g_mqtt_client) {
        ESP_LOGI(TAG, "Stopping MQTT client...");
        esp_mqtt_client_stop(g_mqtt_client);
        esp_mqtt_client_destroy(g_mqtt_client);
        g_mqtt_client = NULL;
        g_connected = false;
    }
}
