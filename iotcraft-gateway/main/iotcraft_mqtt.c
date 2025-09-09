#include "iotcraft_gateway.h"
#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

// Include Mosquitto broker header  
#include "mosquitto_broker.h"

static const char *TAG = "IOTCRAFT_MQTT";
static bool mqtt_broker_running = false;
static TaskHandle_t mqtt_broker_task_handle = NULL;

// For now, just create a placeholder for the broker configuration
static int mqtt_broker_port = 1883;

static void mqtt_broker_task(void *param)
{
    ESP_LOGI(TAG, "Starting MQTT broker on port %d", mqtt_broker_port);

    // TODO: Initialize Mosquitto broker using proper API once confirmed
    // For now, just simulate a running broker loop
    mqtt_broker_running = true;
    ESP_LOGI(TAG, "(Simulated) MQTT broker started successfully");

    while (mqtt_broker_running) {
        vTaskDelay(pdMS_TO_TICKS(1000));
    }

    ESP_LOGI(TAG, "(Simulated) MQTT broker stopped");
    vTaskDelete(NULL);
}

esp_err_t iotcraft_mqtt_broker_init(void)
{
    if (mqtt_broker_running) {
        ESP_LOGW(TAG, "MQTT broker already running");
        return ESP_OK;
    }
    
    // Create MQTT broker task
    BaseType_t ret = xTaskCreate(
        mqtt_broker_task,
        "mqtt_broker",
        8192,  // Stack size - MQTT broker needs substantial stack
        NULL,
        5,     // Priority
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
