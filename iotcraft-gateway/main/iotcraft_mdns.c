#include "iotcraft_gateway.h"
#include "esp_log.h"
#include "mdns.h"

static const char *TAG = "IOTCRAFT_MDNS";
static bool mdns_initialized = false;

esp_err_t iotcraft_mdns_init(void)
{
    if (mdns_initialized) {
        ESP_LOGW(TAG, "mDNS already initialized");
        return ESP_OK;
    }
    
    // Initialize mDNS
    esp_err_t ret = mdns_init();
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to initialize mDNS: %s", esp_err_to_name(ret));
        return ret;
    }
    
    // Set mDNS hostname
    ret = mdns_hostname_set("iotcraft-gateway");
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to set mDNS hostname: %s", esp_err_to_name(ret));
        mdns_free();
        return ret;
    }
    
    // Set mDNS instance name
    ret = mdns_instance_name_set("IoTCraft Gateway");
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to set mDNS instance name: %s", esp_err_to_name(ret));
        mdns_free();
        return ret;
    }
    
    // Add MQTT broker service
    ret = mdns_service_add("MQTT Broker", "_mqtt", "_tcp", 1883, NULL, 0);
    if (ret != ESP_OK) {
        ESP_LOGW(TAG, "Failed to add MQTT service to mDNS: %s", esp_err_to_name(ret));
    } else {
        ESP_LOGI(TAG, "Added MQTT broker service to mDNS (_mqtt._tcp.local:1883)");
    }
    
    // Add HTTP configuration service  
    ret = mdns_service_add("Configuration Server", "_http", "_tcp", 80, NULL, 0);
    if (ret != ESP_OK) {
        ESP_LOGW(TAG, "Failed to add HTTP service to mDNS: %s", esp_err_to_name(ret));
    } else {
        ESP_LOGI(TAG, "Added HTTP configuration service to mDNS (_http._tcp.local:80)");
    }
    
    // Add service instance for IoTCraft specifically
    mdns_txt_item_t iotcraft_txt[] = {
        {"service", "iotcraft-gateway"},
        {"version", "1.0.0"},
        {"features", "dhcp,nat,mqtt,http,display"}
    };
    
    ret = mdns_service_add("IoTCraft Gateway", "_iotcraft", "_tcp", 1883, iotcraft_txt, 3);
    if (ret != ESP_OK) {
        ESP_LOGW(TAG, "Failed to add IoTCraft service to mDNS: %s", esp_err_to_name(ret));
    } else {
        ESP_LOGI(TAG, "Added IoTCraft gateway service to mDNS (_iotcraft._tcp.local:1883)");
    }
    
    mdns_initialized = true;
    ESP_LOGI(TAG, "mDNS service initialized successfully");
    ESP_LOGI(TAG, "Gateway accessible as: iotcraft-gateway.local");
    ESP_LOGI(TAG, "MQTT broker accessible as: iotcraft-gateway.local:1883");
    
    return ESP_OK;
}

esp_err_t iotcraft_mdns_stop(void)
{
    if (!mdns_initialized) {
        return ESP_OK;
    }
    
    mdns_free();
    mdns_initialized = false;
    ESP_LOGI(TAG, "mDNS service stopped");
    
    return ESP_OK;
}

bool iotcraft_mdns_is_running(void)
{
    return mdns_initialized;
}
