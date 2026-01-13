/**
 * @file board_init.c
 * @brief Board-specific initialization for raylib example
 *
 * This file shows how to initialize display panels using either:
 * - Path A: ESP-BSP (noglib versions) for supported boards
 * - Path B: Raw esp_lcd for custom hardware
 */

#include "board_init.h"
#include "esp_log.h"
#include "esp_err.h"
#include "esp_raylib_port.h"
#include "esp_lcd_panel_io.h"

static const char *TAG = "board_init";

// Store panel handle for framebuffer push (P4 uses panel directly, not IO)
static esp_lcd_panel_handle_t g_panel_handle = NULL;

// Example configuration for ESP-BOX-3 using BSP
#ifdef CONFIG_BOARD_ESP_BOX_3

#include "bsp/esp-bsp.h"

esp_err_t board_init_display(void)
{
    ESP_LOGI(TAG, "Initializing ESP-BOX-3 display via BSP...");
    
    // Initialize BSP display (noglib version - no LVGL)
    esp_lcd_panel_handle_t panel_handle = NULL;
    esp_lcd_panel_io_handle_t io_handle = NULL;
    
    bsp_display_config_t cfg = {
        .max_transfer_sz = 320 * 48 * sizeof(uint16_t),  // 48 lines per chunk
    };
    
    bsp_display_new(&cfg, &panel_handle, &io_handle);
    
    if (!panel_handle) {
        ESP_LOGE(TAG, "Failed to initialize BSP display");
        return ESP_FAIL;
    }
    
    // Turn on backlight
    bsp_display_backlight_on();
    
    // Initialize raylib port layer
    ray_port_cfg_t port_cfg = {
        .buff_psram = true,
        .double_buffer = false,
        .buffer_pixels = 0,
        .chunk_bytes = 0,  // Auto
        .swap_rgb_bytes = true,  // SPI LCDs need byte swap
        .hres = 320,
        .vres = 240,
        .rotation = 0,
        .perf_stats = true,
    };
    
    ESP_ERROR_CHECK(ray_port_init(&port_cfg));
    
    // Register display with port
    ray_port_display_cfg_t disp_cfg = {
        .panel = panel_handle,
        .io = io_handle,
        .hres = 320,
        .vres = 240,
        .monochrome = false,
        .dma_capable = true,
    };
    
    ESP_ERROR_CHECK(ray_port_add_display(&disp_cfg));
    
    ESP_LOGI(TAG, "Display initialized: 320x240");
    return ESP_OK;
}

#elif CONFIG_BOARD_M5STACK_CORE_S3

#include "bsp/esp-bsp.h"

esp_err_t board_init_display(void)
{
    ESP_LOGI(TAG, "Initializing M5Stack Core S3 display via BSP...");
    
    // Initialize BSP display (noglib version)
    esp_lcd_panel_handle_t panel_handle = NULL;
    esp_lcd_panel_io_handle_t io_handle = NULL;
    
    bsp_display_config_t cfg = {
        .max_transfer_sz = 320 * 48 * sizeof(uint16_t),
    };
    
    bsp_display_new(&cfg, &panel_handle, &io_handle);
    
    if (!panel_handle) {
        ESP_LOGE(TAG, "Failed to initialize BSP display");
        return ESP_FAIL;
    }
    
    bsp_display_backlight_on();
    
    // Initialize raylib port layer
    ray_port_cfg_t port_cfg = {
        .buff_psram = true,
        .double_buffer = false,
        .buffer_pixels = 0,
        .chunk_bytes = 0,
        .swap_rgb_bytes = true,
        .hres = 320,
        .vres = 240,
        .rotation = 0,
        .perf_stats = true,
    };
    
    ESP_ERROR_CHECK(ray_port_init(&port_cfg));
    
    ray_port_display_cfg_t disp_cfg = {
        .panel = panel_handle,
        .io = io_handle,
        .hres = 320,
        .vres = 240,
        .monochrome = false,
        .dma_capable = true,
    };
    
    ESP_ERROR_CHECK(ray_port_add_display(&disp_cfg));
    
    ESP_LOGI(TAG, "Display initialized: 320x240");
    return ESP_OK;
}

#elif CONFIG_BOARD_ESP32_P4_FUNCTION_EV

#include "bsp/esp-bsp.h"

