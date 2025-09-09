#pragma once

#include <stdbool.h>
#include "esp_err.h"

#ifdef __cplusplus
extern "C" {
#endif

// Service initialization functions
esp_err_t iotcraft_dhcp_init(void);
esp_err_t iotcraft_mqtt_broker_init(void);
esp_err_t iotcraft_mqtt_broker_stop(void);
bool iotcraft_mqtt_is_running(void);
esp_err_t iotcraft_mdns_init(void);
esp_err_t iotcraft_http_server_init(void);
esp_err_t iotcraft_status_gui_init(void);

// Service control functions  
esp_err_t iotcraft_start_all_services(void);
esp_err_t iotcraft_stop_all_services(void);

// Status and monitoring
typedef struct {
    bool dhcp_running;
    bool mqtt_running;
    bool mdns_running;
    bool http_running;
    bool gui_running;
    int connected_clients;
    int mqtt_connections;
} iotcraft_status_t;

esp_err_t iotcraft_get_status(iotcraft_status_t *status);

// WiFi configuration getter
typedef struct {
    char ssid[32];
    char password[64];
} iotcraft_wifi_config_t;

esp_err_t iotcraft_get_wifi_config(iotcraft_wifi_config_t *config);

// Status GUI functions
esp_err_t iotcraft_status_gui_init(void);
esp_err_t iotcraft_status_gui_stop(void);
bool iotcraft_status_gui_is_running(void);
esp_err_t iotcraft_status_gui_update_status(const iotcraft_status_t *status);


#ifdef __cplusplus
}
#endif
