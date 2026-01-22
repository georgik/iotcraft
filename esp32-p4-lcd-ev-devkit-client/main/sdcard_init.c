/**
 * @file sdcard_init.c
 * @brief SD card initialization for ESP32-P4
 */

#include "sdcard_init.h"

#ifdef __DESKTOP_BUILD__
    // Desktop simulator stub - SD card not available
    #include <stdio.h>

    #ifndef ESP_LOGI
        #define ESP_LOGI(tag, fmt, ...) printf("[SDCard] " fmt "\n", ##__VA_ARGS__)
        #define ESP_LOGW(tag, fmt, ...) printf("[SDCard] WARNING: " fmt "\n", ##__VA_ARGS__)
        #define ESP_OK 0
    #endif

    esp_err_t sdcard_init(void) {
        ESP_LOGI("SDCard", "No SD card in simulator (using embedded textures)");
        return ESP_OK;
    }

    bool sdcard_is_available(void) {
        return false;  // No SD card in simulator
    }

    const char* sdcard_get_mount_point(void) {
        return NULL;  // Not mounted
    }

    esp_err_t sdcard_deinit(void) {
        return ESP_OK;
    }

#else
    // ESP32-P4 specific implementation
    #include "esp_log.h"
    #include "esp_vfs_fat.h"
    #include "sdmmc_cmd.h"
    #include "driver/sdmmc_host.h"
    #include "esp_vfs.h"
    #include "sd_pwr_ctrl_by_on_chip_ldo.h"
    #include <string.h>

    #define TAG "SDCard"

    /**
     * ESP-Hosted SDMMC Host Management:
     *
     * ESP-Hosted calls sdmmc_host_init() during its initialization (port_esp_hosted_host_sdio.c:347),
     * which initializes the entire SDMMC peripheral (both slots).
     *
     * When we try to mount the SD card, esp_vfs_fat_sdmmc_mount() attempts to initialize
     * the host again, causing "no available sd host controller" error.
     *
     * Solution: Use dummy init/deinit functions since ESP-Hosted has already initialized
     * the SDMMC host. Each slot (0 and 1) can then be initialized independently.
     */
    static esp_err_t sdmmc_host_init_dummy(void)
    {
        // Host already initialized by ESP-Hosted
        return ESP_OK;
    }

    static esp_err_t sdmmc_host_deinit_dummy(void)
    {
        // Let ESP-Hosted handle deinitialization
        return ESP_OK;
    }

    #define MOUNT_POINT "/sdcard"
    #define MAX_FILES 5

    static bool g_sdcard_mounted = false;
    static sdmmc_card_t* g_card = NULL;
    static sd_pwr_ctrl_handle_t pwr_ctrl_handle = NULL;  // LDO power control handle

    esp_err_t sdcard_init(void) {
        esp_err_t ret;

        // Check if already mounted
        if (g_sdcard_mounted) {
            ESP_LOGW(TAG, "SD card already mounted");
            return ESP_OK;
        }

        // Mount configuration
        esp_vfs_fat_sdmmc_mount_config_t mount_config = {
            .format_if_mount_failed = false,
            .max_files = MAX_FILES,
            .allocation_unit_size = 16 * 1024
        };

        // Host configuration
        // ESP-Hosted has already initialized the SDMMC host, so we use dummy functions
        sdmmc_host_t host = SDMMC_HOST_DEFAULT();
        host.slot = SDMMC_HOST_SLOT_0;  // SD card on slot 0
        host.init = &sdmmc_host_init_dummy;  // Skip init (done by ESP-Hosted)
        host.deinit = &sdmmc_host_deinit_dummy;  // Skip deinit (managed by ESP-Hosted)

        // Configure LDO power control for SD card (required on ESP32-P4 Function EV board)
        // The SD card needs power from LDO channel 4
        sd_pwr_ctrl_ldo_config_t ldo_config = {
            .ldo_chan_id = 4,  // LDO channel 4 for SD card power
        };
        esp_err_t pwr_ret = sd_pwr_ctrl_new_on_chip_ldo(&ldo_config, &pwr_ctrl_handle);
        if (pwr_ret != ESP_OK) {
            ESP_LOGE(TAG, "Failed to create LDO power control: %s", esp_err_to_name(pwr_ret));
            // Continue anyway - might work without explicit power control
        } else {
            host.pwr_ctrl_handle = pwr_ctrl_handle;
            ESP_LOGI(TAG, "LDO power control initialized (channel 4)");
        }

        // Slot configuration for SD card on ESP32-P4 Function EV board
        // IMPORTANT: Slot 0 uses IO MUX (hardwired pins), NOT GPIO matrix!
        // Do NOT specify GPIO pins - they are routed internally by the chip.
        // Source: BSP comment "Slot 0 uses IO MUX, so not specifying the pins here"
        sdmmc_slot_config_t slot_config = SDMMC_SLOT_CONFIG_DEFAULT();

        // Configure 4-bit SDIO mode for better performance
        slot_config.width = 4;

        ESP_LOGI(TAG, "Attempting to mount SD card on SDMMC slot 0 (IO MUX mode, host managed by ESP-Hosted)...");

        // Mount the card
        sdmmc_card_t* card;
        ret = esp_vfs_fat_sdmmc_mount(MOUNT_POINT, &host, &slot_config,
                                       &mount_config, &card);

        if (ret == ESP_OK) {
            g_sdcard_mounted = true;
            ESP_LOGI(TAG, "✓ SD card mounted successfully at %s", MOUNT_POINT);

            // Print card info
            sdmmc_card_print_info(stdout, card);
            return ESP_OK;
        } else if (ret == ESP_ERR_NOT_FOUND) {
            // No SD card inserted - this is OK
            ESP_LOGW(TAG, "No SD card detected (using embedded textures)");
            g_sdcard_mounted = false;
            return ESP_OK;  // Not an error - we have fallback
        } else {
            // Other error (corrupt card, wrong format, etc.)
            ESP_LOGW(TAG, "SD card mount failed: %s (using embedded textures)",
                     esp_err_to_name(ret));
            g_sdcard_mounted = false;
            return ESP_OK;  // Not an error - we have fallback
        }
    }

    bool sdcard_is_available(void) {
        return g_sdcard_mounted;
    }

    const char* sdcard_get_mount_point(void) {
        if (g_sdcard_mounted) {
            return MOUNT_POINT;
        }
        return NULL;
    }

    esp_err_t sdcard_deinit(void) {
        if (!g_sdcard_mounted) {
            return ESP_OK;
        }

        esp_err_t ret = esp_vfs_fat_sdcard_unmount(MOUNT_POINT, NULL);
        if (ret == ESP_OK) {
            g_sdcard_mounted = false;
            ESP_LOGI(TAG, "SD card unmounted");
        } else {
            ESP_LOGW(TAG, "Failed to unmount SD card: %s", esp_err_to_name(ret));
        }

        return ret;
    }

#endif
