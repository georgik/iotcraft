/**
 * @file freertos_mock.h
 * @brief Mock FreeRTOS functions for desktop simulator
 */

#ifndef FREERTOS_MOCK_H
#define FREERTOS_MOCK_H

#include <stdint.h>
#include <stdbool.h>
#include <unistd.h>
#include <pthread.h>
#include <time.h>

// Tick type
typedef uint32_t TickType_t;
#define pdMS_TO_TICKS(ms) (ms)

// Task functions (mocked)
#define xTaskCreatePinnedToCore(task, name, stack, param, prio, handle, core) \
    do { \
        pthread_t thread; \
        pthread_create(&thread, NULL, (void*(*)(void*))task, param); \
    } while(0)

static inline void vTaskDelay(TickType_t ticks) {
    usleep(ticks * 1000);
}

static inline TickType_t xTaskGetTickCount(void) {
    // Return milliseconds since start
    struct timespec ts;
    clock_gettime(CLOCK_REALTIME, &ts);
    return (TickType_t)(ts.tv_sec * 1000 + ts.tv_nsec / 1000000);
}

// Mock other FreeRTOS constructs
#define xTaskCreate(task, name, stack, param, prio, handle) \
    xTaskCreatePinnedToCore(task, name, stack, param, prio, handle, 0)

static inline void vTaskDelete(void* handle) {
    pthread_exit(NULL);
}

#endif // FREERTOS_MOCK_H
