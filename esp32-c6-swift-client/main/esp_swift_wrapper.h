#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// System initialization
int nvs_init(void);

// WiFi functions
int wifi_init(void);
int wifi_connect(const char* ssid, const char* password);

// MQTT functions
int mqtt_client_init(void);
int mqtt_client_start(void);
int mqtt_publish(const char* topic, const char* payload);
int mqtt_publish_device_announcement(void);

// Device utilities
void generate_device_id(char* buffer, int buffer_size);
void delay_ms(unsigned int ms);
unsigned long get_millis(void);

// LED control callback registration
void register_led_control_callback(void (*callback)(bool is_on));

// Swift main function (to be implemented in Swift)
void swift_main(void);

#ifdef __cplusplus
}
#endif