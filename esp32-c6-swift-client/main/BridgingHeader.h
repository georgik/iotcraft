//===----------------------------------------------------------------------===//
// BridgingHeader.h for ESP32-C6 Swift IoTCraft Client
// Provides access to ESP-IDF functions and our C wrapper functions from Swift
//===----------------------------------------------------------------------===//

#include <stdio.h>

// FreeRTOS includes
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

// ESP-IDF includes
#include "esp_log.h"
#include "led_strip.h"
#include "sdkconfig.h"

// Our ESP Swift wrapper functions
#include "esp_swift_wrapper.h"
