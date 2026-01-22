/**
 * @file FreeRTOS.h
 * @brief Mock FreeRTOS header for desktop simulator
 */

#ifndef FREERTOS_MOCK_FREERTOS_H
#define FREERTOS_MOCK_FREERTOS_H

#include <stdint.h>
#include <stdbool.h>
#include <unistd.h>
#include <time.h>

// Tick type
typedef uint32_t TickType_t;
#define pdMS_TO_TICKS(ms) (ms)

// Mock task functions
static inline void vTaskDelay(TickType_t ticks) {
    usleep(ticks * 1000);
}

static inline TickType_t xTaskGetTickCount(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (TickType_t)(ts.tv_sec * 1000 + ts.tv_nsec / 1000000);
}

// Stub out other FreeRTOS constructs
#define xTaskCreate(task, name, stack, param, prio, handle) do {} while(0)
#define xTaskCreatePinnedToCore(task, name, stack, param, prio, handle, core) do {} while(0)
#define vTaskDelete(handle) do {} while(0)

#endif // FREERTOS_MOCK_FREERTOS_H
