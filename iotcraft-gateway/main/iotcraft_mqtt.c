#include "iotcraft_gateway.h"
#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

// Include Mosquitto broker header for ESP-IDF port
#include "mosq_broker.h"

static const char *TAG = "IOTCRAFT_MQTT";
static bool mqtt_broker_running = false;
static TaskHandle_t mqtt_broker_task_handle = NULL;
static int mqtt_connected_clients = 0;

// MQTT broker configuration
static int mqtt_broker_port = 1883;

// MQTT client tracking functions
static void mqtt_client_connected(void)
{
    mqtt_connected_clients++;
    ESP_LOGI(TAG, "MQTT client connected (Total clients: %d)", mqtt_connected_clients);
}

static void mqtt_client_disconnected(void)
{
    if (mqtt_connected_clients > 0) {
        mqtt_connected_clients--;
    }
    ESP_LOGI(TAG, "MQTT client disconnected (Total clients: %d)", mqtt_connected_clients);
}

// Function to manually update client count (can be called from network monitoring)
void iotcraft_mqtt_set_client_count(int count)
{
    mqtt_connected_clients = count;
    ESP_LOGI(TAG, "MQTT client count updated: %d", mqtt_connected_clients);
}

// MQTT message callback - called when broker processes any message
// We can use this to infer client activity and estimate connections
static void mqtt_message_callback(char *client, char *topic, char *data, int len, int qos, int retain)
{
    // Log message activity for debugging
    ESP_LOGD(TAG, "MQTT message from client '%s' on topic '%s' (len=%d, qos=%d, retain=%d)", 
             client ? client : "unknown", topic ? topic : "unknown", len, qos, retain);
    
    // Simple heuristic: if we see messages, assume at least 1 client is connected
    // This is not perfect but better than showing 0 when clients are active
    if (mqtt_connected_clients == 0) {
        ESP_LOGI(TAG, "Detected MQTT client activity - updating count to 1");
        mqtt_connected_clients = 1;
    }
}

static void mqtt_broker_task(void *param)
{
    ESP_LOGI(TAG, "Starting MQTT broker on port %d", mqtt_broker_port);

    // Configure the broker according to ESP-IDF Mosquitto port documentation
    struct mosq_broker_config config = {
        .host = "0.0.0.0",  // Listen on all interfaces
        .port = mqtt_broker_port,  // Standard MQTT port
        .tls_cfg = NULL,   // Plain TCP (no TLS)
        .handle_message_cb = mqtt_message_callback  // Track messages to estimate client activity
    };

    mqtt_broker_running = true;
    ESP_LOGI(TAG, "MQTT broker started successfully on port %d", mqtt_broker_port);

    // Start the broker (runs in the current task)
    // According to the documentation, this is a blocking call
    int ret = mosq_broker_run(&config);
    
    if (ret != 0) {
        ESP_LOGE(TAG, "MQTT broker failed to start or exited with error: %d", ret);
    } else {
        ESP_LOGI(TAG, "MQTT broker stopped normally");
    }
    
    mqtt_broker_running = false;
    vTaskDelete(NULL);
}

esp_err_t iotcraft_mqtt_broker_init(void)
{
    if (mqtt_broker_running) {
        ESP_LOGW(TAG, "MQTT broker already running");
        return ESP_OK;
    }
    
    // Create MQTT broker task with adequate stack size
    // According to documentation: minimum 5KB stack, but we use more for safety
    BaseType_t ret = xTaskCreate(
        mqtt_broker_task,
        "mqtt_broker",
        12288,  // 12KB stack size - Mosquitto broker needs substantial stack 
        NULL,
        5,      // Priority
        &mqtt_broker_task_handle
    );
    
    if (ret != pdPASS) {
        ESP_LOGE(TAG, "Failed to create MQTT broker task");
        return ESP_FAIL;
    }
    
    // Wait a bit for the task to start
    vTaskDelay(pdMS_TO_TICKS(100));
    
    ESP_LOGI(TAG, "MQTT broker task created");
    return ESP_OK;
}

esp_err_t iotcraft_mqtt_broker_stop(void)
{
    if (!mqtt_broker_running) {
        return ESP_OK;
    }
    
    mqtt_broker_running = false;
    
    if (mqtt_broker_task_handle != NULL) {
        // Wait for task to finish
        vTaskDelay(pdMS_TO_TICKS(1000));
        mqtt_broker_task_handle = NULL;
    }
    
    ESP_LOGI(TAG, "MQTT broker stopped");
    return ESP_OK;
}

bool iotcraft_mqtt_is_running(void)
{
    return mqtt_broker_running;
}

int iotcraft_mqtt_get_client_count(void)
{
    return mqtt_connected_clients;
}

// Function to estimate MQTT client count by monitoring TCP connections on port 1883
// This is a workaround since ESP-IDF Mosquitto port doesn't expose connection callbacks
static int estimate_mqtt_client_count(void)
{
    // TODO: Implement actual TCP connection monitoring
    // For now, return a placeholder that can be updated manually
    // In a full implementation, this could:
    // 1. Query netstat-like information
    // 2. Monitor TCP connection state on port 1883
    // 3. Track CONNACK/DISCONNECT messages via handle_message_cb
    
    // This is a simplistic approach - in practice, you'd want to:
    // - Monitor TCP sockets on port 1883
    // - Track MQTT CONNECT/DISCONNECT messages
    // - Use system networking APIs to count connections
    
    return mqtt_connected_clients;  // Return current tracked count
}

// Function to update MQTT client count periodically
void iotcraft_mqtt_update_client_count(void)
{
    // This could be called periodically to update the count
    // For now, we'll leave the count as manually managed
    // In future implementations, this could call estimate_mqtt_client_count()
    
    ESP_LOGD(TAG, "Current MQTT client count: %d", mqtt_connected_clients);
}
