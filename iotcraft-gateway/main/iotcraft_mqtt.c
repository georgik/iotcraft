#include "iotcraft_gateway.h"
#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

// Include Mosquitto broker header for ESP-IDF port
#include "mosq_broker.h"

static const char *TAG = "IOTCRAFT_MQTT";
static bool mqtt_broker_running = false;
static TaskHandle_t mqtt_broker_task_handle = NULL;

// MQTT broker configuration
static int mqtt_broker_port = 1883;

static void mqtt_broker_task(void *param)
{
    ESP_LOGI(TAG, "Starting MQTT broker on port %d", mqtt_broker_port);

    // Configure the broker according to ESP-IDF Mosquitto port documentation
    struct mosq_broker_config config = {
        .host = "0.0.0.0",  // Listen on all interfaces
        .port = mqtt_broker_port,  // Standard MQTT port
        .tls_cfg = NULL,   // Plain TCP (no TLS)
        .handle_message_cb = NULL  // No message callback
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
