/**
 * @file brightness_mock.c
 * @brief Mock brightness control for desktop builds
 */

#include <stdint.h>
#include <stdbool.h>

// Mock brightness functions for desktop (no actual hardware)
bool brightness_init(void) {
    return true;
}

bool brightness_set(uint8_t percent) {
    (void)percent;
    return true;
}

uint8_t brightness_increase(uint8_t step) {
    (void)step;
    return 100;
}

uint8_t brightness_decrease(uint8_t step) {
    (void)step;
    return 0;
}

uint8_t brightness_get(void) {
    return 100;
}
