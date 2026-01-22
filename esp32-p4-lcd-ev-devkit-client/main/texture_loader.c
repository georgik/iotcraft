/**
 * @file texture_loader.c
 * @brief Texture loading from SD card or embedded fallback
 */

#include "texture_loader.h"
#include "sdcard_init.h"

#ifdef __DESKTOP_BUILD__
    #include <stdio.h>
    #define TAG "TextureLoader"
    // Ensure ESP_FAIL is defined (pp a_helper.h defines ESP_OK but not ESP_FAIL)
    #ifndef ESP_FAIL
        #define ESP_FAIL -1
    #endif
#else
    #include "esp_log.h"
    #define TAG "TextureLoader"
#endif

#include <string.h>

// External references to embedded textures (from block_textures.c)
extern const uint16_t texture_grass[64];
extern const uint16_t texture_dirt[64];
extern const uint16_t texture_stone[64];
extern const uint16_t texture_quartz[64];
extern const uint16_t texture_glass[64];
extern const uint16_t texture_terracotta[64];
extern const uint16_t texture_water[64];

// Texture storage (dynamic for SD card, embedded as fallback)
#define TEXTURE_SIZE 16  // 16x16 pixels
#define TEXTURE_PIXELS (TEXTURE_SIZE * TEXTURE_SIZE)  // 256 pixels
#define TEXTURE_BYTES (TEXTURE_PIXELS * 2)  // 512 bytes (RGB565)

static uint16_t g_textures[BLOCK_COUNT][TEXTURE_PIXELS];
static bool g_using_sdcard = false;

// Forward declaration
static esp_err_t texture_load_all_from_sdcard(void);

// Texture filenames (must match block_type_t order)
static const char* texture_filenames[BLOCK_COUNT] = {
    NULL,                   // BLOCK_AIR (no texture needed)
    "grass.rgb565",          // BLOCK_GRASS
    "dirt.rgb565",           // BLOCK_DIRT
    "stone.rgb565",          // BLOCK_STONE
    "quartz_block.rgb565",   // BLOCK_QUARTZ
    "glass_pane.rgb565",     // BLOCK_GLASS
    "cyan_terracotta.rgb565",// BLOCK_TERRACOTTA
    "water.rgb565",          // BLOCK_WATER
};

esp_err_t texture_loader_init(void) {
    // Initialize SD card (OK if not present)
    sdcard_init();

    if (sdcard_is_available()) {
        ESP_LOGI(TAG, "Loading textures from SD card...");
        return texture_load_all_from_sdcard();
    } else {
        ESP_LOGI(TAG, "SD card not available, using embedded textures");
        texture_loader_use_embedded();
        return ESP_OK;
    }
}

static esp_err_t texture_load_all_from_sdcard(void) {
    const char* mount_point = sdcard_get_mount_point();
    if (!mount_point) {
        texture_loader_use_embedded();
        return ESP_FAIL;
    }

    ESP_LOGI(TAG, "Reading textures from %s/iotcraft/textures/", mount_point);

    for (int i = 0; i < BLOCK_COUNT; i++) {
        // Skip BLOCK_AIR (no texture needed)
        if (i == BLOCK_AIR || texture_filenames[i] == NULL) {
            // Initialize AIR with transparent color (black)
            memset(g_textures[i], 0, TEXTURE_BYTES);
            continue;
        }

        char path[256];
        snprintf(path, sizeof(path), "%s/iotcraft/textures/%s",
                 mount_point, texture_filenames[i]);

        FILE* f = fopen(path, "rb");
        if (!f) {
            ESP_LOGW(TAG, "Failed to open %s, using embedded textures", path);
            texture_loader_use_embedded();
            g_using_sdcard = false;
            return ESP_FAIL;
        }

        // Read RGB565 data (512 bytes for 16x16 texture)
        size_t read = fread(g_textures[i], sizeof(uint16_t), TEXTURE_PIXELS, f);
        fclose(f);

        if (read != TEXTURE_PIXELS) {
            ESP_LOGW(TAG, "Invalid texture size in %s (expected %d, got %zu)",
                     path, TEXTURE_PIXELS, read);
            texture_loader_use_embedded();
            g_using_sdcard = false;
            return ESP_FAIL;
        }

        ESP_LOGI(TAG, "  ✓ Loaded %s", texture_filenames[i]);
    }

    g_using_sdcard = true;
    ESP_LOGI(TAG, "✓ Successfully loaded %d textures from SD card", BLOCK_COUNT);
    return ESP_OK;
}

void texture_loader_use_embedded(void) {
    ESP_LOGI(TAG, "Using embedded 8x8 textures");

    // Copy embedded textures to g_textures array
    // Note: Embedded textures are 8x8, we're upscaling to 16x16

    // For each 8x8 embedded texture, upscale to 16x16
    for (int ty = 0; ty < TEXTURE_SIZE; ty++) {
        for (int tx = 0; tx < TEXTURE_SIZE; tx++) {
            // Map 16x16 coordinate to 8x8 source
            int src_ty = ty / 2;
            int src_tx = tx / 2;

            // Copy pixel (simple nearest-neighbor upscale)
            for (int i = 0; i < BLOCK_COUNT; i++) {
                uint16_t src_pixel;

                switch (i) {
                    case BLOCK_GRASS:
                        src_pixel = texture_grass[src_ty * 8 + src_tx];
                        break;
                    case BLOCK_DIRT:
                        src_pixel = texture_dirt[src_ty * 8 + src_tx];
                        break;
                    case BLOCK_STONE:
                        src_pixel = texture_stone[src_ty * 8 + src_tx];
                        break;
                    case BLOCK_QUARTZ:
                        src_pixel = texture_quartz[src_ty * 8 + src_tx];
                        break;
                    case BLOCK_GLASS:
                        src_pixel = texture_glass[src_ty * 8 + src_tx];
                        break;
                    case BLOCK_TERRACOTTA:
                        src_pixel = texture_terracotta[src_ty * 8 + src_tx];
                        break;
                    case BLOCK_WATER:
                        src_pixel = texture_water[src_ty * 8 + src_tx];
                        break;
                    default:
                        src_pixel = texture_stone[src_ty * 8 + src_tx];
                        break;
                }

                g_textures[i][ty * TEXTURE_SIZE + tx] = src_pixel;
            }
        }
    }

    g_using_sdcard = false;
}

bool texture_loader_using_sdcard(void) {
    return g_using_sdcard;
}

const uint16_t* texture_get(block_type_t type) {
    if (type < 0 || type >= BLOCK_COUNT) {
        return g_textures[BLOCK_STONE];  // Fallback to stone
    }
    return g_textures[type];
}

int texture_get_size(void) {
    return TEXTURE_SIZE;
}
