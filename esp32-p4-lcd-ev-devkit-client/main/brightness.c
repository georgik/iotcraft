/**
 * @file brightness.c
 * @brief Display brightness control using PWM
 */

#include "brightness.h"
#include "driver/ledc.h"
#include "esp_log.h"
#include "soc/gpio_struct.h"

static const char* TAG = "Brightness";

// Hardware configuration
#define BRIGHTNESS_PWM_GPIO      26  // GPIO26 for LCD backlight PWM
#define BRIGHTNESS_PWM_LEDC_MODE LEDC_LOW_SPEED_MODE
#define BRIGHTNESS_PWM_LEDC_CHANNEL LEDC_CHANNEL_0
#define BRIGHTNESS_PWM_LEDC_TIMER LEDC_TIMER_0
#define BRIGHTNESS_PWM_FREQ_HZ   5000  // 5 kHz PWM frequency
#define BRIGHTNESS_PWM_RES_BITS  10    // 10-bit resolution (0-1023)

// State
static uint8_t g_current_brightness = 100;  // Default: 100% brightness
static bool g_initialized = false;

bool brightness_init(void) {
    if (g_initialized) {
        ESP_LOGW(TAG, "Brightness control already initialized");
        return true;
    }

    ESP_LOGI(TAG, "Initializing brightness control on GPIO%d...", BRIGHTNESS_PWM_GPIO);

    // Prepare LEDC timer configuration
    ledc_timer_config_t ledc_timer = {
        .speed_mode = BRIGHTNESS_PWM_LEDC_MODE,
        .duty_resolution = (ledc_timer_bit_t)BRIGHTNESS_PWM_RES_BITS,
        .timer_num = BRIGHTNESS_PWM_LEDC_TIMER,
        .freq_hz = BRIGHTNESS_PWM_FREQ_HZ,
        .clk_cfg = LEDC_AUTO_CLK,
    };

    esp_err_t ret = ledc_timer_config(&ledc_timer);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to configure LEDC timer: %s", esp_err_to_name(ret));
        return false;
    }

    // Prepare LEDC channel configuration
    ledc_channel_config_t ledc_channel = {
        .gpio_num = BRIGHTNESS_PWM_GPIO,
        .speed_mode = BRIGHTNESS_PWM_LEDC_MODE,
        .channel = BRIGHTNESS_PWM_LEDC_CHANNEL,
        .intr_type = LEDC_INTR_DISABLE,
        .timer_sel = BRIGHTNESS_PWM_LEDC_TIMER,
        .duty = 0,
        .hpoint = 0,
    };

    ret = ledc_channel_config(&ledc_channel);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to configure LEDC channel: %s", esp_err_to_name(ret));
        return false;
    }

    // Mark as initialized BEFORE setting brightness
    g_initialized = true;

    // Set initial brightness to 50%
    brightness_set(50);

    ESP_LOGI(TAG, "✓ Brightness control initialized (50%%)");
    return true;
}

bool brightness_set(uint8_t percent) {
    if (!g_initialized) {
        ESP_LOGE(TAG, "Brightness control not initialized");
        return false;
    }

    // Clamp to valid range
    if (percent > 100) {
        percent = 100;
    }

    // Calculate PWM duty cycle (0-1023 for 10-bit resolution)
    uint32_t duty = (percent * ((1 << BRIGHTNESS_PWM_RES_BITS) - 1)) / 100;

    // Update PWM duty
    esp_err_t ret = ledc_set_duty(BRIGHTNESS_PWM_LEDC_MODE,
                                   BRIGHTNESS_PWM_LEDC_CHANNEL,
                                   duty);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to set duty: %s", esp_err_to_name(ret));
        return false;
    }

    ret = ledc_update_duty(BRIGHTNESS_PWM_LEDC_MODE, BRIGHTNESS_PWM_LEDC_CHANNEL);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to update duty: %s", esp_err_to_name(ret));
        return false;
    }

    g_current_brightness = percent;
    return true;
}

uint8_t brightness_increase(uint8_t step) {
    uint8_t new_brightness = g_current_brightness + step;
    if (new_brightness > 100) {
        new_brightness = 100;
    }

    if (brightness_set(new_brightness)) {
        ESP_LOGI(TAG, "Brightness: %d%% (+%d)", new_brightness, step);
    }
    return g_current_brightness;
}

uint8_t brightness_decrease(uint8_t step) {
    if (g_current_brightness < step) {
        step = g_current_brightness;
    }
    uint8_t new_brightness = g_current_brightness - step;

    if (brightness_set(new_brightness)) {
        ESP_LOGI(TAG, "Brightness: %d%% (-%d)", new_brightness, step);
    }
    return g_current_brightness;
}

uint8_t brightness_get(void) {
    return g_current_brightness;
}