esp_err_t board_init_display(void)
{
    ESP_LOGI(TAG, "Initializing ESP32-P4 Function EV Board display via BSP...");
    
    // Initialize BSP display (noglib version - DSI interface)
    esp_lcd_panel_handle_t panel_handle = NULL;
    esp_lcd_panel_io_handle_t io_handle = NULL;
    
    // P4 requires proper DSI bus configuration
    const bsp_display_config_t bsp_disp_cfg = {
        .hdmi_resolution = 0,  // BSP_HDMI_RES_NONE - Use LCD, not HDMI
        .dsi_bus = {
            .phy_clk_src = 0,  // MIPI_DSI_PHY_CLK_SRC_DEFAULT
            .lane_bit_rate_mbps = 1000,  // BSP_LCD_MIPI_DSI_LANE_BITRATE_MBPS default
        },
    };
    
    esp_err_t ret = bsp_display_new(&bsp_disp_cfg, &panel_handle, &io_handle);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to initialize BSP display: %d", ret);
        return ret;
    }

    // Store panel handle for framebuffer push (P4 uses panel, not IO)
    g_panel_handle = panel_handle;
    
    if (!panel_handle) {
        ESP_LOGE(TAG, "Failed to initialize BSP display");
        return ESP_FAIL;
    }
    
    // Initialize backlight control
    ret = bsp_display_brightness_init();
    if (ret == ESP_OK) {
        ret = bsp_display_backlight_on();
        if (ret != ESP_OK) {
            ESP_LOGW(TAG, "Failed to turn on backlight: %d", ret);
            // Don't fail initialization if backlight fails
        }
    } else {
        ESP_LOGW(TAG, "Backlight initialization failed: %d", ret);
        // Don't fail initialization if backlight init fails
    }
    
    // Get display dimensions from BSP
    // P4 typically has 1024x600 or 1280x800
    uint16_t width = 1024;  // Default, adjust based on CONFIG
    uint16_t height = 600;
    
    #ifdef CONFIG_BSP_LCD_TYPE_1280_800
    width = 1280;
    height = 800;
    #endif
    
    // Initialize raylib port layer
    ray_port_cfg_t port_cfg = {
        .buff_psram = true,
        .double_buffer = false,
        .buffer_pixels = 0,
        .chunk_bytes = 0,  // DSI doesn't need chunking
        .swap_rgb_bytes = false,  // DSI panels don't need byte swap
        .hres = width,
        .vres = height,
        .rotation = 0,
        .perf_stats = true,
    };
    
    ESP_ERROR_CHECK(ray_port_init(&port_cfg));
    
    ray_port_display_cfg_t disp_cfg = {
        .panel = panel_handle,
        .io = io_handle,
        .hres = width,
        .vres = height,
        .monochrome = false,
        .dma_capable = true,
    };
    
    ESP_ERROR_CHECK(ray_port_add_display(&disp_cfg));
    
    ESP_LOGI(TAG, "Display initialized: %dx%d", width, height);
    return ESP_OK;
}

esp_err_t board_display_push_frame(const uint16_t* framebuffer, int width, int height)
{
    if (!g_panel_handle) {
        // Only log this error occasionally (simulator may not have display HW)
        static int error_count = 0;
        if (++error_count % 100 == 1) {
            ESP_LOGE(TAG, "Panel handle not initialized (simulator may not have display HW)");
        }
        return ESP_ERR_INVALID_STATE;
    }

    if (!framebuffer || width <= 0 || height <= 0) {
        ESP_LOGE(TAG, "Invalid framebuffer parameters");
        return ESP_ERR_INVALID_ARG;
    }

    // Use esp_lcd_panel_draw_bitmap for MIPI DSI displays (P4)
    // This is the correct API for DSI panels, not esp_lcd_panel_io_tx_color
    esp_err_t ret = esp_lcd_panel_draw_bitmap(g_panel_handle, 0, 0, width, height, (void*)framebuffer);
    if (ret != ESP_OK) {
        // Only log occasionally to avoid spam (simulator may not have display)
        static int fail_count = 0;
        if (++fail_count % 100 == 1) {
            ESP_LOGW(TAG, "Failed to draw bitmap: %d (simulator may not have display)", ret);
        }
        return ret;
    }

    return ESP_OK;
}

#else

// Fallback: User must provide custom initialization
esp_err_t board_init_display(void)
{
    ESP_LOGE(TAG, "No board selected! Please:");
    ESP_LOGE(TAG, "1. Set CONFIG_BOARD_ESP_BOX_3 or CONFIG_BOARD_M5STACK_CORE_S3 in sdkconfig");
    ESP_LOGE(TAG, "2. Or implement custom esp_lcd panel creation here");
    ESP_LOGE(TAG, "");
    ESP_LOGE(TAG, "Example for custom hardware:");
    ESP_LOGE(TAG, "  - Create SPI bus and esp_lcd_panel_handle_t");
    ESP_LOGE(TAG, "  - Call ray_port_init() with your config");
    ESP_LOGE(TAG, "  - Call ray_port_add_display() with panel handle");
    return ESP_FAIL;
}

#endif
