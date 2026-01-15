/**
 * @file renderer.c
 * @brief Raycasting renderer implementation
 */

#include "renderer.h"
#include "world.h"
#include "trig_lut.h"
#include "fixed_point.h"
#include "ppa_helper.h"
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <math.h>
#include <esp_log.h>

static const char* TAG = "Renderer";

// Fixed-point optimization: Use 16.16 format for faster calculations
#define USE_FIXED_POINT 1

// PPA hardware upscaling: Render at lower resolution and scale up
#define USE_PPA_UPSCALING 1
#define PPA_SCALE_DIV 2  // Render at half resolution (160x120 -> 320x240)

// Wireframe mode (toggle with F1)
static bool g_wireframe_mode = false;

// Forward declarations
static void draw_voxel_3d(renderer_t* renderer, int32_t x, int32_t y, int32_t z, block_type_t block);

// RGB565 color helpers
#define RGB565(r,g,b) ((((r)&0xF8)<<8)|(((g)&0xFC)<<3)|((b)>>3))

// Wireframe colors
#define COLOR_WIREFRAME_EDGE 0xFFFF  // White
#define COLOR_WIREFRAME_FACE  0x4208  // Dark blue (transparent look)

/**
 * @brief Shade an RGB565 color by a factor
 * @param color RGB565 color
 * @param factor Shading factor (0.0 to 1.0, where 1.0 is unchanged)
 * @return Shaded RGB565 color
 */
static uint16_t shade_color(uint16_t color, float factor) {
    // Extract RGB components
    uint8_t r5 = (color >> 11) & 0x1F;
    uint8_t g6 = (color >> 5) & 0x3F;
    uint8_t b5 = color & 0x1F;

    // Apply shading
    r5 = (uint8_t)(r5 * factor);
    g6 = (uint8_t)(g6 * factor);
    b5 = (uint8_t)(b5 * factor);

    // Clamp and recombine
    if (r5 > 0x1F) r5 = 0x1F;
    if (g6 > 0x3F) g6 = 0x3F;
    if (b5 > 0x1F) b5 = 0x1F;

    return (r5 << 11) | (g6 << 5) | b5;
}

/**
 * @brief Blend two RGB565 colors
 * @param color1 First color
 * @param color2 Second color
 * @param factor Blend factor (0.0 = color1, 1.0 = color2)
 * @return Blended RGB565 color
 */
static uint16_t blend_colors(uint16_t color1, uint16_t color2, float factor) {
    uint8_t r1 = (color1 >> 11) & 0x1F;
    uint8_t g1 = (color1 >> 5) & 0x3F;
    uint8_t b1 = color1 & 0x1F;

    uint8_t r2 = (color2 >> 11) & 0x1F;
    uint8_t g2 = (color2 >> 5) & 0x3F;
    uint8_t b2 = color2 & 0x1F;

    uint8_t r5 = (uint8_t)(r1 + (r2 - r1) * factor);
    uint8_t g6 = (uint8_t)(g1 + (g2 - g1) * factor);
    uint8_t b5 = (uint8_t)(b1 + (b2 - b1) * factor);

    return (r5 << 11) | (g6 << 5) | b5;
}

// Texture data (8x8 RGB565) - scaled from desktop client textures
static const uint16_t texture_grass[64] = {
    0x23c5, 0x2c46, 0x2c65, 0x2c45, 0x3ca5, 0x3ca5, 0x3485, 0x23e5,
    0x3465, 0x3465, 0x2c45, 0x2c25, 0x3485, 0x2c45, 0x3485, 0x2c65,
    0x3485, 0x3ca5, 0x3485, 0x3465, 0x2c45, 0x2c65, 0x2c45, 0x2c45,
    0x3485, 0x2c45, 0x3485, 0x2c65, 0x2c45, 0x2c65, 0x2c45, 0x3465,
    0x3465, 0x3cc5, 0x2c45, 0x2405, 0x2c45, 0x2c45, 0x2c65, 0x3465,
    0x2c65, 0x2c45, 0x3465, 0x3465, 0x3485, 0x2c45, 0x2c45, 0x3465,
    0x2c45, 0x2c45, 0x2425, 0x2425, 0x3485, 0x2c45, 0x3485, 0x2c45,
    0x1bc5, 0x2c45, 0x2c65, 0x2c65, 0x2c45, 0x2c65, 0x2c65, 0x2405,
};

static const uint16_t texture_dirt[64] = {
    0x6a44, 0x6a44, 0x6204, 0x6204, 0x6a04, 0x7245, 0x7245, 0x6a04,
    0x6a44, 0x59c3, 0x59c3, 0x6a24, 0x6a45, 0x7a86, 0x7265, 0x7225,
    0x6a24, 0x6204, 0x6204, 0x6204, 0x6204, 0x6a24, 0x6a24, 0x6a04,
    0x6204, 0x6224, 0x6a04, 0x6204, 0x59c2, 0x6203, 0x6a04, 0x6a04,
    0x6a24, 0x7245, 0x7245, 0x7a86, 0x6a24, 0x6a24, 0x6204, 0x6204,
    0x7265, 0x7286, 0x6a24, 0x7265, 0x6a24, 0x6224, 0x59c3, 0x59c3,
    0x6a24, 0x61e3, 0x61c3, 0x6a04, 0x6224, 0x6224, 0x59e3, 0x59e3,
    0x6a24, 0x61e3, 0x61e3, 0x6a04, 0x6224, 0x6224, 0x6224, 0x6224,
};

static const uint16_t texture_stone[64] = {
    0x8c71, 0x8c71, 0x94b2, 0x7bef, 0x528a, 0x9cd3, 0x8c51, 0x8c71,
    0x8c71, 0x9cd3, 0x9cf3, 0x8430, 0x52aa, 0xa514, 0x9492, 0x94b2,
    0x94b2, 0x9cf3, 0x9cf3, 0x8430, 0x528a, 0xa514, 0x94b2, 0x9cd3,
    0x7bef, 0x8410, 0x8430, 0x738e, 0x4228, 0x8430, 0x7bef, 0x8410,
    0x52aa, 0x5acb, 0x5acb, 0x4a49, 0x3186, 0x52aa, 0x4a69, 0x528a,
    0x9cd3, 0xad55, 0xad55, 0x8c71, 0x52aa, 0xa534, 0x94b2, 0x9cd3,
    0x8c71, 0x94b2, 0x9cd3, 0x8410, 0x4a69, 0x94b2, 0x8c51, 0x8c71,
    0x9492, 0x9cd3, 0x9cf3, 0x8430, 0x4a69, 0x9cd3, 0x8c71, 0x8c71,
};

static const uint16_t texture_quartz[64] = {
    0x4207, 0x840f, 0x83ee, 0x6b4c, 0x4228, 0x840f, 0x7bae, 0x7bce,
    0x840f, 0xffff, 0xffdd, 0xd6b9, 0x8c2f, 0xffff, 0xf77c, 0xf79c,
    0x83ee, 0xffdd, 0xef7c, 0xce57, 0x840f, 0xffdd, 0xef3b, 0xef5c,
    0x6b4c, 0xd6b9, 0xce58, 0xad54, 0x736d, 0xd699, 0xc617, 0xce37,
    0x4228, 0x8c2f, 0x840f, 0x736d, 0x4a48, 0x8c50, 0x83ee, 0x840f,
    0x840f, 0xffff, 0xffdd, 0xd699, 0x8c50, 0xfffe, 0xf79c, 0xf7bc,
    0x7bce, 0xf77c, 0xe71b, 0xc617, 0x83ee, 0xf79c, 0xdef9, 0xe71a,
    0x7bce, 0xf79d, 0xef3b, 0xc637, 0x840f, 0xf7bc, 0xe71a, 0xe73a,
};

static const uint16_t texture_glass[64] = {
    0x4228, 0x8c71, 0x7c10, 0x8430, 0x8430, 0x8430, 0x8430, 0x8430,
    0x8c71, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff,
    0x8410, 0xffff, 0xef9e, 0xf7be, 0xf7be, 0xf7be, 0xf7be, 0xf7be,
    0x8430, 0xffff, 0xf7be, 0xf7df, 0xf7df, 0xf7df, 0xf7df, 0xf7df,
    0x8430, 0xffff, 0xf7be, 0xf7df, 0xf7df, 0xf7ff, 0xf7ff, 0xf7ff,
    0x8430, 0xffff, 0xf7be, 0xf7df, 0xf7ff, 0xf7ff, 0xf7ff, 0xf7ff,
    0x8430, 0xffff, 0xf7be, 0xf7df, 0xf7ff, 0xf7ff, 0xf7ff, 0xf7ff,
    0x8430, 0xffff, 0xf7be, 0xf7df, 0xf7ff, 0xf7ff, 0xf7ff, 0xf7ff,
};

static const uint16_t texture_terracotta[64] = {
    0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef,
    0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef,
    0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef,
    0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef,
    0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef,
    0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef,
    0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef,
    0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef, 0x5bef,
};

static const uint16_t texture_water[64] = {
    0x5d5a, 0x5d5a, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b,
    0x5d5a, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b,
    0x5d5b, 0x5d5b, 0x5d5a, 0x5d5a, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b,
    0x5d5b, 0x5d5b, 0x5d5a, 0x5d5a, 0x5d5a, 0x5d5b, 0x5d5b, 0x5d5b,
    0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b,
    0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b,
    0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b,
    0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b, 0x5d5b,
};

// Texture lookup table
static const uint16_t* const textures[BLOCK_COUNT] = {
    NULL,                // BLOCK_AIR
    texture_grass,       // BLOCK_GRASS
    texture_dirt,        // BLOCK_DIRT
    texture_stone,       // BLOCK_STONE
    texture_quartz,      // BLOCK_QUARTZ
    texture_glass,       // BLOCK_GLASS
    texture_terracotta,  // BLOCK_TERRACOTTA
    texture_water,       // BLOCK_WATER
};

#define TEXTURE_SIZE 8

// Sky and ground colors
#define COLOR_SKY 0x867d    // Sky blue (from desktop client)
#define COLOR_GROUND 0x2444 // Grass green (from desktop client)

bool renderer_init(renderer_t* renderer, int32_t width, int32_t height,
                   camera_t* camera, voxel_world_t* world) {
    if (!renderer || !camera || !world) {
        ESP_LOGE(TAG, "Null parameters in renderer_init");
        return false;
    }

    renderer->width = width;
    renderer->height = height;
    renderer->camera = camera;
    renderer->world = world;
    renderer->ppa_scaler = NULL;

#if USE_PPA_UPSCALING
    // Initialize PPA scaler for hardware upscaling
    ppa_scaler_t* scaler = (ppa_scaler_t*)malloc(sizeof(ppa_scaler_t));
    if (scaler && ppa_scaler_init(scaler, width, height, PPA_SCALE_DIV)) {
        renderer->ppa_scaler = scaler;
        renderer->framebuffer = ppa_scaler_get_render_buffer(scaler);
        ESP_LOGI(TAG, "PPA scaler initialized: rendering at %dx%d, output %dx%d",
                 ppa_scaler_get_render_buffer(scaler) ? scaler->small_width : width,
                 ppa_scaler_get_render_buffer(scaler) ? scaler->small_height : height,
                 width, height);
    } else {
        ESP_LOGW(TAG, "PPA scaler init failed, falling back to software rendering");
        if (scaler) free(scaler);

        // Allocate framebuffer (RGB565 = 2 bytes per pixel)
        renderer->framebuffer = (uint16_t*)malloc(width * height * sizeof(uint16_t));
        if (!renderer->framebuffer) {
            ESP_LOGE(TAG, "Failed to allocate framebuffer");
            return false;
        }
    }
#else
    // Allocate framebuffer (RGB565 = 2 bytes per pixel)
    renderer->framebuffer = (uint16_t*)malloc(width * height * sizeof(uint16_t));
    if (!renderer->framebuffer) {
        ESP_LOGE(TAG, "Failed to allocate framebuffer");
        return false;
    }
#endif

    ESP_LOGI(TAG, "Renderer initialized: %dx%d", width, height);
    return true;
}

void renderer_free(renderer_t* renderer) {
    if (!renderer) {
        return;
    }

#if USE_PPA_UPSCALING
    if (renderer->ppa_scaler) {
        ppa_scaler_free((ppa_scaler_t*)renderer->ppa_scaler);
        free(renderer->ppa_scaler);
        renderer->ppa_scaler = NULL;
        renderer->framebuffer = NULL;
    } else
#endif
    if (renderer->framebuffer) {
        free(renderer->framebuffer);
        renderer->framebuffer = NULL;
    }

    renderer->width = 0;
    renderer->height = 0;
}

void renderer_clear(renderer_t* renderer, uint16_t color) {
    if (!renderer || !renderer->framebuffer) {
        return;
    }

    static int clear_count = 0;
    clear_count++;

    if (clear_count <= 5) {
        ESP_LOGI(TAG, "renderer_clear called #%d, color=0x%04x", clear_count, color);
    }

    // Fill framebuffer with solid color
    int32_t total_pixels = renderer->width * renderer->height;
    for (int32_t i = 0; i < total_pixels; i++) {
        renderer->framebuffer[i] = color;
    }
}

/**
 * @brief Draw a textured vertical column on the framebuffer
 * @param renderer Renderer context
 * @param x Screen X coordinate
 * @param y_start Top Y coordinate
 * @param y_end Bottom Y coordinate
 * @param tex_x Texture X coordinate (0-7)
 * @param texture Texture data (8x8 RGB565)
 * @param tex_y_start Texture Y start (fixed-point 12.4)
 * @param tex_y_step Texture Y step per pixel (fixed-point 12.4)
 */
static void draw_textured_column(renderer_t* renderer, int32_t x,
                                  int32_t y_start, int32_t y_end,
                                  int32_t tex_x, const uint16_t* texture,
                                  int32_t tex_y_start, int32_t tex_y_step,
                                  float shade_factor, uint16_t fog_color, float fog_factor) {
    // Clamp to screen bounds
    if (x < 0 || x >= renderer->width) {
        return;
    }
    if (y_start < 0) y_start = 0;
    if (y_end >= renderer->height) y_end = renderer->height - 1;

    // Draw textured vertical line
    int32_t tex_y = tex_y_start;

    // Debug: Log first few wall columns to verify X position
    static int tex_debug_count = 0;
    if (tex_debug_count < 5) {
        ESP_LOGI(TAG, "TEX_DEBUG #%d: x=%d, y_range=%d..%d, tex_x=%d", tex_debug_count, x, y_start, y_end, tex_x);
        ESP_LOGI(TAG, "  tex_y_start=%d (0x%x), tex_y_step=%d (0x%x)",
                 tex_y_start, tex_y_start, tex_y_step, tex_y_step);
        ESP_LOGI(TAG, "  Column height: %d pixels", y_end - y_start + 1);
        ESP_LOGI(TAG, "  Framebuffer index: [0]=%d, [mid]=%d, [last]=%d",
                 y_start * renderer->width + x,
                 ((y_start + y_end) / 2) * renderer->width + x,
                 y_end * renderer->width + x);
        tex_debug_count++;
    }

    for (int32_t y = y_start; y <= y_end; y++) {
        // Get texture coordinate (fixed-point 12.4, extract top 3 bits for 0-7)
        int32_t ty = (tex_y >> 4) & 0x7;

        // Sample texture
        uint16_t texel = texture[ty * TEXTURE_SIZE + tex_x];

        // Apply shading
        texel = shade_color(texel, shade_factor);

        // Apply fog
        if (fog_factor > 0.0f) {
            texel = blend_colors(texel, fog_color, fog_factor);
        }

        renderer->framebuffer[y * renderer->width + x] = texel;
        tex_y += tex_y_step;
    }
}

void renderer_render_columns(renderer_t* renderer, int32_t start_col, int32_t end_col) {
    if (!renderer || !renderer->framebuffer || !renderer->camera || !renderer->world) {
        return;
    }

    // Clamp column range to valid bounds
    if (start_col < 0) start_col = 0;
    if (end_col > renderer->width) end_col = renderer->width;
    if (start_col >= end_col) return;

    int32_t walls_drawn = 0;  // Debug counter

    // Raycast for specified column range
    for (int32_t x = start_col; x < end_col; x++) {
        // Calculate ray direction based on camera angle and screen position
        float camera_x = 2.0f * x / (float)renderer->width - 1.0f;  // -1 to +1
        float ray_angle = renderer->camera->yaw + camera_x * renderer->camera->fov * 0.5f;

        // Ray direction (use fast lookup tables)
        float ray_dir_x = cosf_fast(ray_angle) * cosf_fast(renderer->camera->pitch);
        float ray_dir_y = sinf_fast(renderer->camera->pitch);
        float ray_dir_z = sinf_fast(ray_angle) * cosf_fast(renderer->camera->pitch);

        // Current position in voxel grid (3D)
        int32_t map_x = (int32_t)floorf(renderer->camera->x);
        int32_t map_y = (int32_t)floorf(renderer->camera->y);
        int32_t map_z = (int32_t)floorf(renderer->camera->z);

        // Distance to next grid line
        float delta_dist_x = fabsf(1.0f / ray_dir_x);
        float delta_dist_y = fabsf(1.0f / ray_dir_y);
        float delta_dist_z = fabsf(1.0f / ray_dir_z);

        // Step direction and initial side distance
        int32_t step_x, step_y, step_z;
        float side_dist_x, side_dist_y, side_dist_z;

        if (ray_dir_x < 0) {
            step_x = -1;
            side_dist_x = (renderer->camera->x - map_x) * delta_dist_x;
        } else {
            step_x = 1;
            side_dist_x = (map_x + 1.0f - renderer->camera->x) * delta_dist_x;
        }

        if (ray_dir_y < 0) {
            step_y = -1;
            side_dist_y = (renderer->camera->y - map_y) * delta_dist_y;
        } else {
            step_y = 1;
            side_dist_y = (map_y + 1.0f - renderer->camera->y) * delta_dist_y;
        }

        if (ray_dir_z < 0) {
            step_z = -1;
            side_dist_z = (renderer->camera->z - map_z) * delta_dist_z;
        } else {
            step_z = 1;
            side_dist_z = (map_z + 1.0f - renderer->camera->z) * delta_dist_z;
        }

        // Perform 3D DDA and collect ALL visible faces (voxel-style)
        // Unlike Wolfenstein, we continue through all blocks to see everything
        typedef struct {
            int32_t x, y, z;
            int32_t side;  // 0=NS, 1=EW, 2=TB
            block_type_t type;
            float distance;
            float perp_dist;  // Perpendicular distance for rendering
        } visible_face_t;

        visible_face_t faces[64];  // Collect up to 64 faces per column
        int32_t face_count = 0;
        int32_t max_steps = 50;  // Prevent infinite loops
        int32_t steps = 0;

        while (steps < max_steps && face_count < 64) {
            // Check for block at current position
            block_type_t block_type = world_get_block(renderer->world, map_x, map_y, map_z);

            if (block_type != BLOCK_AIR) {
                // Block found! Determine which face is visible and calculate distance
                int32_t side = 0;
                float perp_wall_dist = 0.0f;

                // Determine which face we hit based on which side_dist was smallest
                // This tells us which grid plane we crossed to get here
                if (side_dist_x < side_dist_z) {
                    if (side_dist_x < side_dist_y) {
                        // Crossed X grid line → North/South face
                        side = 0;
                        perp_wall_dist = (map_x - renderer->camera->x + (1 - step_x) / 2.0f) / ray_dir_x;
                    } else {
                        // Crossed Y grid line → Top/Bottom face
                        side = 2;
                        perp_wall_dist = (map_y - renderer->camera->y + (1 - step_y) / 2.0f) / ray_dir_y;
                    }
                } else {
                    if (side_dist_z < side_dist_y) {
                        // Crossed Z grid line → East/West face
                        side = 1;
                        perp_wall_dist = (map_z - renderer->camera->z + (1 - step_z) / 2.0f) / ray_dir_z;
                    } else {
                        // Crossed Y grid line → Top/Bottom face
                        side = 2;
                        perp_wall_dist = (map_y - renderer->camera->y + (1 - step_y) / 2.0f) / ray_dir_y;
                    }
                }

                // Only add faces in front of camera
                if (perp_wall_dist > 0.1f) {
                    // Check for duplicate faces (same position and side)
                    bool is_duplicate = false;
                    for (int32_t f = 0; f < face_count; f++) {
                        if (faces[f].x == map_x && faces[f].y == map_y &&
                            faces[f].z == map_z && faces[f].side == side) {
                            is_duplicate = true;
                            break;
                        }
                    }

                    if (!is_duplicate) {
                        faces[face_count].x = map_x;
                        faces[face_count].y = map_y;
                        faces[face_count].z = map_z;
                        faces[face_count].side = side;
                        faces[face_count].type = block_type;
                        faces[face_count].distance = perp_wall_dist;  // Use perpendicular as distance
                        faces[face_count].perp_dist = perp_wall_dist;
                        face_count++;
                    }
                }
            }

            // Step to next grid position (DDA always steps, doesn't stop at blocks)
            if (side_dist_x < side_dist_z) {
                if (side_dist_x < side_dist_y) {
                    side_dist_x += delta_dist_x;
                    map_x += step_x;
                } else {
                    side_dist_y += delta_dist_y;
                    map_y += step_y;
                }
            } else {
                if (side_dist_z < side_dist_y) {
                    side_dist_z += delta_dist_z;
                    map_z += step_z;
                } else {
                    side_dist_y += delta_dist_y;
                    map_y += step_y;
                }
            }

            steps++;
        }

        // Sort faces by distance (far to near) for painter's algorithm
        // Use a small epsilon to prevent flickering when distances are nearly equal
        const float epsilon = 0.0001f;

        // Simple bubble sort - we have at most 64 faces, so this is fast enough
        for (int32_t i = 0; i < face_count - 1; i++) {
            for (int32_t j = 0; j < face_count - 1 - i; j++) {
                // Only swap if distances are significantly different
                // This prevents flickering when faces are at nearly the same distance
                float dist_diff = faces[j].distance - faces[j + 1].distance;

                if (dist_diff < -epsilon) {
                    // faces[j] is definitely closer than faces[j+1], swap
                    visible_face_t temp = faces[j];
                    faces[j] = faces[j + 1];
                    faces[j + 1] = temp;
                } else if (fabsf(dist_diff) <= epsilon) {
                    // Distances are nearly equal - use secondary sort criteria
                    // Prefer top faces (side=2) over side faces to prevent z-fighting
                    if (faces[j].side == 2 && faces[j + 1].side != 2) {
                        // Keep top face first (it should be on top)
                        visible_face_t temp = faces[j];
                        faces[j] = faces[j + 1];
                        faces[j + 1] = temp;
                    } else if (faces[j].side != 2 && faces[j + 1].side == 2) {
                        // Already in correct order (top face is at j+1, closer)
                        // Do nothing
                    } else {
                        // Same side type or both not top faces - use position as tiebreaker
                        // This ensures deterministic sorting
                        if (faces[j].y < faces[j + 1].y ||
                            (faces[j].y == faces[j + 1].y && faces[j].x < faces[j + 1].x) ||
                            (faces[j].y == faces[j + 1].y && faces[j].x == faces[j + 1].x && faces[j].z < faces[j + 1].z)) {
                            // Swap to get consistent ordering
                            visible_face_t temp = faces[j];
                            faces[j] = faces[j + 1];
                            faces[j + 1] = temp;
                        }
                    }
                }
                // else: faces[j] is definitely farther, no swap needed
            }
        }

        // Debug: Log face collection for first column
        static int collection_debug_count = 0;
        if (collection_debug_count < 3 && face_count > 0) {
            ESP_LOGI(TAG, "COLLECTION_DEBUG #%d: x=%d, collected %d faces", collection_debug_count, x, face_count);
            for (int32_t f = 0; f < face_count && f < 5; f++) {
                ESP_LOGI(TAG, "  Face %d: pos=(%d,%d,%d), side=%d, dist=%.2f",
                         f, faces[f].x, faces[f].y, faces[f].z, faces[f].side, faces[f].distance);
            }
            collection_debug_count++;
        }

        // Render each collected face (far to near)
        for (int32_t f = 0; f < face_count; f++) {
            visible_face_t* face = &faces[f];
            int32_t side = face->side;
            block_type_t block_type = face->type;
            float perp_wall_dist = face->perp_dist;

            // Skip faces behind camera
            if (perp_wall_dist <= 0.0f) {
                continue;
            }

            // Calculate fog
            float fog_factor = 0.0f;
            if (perp_wall_dist > 10.0f) {
                fog_factor = (perp_wall_dist - 10.0f) / 20.0f;
                if (fog_factor > 1.0f) fog_factor = 1.0f;
            }

            // Render based on face type
            if (side == 2) {
                // Horizontal face (top or bottom)
            } else {
                // Vertical face (North/South/East/West)
                float wall_x;
                if (side == 0) {
                    wall_x = renderer->camera->z + perp_wall_dist * ray_dir_z;
                } else {
                    wall_x = renderer->camera->x + perp_wall_dist * ray_dir_x;
                }

                // Calculate height of line to draw
                int32_t line_height = (int32_t)(renderer->height / perp_wall_dist);

                // Calculate draw start and end positions
                int32_t vdraw_start = -line_height / 2 + renderer->height / 2;
                if (vdraw_start < 0) vdraw_start = 0;
                int32_t vdraw_end = line_height / 2 + renderer->height / 2;
                if (vdraw_end >= renderer->height) vdraw_end = renderer->height - 1;

                int32_t column_height = vdraw_end - vdraw_start + 1;

                // Get texture
                const uint16_t* texture = textures[block_type];
                if (!texture) continue;

                // Calculate texture X coordinate
                int32_t tex_x = (int32_t)(wall_x * TEXTURE_SIZE);
                if (side == 0 && ray_dir_x > 0) tex_x = TEXTURE_SIZE - 1 - tex_x;
                if (side == 1 && ray_dir_z < 0) tex_x = TEXTURE_SIZE - 1 - tex_x;
                tex_x = tex_x & (TEXTURE_SIZE - 1);

                // Calculate texture Y step
                int32_t tex_y_step = (TEXTURE_SIZE << 12) / column_height;
                int32_t tex_y_start = ((renderer->height / 2 - vdraw_start) * tex_y_step + (TEXTURE_SIZE << 11)) & (~0xFFF);

                // Calculate shading
                float shade_factor = (side == 0) ? 0.85f : 0.7f;

                // Draw textured column
                draw_textured_column(renderer, x, vdraw_start, vdraw_end,
                                   tex_x, texture, tex_y_start, tex_y_step,
                                   shade_factor, COLOR_SKY, fog_factor);
            }

            walls_drawn++;
        }
    }

    // Debug log every 30 frames (only log if walls were drawn)
    static int32_t frame_count = 0;
    if (++frame_count % 30 == 0 && walls_drawn > 0) {
        ESP_LOGI(TAG, "Rendered %d faces/%d cols", walls_drawn, (end_col - start_col));
    }
}
static void renderer_render_wireframe(renderer_t* renderer);
static void renderer_render_3d(renderer_t* renderer);

/**
 * @brief Render a frame
 */
void renderer_render_frame(renderer_t* renderer) {
    if (!renderer || !renderer->framebuffer || !renderer->camera || !renderer->world) {
        return;
    }

    // Clear framebuffer with sky color
    renderer_clear(renderer, COLOR_SKY);

    // Check wireframe mode
    if (g_wireframe_mode) {
        // Render wireframe view (shows all voxels)
        renderer_render_wireframe(renderer);
    } else {
        // Normal textured rendering using true 3D projection
        renderer_render_3d(renderer);
    }

#if USE_PPA_UPSCALING
    // Apply PPA hardware upscaling (if enabled)
    if (renderer->ppa_scaler) {
        ppa_scaler_scale((ppa_scaler_t*)renderer->ppa_scaler);
    }
#endif
}

const uint16_t* renderer_get_framebuffer(const renderer_t* renderer) {
    if (!renderer) {
        return NULL;
    }

#if USE_PPA_UPSCALING
    // Return final scaled buffer (if PPA is enabled)
    if (renderer->ppa_scaler) {
        return ppa_scaler_get_final_buffer((ppa_scaler_t*)renderer->ppa_scaler);
    }
#endif

    return renderer->framebuffer;
}

void renderer_get_dimensions(const renderer_t* renderer, int32_t* width, int32_t* height) {
    if (!renderer) {
        if (width) *width = 0;
        if (height) *height = 0;
        return;
    }
    if (width) *width = renderer->width;
    if (height) *height = renderer->height;
}

/**
 * @brief Toggle wireframe rendering mode
 * @param new_mode If true, enable wireframe; if false, disable. If -1, toggle current state
 * @return Current wireframe mode state
 */
bool renderer_toggle_wireframe(int new_mode) {
    if (new_mode == -1) {
        g_wireframe_mode = !g_wireframe_mode;
    } else {
        g_wireframe_mode = (new_mode != 0);
    }

    ESP_LOGI(TAG, "Wireframe mode: %s", g_wireframe_mode ? "ON" : "OFF");
    return g_wireframe_mode;
}

// ============================================================
// MULTI-CORE RENDERING: Parallel Voxel Processing
// ============================================================

/**
 * @brief Collect voxels in world space range (thread-safe)
 * @param renderer Renderer context
 * @param buffer Output voxel buffer
 * @param x_min, x_max X-axis world space bounds (exclusive per-core split)
 *
 * This function collects voxels from the world within the specified X-axis range.
 * It's designed to be called in parallel by multiple cores with non-overlapping ranges.
 */
void renderer_collect_voxels_parallel(renderer_t* renderer, voxel_buffer_t* buffer,
                                      int32_t x_min, int32_t x_max) {
    if (!renderer || !renderer->world || !buffer) {
        return;
    }

    buffer->count = 0;

    int32_t cam_x = (int32_t)floorf(renderer->camera->x);
    int32_t cam_z = (int32_t)floorf(renderer->camera->z);
    int32_t render_distance = 20;

    voxel_world_t* world = renderer->world;

    // Iterate through world space, filtered by X range
    for (int32_t x = cam_x - render_distance; x <= cam_x + render_distance; x++) {
        // Skip if outside this core's X range
        if (x < x_min || x > x_max) continue;

        for (int32_t z = cam_z - render_distance; z <= cam_z + render_distance; z++) {
            int32_t highest_y = world_get_height(world, x, z);

            for (int32_t y = 0; y <= highest_y; y++) {
                block_type_t block = world_get_block(world, x, y, z);
                if (block != BLOCK_AIR) {
                    if (buffer->count < 2048) {
                        buffer->voxels[buffer->count].x = x;
                        buffer->voxels[buffer->count].y = y;
                        buffer->voxels[buffer->count].z = z;
                        buffer->voxels[buffer->count].block = block;
                        buffer->voxels[buffer->count].depth = 0.0f;  // Will be calculated
                        buffer->count++;
                    }
                }
            }
        }
    }
}

/**
 * @brief Sort voxels in buffer by depth (thread-safe)
 * @param buffer Voxel buffer to sort
 * @param cam_x, cam_y, cam_z Camera position for distance calculation
 *
 * Uses insertion sort to order voxels from far to near (painter's algorithm).
 * Each core sorts its own buffer independently, so no synchronization is needed.
 */
void renderer_sort_voxel_buffer(voxel_buffer_t* buffer,
                                 float cam_x, float cam_y, float cam_z) {
    if (!buffer || buffer->count == 0) {
        return;
    }

    const float epsilon = 0.01f;

    // OPTIMIZATION: Fixed-point distance sorting (1.2-1.5x faster on RISC-V)
    // Convert camera position to fixed-point once
    const fixed_t sort_cam_x = FIXED_FROM_FLOAT(cam_x);
    const fixed_t sort_cam_y = FIXED_FROM_FLOAT(cam_y);
    const fixed_t sort_cam_z = FIXED_FROM_FLOAT(cam_z);

    // Fixed-point epsilon for comparison (0.01 in 16.16 format)
    const fixed_t epsilon_fixed = FIXED_FROM_FLOAT(epsilon);

    // Insertion sort: O(n²) but with much better constants than bubble sort
    for (int i = 1; i < buffer->count; i++) {
        visible_voxel_t key = buffer->voxels[i];
        int j = i - 1;

        // Calculate distance for key element once (using fixed-point)
        fixed_t key_dx = FIXED_FROM_INT(key.x) - sort_cam_x;
        fixed_t key_dy = FIXED_FROM_INT(key.y) - sort_cam_y;
        fixed_t key_dz = FIXED_FROM_INT(key.z) - sort_cam_z;
        fixed_t key_dist_sq = fixed_mul(key_dx, key_dx) +
                             fixed_mul(key_dy, key_dy) +
                             fixed_mul(key_dz, key_dz);

        // Move elements that should come after key to one position ahead
        while (j >= 0) {
            // Calculate distance for current element (using fixed-point)
            fixed_t v_dx = FIXED_FROM_INT(buffer->voxels[j].x) - sort_cam_x;
            fixed_t v_dy = FIXED_FROM_INT(buffer->voxels[j].y) - sort_cam_y;
            fixed_t v_dz = FIXED_FROM_INT(buffer->voxels[j].z) - sort_cam_z;
            fixed_t v_dist_sq = fixed_mul(v_dx, v_dx) +
                               fixed_mul(v_dy, v_dy) +
                               fixed_mul(v_dz, v_dz);

            fixed_t dist_diff = v_dist_sq - key_dist_sq;
            bool should_swap_here = false;

            // v (current element) closer than key (should swap)
            if (dist_diff < -epsilon_fixed) {
                should_swap_here = true;
            } else if (dist_diff >= -epsilon_fixed && dist_diff <= epsilon_fixed) {
                // Distances are nearly equal - use deterministic tiebreaker
                // Sort by position (Y, then X, then Z) for consistent ordering
                if (buffer->voxels[j].y > key.y ||
                    (buffer->voxels[j].y == key.y && buffer->voxels[j].x > key.x) ||
                    (buffer->voxels[j].y == key.y && buffer->voxels[j].x == key.x && buffer->voxels[j].z > key.z)) {
                    should_swap_here = true;
                }
            }

            if (!should_swap_here) {
                break;  // Found correct position
            }

            // Move element one position ahead
            buffer->voxels[j + 1] = buffer->voxels[j];
            j--;
        }

        // Place key in its correct position
        buffer->voxels[j + 1] = key;
    }
}

/**
 * @brief Render voxels from buffer
 * @param renderer Renderer context
 * @param buffer Voxel buffer to render
 *
 * Renders all voxels in the buffer using the 3D textured voxel renderer.
 * Voxels should already be sorted by depth (far to near) for correct rendering.
 */
void renderer_render_voxel_buffer(renderer_t* renderer, const voxel_buffer_t* buffer) {
    if (!renderer || !buffer) {
        return;
    }

    // Render voxels back-to-front (already sorted)
    for (int i = 0; i < buffer->count; i++) {
        draw_voxel_3d(renderer, buffer->voxels[i].x, buffer->voxels[i].y,
                      buffer->voxels[i].z, buffer->voxels[i].block);
    }
}

/**
 * @brief Project a 3D point to screen space
 * @param renderer Renderer context
 * @param wx, wy, wz World coordinates
 * @param sx, sy Output screen coordinates
 * @return true if point is in front of camera, false if behind
 */
static bool project_point(renderer_t* renderer, float wx, float wy, float wz, float* sx, float* sy, float* depth) {
    // Calculate offset from camera
    float dx = wx - renderer->camera->x;
    float dy = wy - renderer->camera->y;
    float dz = wz - renderer->camera->z;

    // Rotate by camera yaw (around Y axis)
    float cos_yaw = cosf_fast(-renderer->camera->yaw);
    float sin_yaw = sinf_fast(-renderer->camera->yaw);

    float rx = dx * cos_yaw - dz * sin_yaw;
    float rz = dx * sin_yaw + dz * cos_yaw;

    // Skip if behind camera
    if (rz < 0.1f) {
        return false;
    }

    // Rotate by camera pitch (around X axis)
    float cos_pitch = cosf_fast(-renderer->camera->pitch);
    float sin_pitch = sinf_fast(-renderer->camera->pitch);

    float ry = dy * cos_pitch - rz * sin_pitch;
    float final_rz = dy * sin_pitch + rz * cos_pitch;

    // Skip if behind camera after pitch rotation
    if (final_rz < 0.1f) {
        return false;
    }

    // Perspective projection
    float fov_scale = renderer->height / 2.0f;
    *sx = renderer->width / 2 + (rx / final_rz) * fov_scale;
    *sy = renderer->height / 2 - (ry / final_rz) * fov_scale;
    *depth = final_rz;

    return true;
}

#if USE_FIXED_POINT
// OPTIMIZATION: Place in IRAM - called 16,000 times per frame
// This is THE hottest function in the rendering pipeline
#ifdef __ESP32_P4__
#define IRAM_FN __attribute__((section(".iram.text")))
#else
#define IRAM_FN
#endif

/**
 * @brief Fixed-point version of project_point (2-3x faster on RISC-V without FPU)
 * @param renderer Renderer context
 * @param wx, wy, wz World coordinates (float for interface compatibility)
 * @param sx, sy Output screen coordinates (float for interface compatibility)
 * @param depth Output depth
 * @return true if point is in front of camera
 *
 * CRITICAL: This function is called 16,000 times per frame (8 corners × 2000 voxels)
 * Placed in IRAM for maximum performance
 */
static bool IRAM_FN project_point_fixed(renderer_t* renderer, float wx, float wy, float wz,
                                float* sx, float* sy, float* depth) {
    // Convert world coordinates to fixed-point
    fixed_t fx = FIXED_FROM_FLOAT(wx);
    fixed_t fy = FIXED_FROM_FLOAT(wy);
    fixed_t fz = FIXED_FROM_FLOAT(wz);

    fixed_t cam_x = FIXED_FROM_FLOAT(renderer->camera->x);
    fixed_t cam_y = FIXED_FROM_FLOAT(renderer->camera->y);
    fixed_t cam_z = FIXED_FROM_FLOAT(renderer->camera->z);

    // Calculate offset from camera (fixed-point subtraction)
    fixed_t dx = fx - cam_x;
    fixed_t dy = fy - cam_y;
    fixed_t dz = fz - cam_z;

    // Rotate by camera yaw (around Y axis)
    fixed_t cos_yaw = fixed_cos(FIXED_FROM_FLOAT(-renderer->camera->yaw));
    fixed_t sin_yaw = fixed_sin(FIXED_FROM_FLOAT(-renderer->camera->yaw));

    fixed_t rx = fixed_mul(dx, cos_yaw) - fixed_mul(dz, sin_yaw);
    fixed_t rz = fixed_mul(dx, sin_yaw) + fixed_mul(dz, cos_yaw);

    // Skip if behind camera (0.1 in fixed-point)
    if (rz < FIXED_FROM_FLOAT(0.1f)) {
        return false;
    }

    // Rotate by camera pitch (around X axis)
    fixed_t cos_pitch = fixed_cos(FIXED_FROM_FLOAT(-renderer->camera->pitch));
    fixed_t sin_pitch = fixed_sin(FIXED_FROM_FLOAT(-renderer->camera->pitch));

    fixed_t ry = fixed_mul(dy, cos_pitch) - fixed_mul(rz, sin_pitch);
    fixed_t final_rz = fixed_mul(dy, sin_pitch) + fixed_mul(rz, cos_pitch);

    // Skip if behind camera after pitch rotation
    if (final_rz < FIXED_FROM_FLOAT(0.1f)) {
        return false;
    }

    // Perspective projection (division is still slow, but we use fixed_mul)
    fixed_t fov_scale = FIXED_FROM_INT(renderer->height / 2);
    fixed_t screen_x_fixed = FIXED_FROM_INT(renderer->width / 2) +
                            fixed_div(fixed_mul(rx, fov_scale), final_rz);
    fixed_t screen_y_fixed = FIXED_FROM_INT(renderer->height / 2) -
                            fixed_div(fixed_mul(ry, fov_scale), final_rz);

    // Convert back to float for interface compatibility
    *sx = FIXED_TO_FLOAT(screen_x_fixed);
    *sy = FIXED_TO_FLOAT(screen_y_fixed);
    *depth = FIXED_TO_FLOAT(final_rz);

    return true;
}
#endif // USE_FIXED_POINT

/**
 * @brief Draw a textured quad (4 vertices)
 * @param renderer Renderer context
 * @param x0, y0, x1, y1, x2, y2, x3, y3 Screen coordinates (4 corners in order)
 * @param tex_x0, tex_y0, tex_x1, tex_y1 Texture coordinates (0-7)
 * @param texture Texture data
 * @param shade Shading factor
 */
static void draw_textured_quad(renderer_t* renderer,
                               float x0, float y0, float x1, float y1,
                               float x2, float y2, float x3, float y3,
                               int tex_x0, int tex_y0, int tex_x1, int tex_y1,
                               const uint16_t* texture, float shade) {
    if (!renderer || !renderer->framebuffer || !texture) {
        return;
    }

    // Debug: Log first quad
    static bool first_quad = true;
    static int total_quads = 0;
    static int total_pixels = 0;
    int pixels_drawn = 0;

    total_quads++;
    if (total_quads <= 10) {
        ESP_LOGI(TAG, "Quad #%d: vertices (%.1f,%.1f) (%.1f,%.1f) (%.1f,%.1f) (%.1f,%.1f)",
                 total_quads, x0, y0, x1, y1, x2, y2, x3, y3);
    }

    // Bounding box for clipping
    int min_x = (int)floorf(fminf(fminf(x0, x1), fminf(x2, x3)));
    int max_x = (int)ceilf(fmaxf(fmaxf(x0, x1), fmaxf(x2, x3)));
    int min_y = (int)floorf(fminf(fminf(y0, y1), fminf(y2, y3)));
    int max_y = (int)ceilf(fmaxf(fmaxf(y0, y1), fmaxf(y2, y3)));

    // Clip to screen
    if (min_x < 0) min_x = 0;
    if (max_x >= renderer->width) max_x = renderer->width - 1;
    if (min_y < 0) min_y = 0;
    if (max_y >= renderer->height) max_y = renderer->height - 1;

    // Rasterize quad using scanlines
    for (int y = min_y; y <= max_y; y++) {
        // Find intersections with quad edges
        float intersections[4];
        int intersect_count = 0;

        // Edge 0-1
        if ((y0 <= y && y1 > y) || (y1 <= y && y0 > y)) {
            float t = (y - y0) / (y1 - y0);
            intersections[intersect_count++] = x0 + t * (x1 - x0);
        }
        // Edge 1-2
        if ((y1 <= y && y2 > y) || (y2 <= y && y1 > y)) {
            float t = (y - y1) / (y2 - y1);
            intersections[intersect_count++] = x1 + t * (x2 - x1);
        }
        // Edge 2-3
        if ((y2 <= y && y3 > y) || (y3 <= y && y2 > y)) {
            float t = (y - y2) / (y3 - y2);
            intersections[intersect_count++] = x2 + t * (x3 - x2);
        }
        // Edge 3-0
        if ((y3 <= y && y0 > y) || (y0 <= y && y3 > y)) {
            float t = (y - y3) / (y0 - y3);
            intersections[intersect_count++] = x3 + t * (x0 - x3);
        }

        if (intersect_count < 2) continue;

        // Sort intersections
        if (intersections[0] > intersections[1]) {
            float temp = intersections[0];
            intersections[0] = intersections[1];
            intersections[1] = temp;
        }

        // Draw span
        int x_start = (int)ceilf(intersections[0]);
        int x_end = (int)floorf(intersections[1]);

        if (x_start < min_x) x_start = min_x;
        if (x_end > max_x) x_end = max_x;

        for (int x = x_start; x <= x_end; x++) {
            // Calculate barycentric coordinates for texture mapping
            // Simplified: just interpolate based on position
            float u = (float)(x - x_start) / (x_end - x_start + 1);
            int tex_x = tex_x0 + (int)(u * (tex_x1 - tex_x0));
            int tex_y = tex_y0 + (int)(u * (tex_y1 - tex_y0));

            // Clamp texture coordinates
            tex_x = tex_x & 0x7;
            tex_y = tex_y & 0x7;

            uint16_t texel = texture[tex_y * TEXTURE_SIZE + tex_x];
            texel = shade_color(texel, shade);

            renderer->framebuffer[y * renderer->width + x] = texel;
            pixels_drawn++;
        }
    }

    total_pixels += pixels_drawn;

    if (first_quad && pixels_drawn > 0) {
        ESP_LOGI(TAG, "First quad: bbox=(%d,%d)-(%d,%d), drew %d pixels",
                 min_x, min_y, max_x, max_y, pixels_drawn);
        ESP_LOGI(TAG, "  Vertices: (%.1f,%.1f) (%.1f,%.1f) (%.1f,%.1f) (%.1f,%.1f)",
                 x0, y0, x1, y1, x2, y2, x3, y3);
        first_quad = false;
    }

    // Log summary after 1000 quads
    if (total_quads == 1000) {
        ESP_LOGI(TAG, "After 1000 quads: %d total pixels drawn", total_pixels);
    }
}

// OPTIMIZATION: Precomputed voxel face data (static const = initialized once at boot)
// Voxel corners: 0-3 are bottom (Y=0), 4-7 are top (Y=1)
// Bottom: 0=(0,0,0) 1=(1,0,0) 2=(1,0,1) 3=(0,0,1)
// Top:    4=(0,1,0) 5=(1,1,0) 6=(1,1,1) 7=(0,1,1)
// Format: {4 corner indices in counter-clockwise order when viewed from outside}
static const int g_voxel_faces[6][5] = {
    {0, 3, 7, 4, 0},  // Left (X-): normal = (-1, 0, 0)
    {1, 2, 6, 5, 1},  // Right (X+): normal = (1, 0, 0)
    {0, 1, 5, 4, 2},  // Front (Z-): normal = (0, 0, -1)
    {2, 3, 7, 6, 2},  // Back (Z+): normal = (0, 0, 1)
    {4, 5, 6, 7, 1},  // Top (Y+): normal = (0, 1, 0)
    {0, 3, 2, 1, 1}   // Bottom (Y-): normal = (0, -1, 0)
};

static const float g_voxel_normals[6][3] = {
    {-1, 0, 0},  // Left (X-)
    {1, 0, 0},   // Right (X+)
    {0, 0, -1},  // Front (Z-)
    {0, 0, 1},   // Back (Z+)
    {0, 1, 0},   // Top (Y+)
    {0, -1, 0}   // Bottom (Y-)
};

static const float g_voxel_face_shades[6] = {0.6f, 0.5f, 0.7f, 0.5f, 1.0f, 0.4f};

/**
 * @brief Draw a single voxel with textured faces
 * @param renderer Renderer context
 * @param x, y, z World coordinates of voxel
 * @param block Block type
 */
static void draw_voxel_3d(renderer_t* renderer, int32_t x, int32_t y, int32_t z, block_type_t block) {
    if (!renderer || !renderer->framebuffer) {
        return;
    }

    const uint16_t* texture = textures[block];
    if (!texture) return;

    // Voxel 8 corners in world space
    float corners[8][3] = {
        {x, y, z}, {x+1, y, z}, {x+1, y, z+1}, {x, y, z+1},     // Bottom face (y)
        {x, y+1, z}, {x+1, y+1, z}, {x+1, y+1, z+1}, {x, y+1, z+1}  // Top face (y+1)
    };

    // Project all 8 corners
    float screen_x[8], screen_y[8], corner_depth[8];
    int visible_corners = 0;

    // Initialize all corner_depth to -1 (behind camera/unprojected)
    for (int i = 0; i < 8; i++) {
        corner_depth[i] = -1.0f;
    }

    for (int i = 0; i < 8; i++) {
#if USE_FIXED_POINT
        if (project_point_fixed(renderer, corners[i][0], corners[i][1], corners[i][2],
                               &screen_x[i], &screen_y[i], &corner_depth[i])) {
#else
        if (project_point(renderer, corners[i][0], corners[i][1], corners[i][2],
                         &screen_x[i], &screen_y[i], &corner_depth[i])) {
#endif
            visible_corners++;
        }
    }

    if (visible_corners == 0) {
        return;  // Entire voxel behind camera
    }

    // Check camera direction to determine which faces are visible
    float cam_dx = renderer->camera->x - (x + 0.5f);
    float cam_dy = renderer->camera->y - (y + 0.5f);
    float cam_dz = renderer->camera->z - (z + 0.5f);

    // OPTIMIZATION: Check all 6 neighbors ONCE per voxel (not 6× per face)
    // Bitmask: bit 0 = X-, 1 = X+, 2 = Z-, 3 = Z+, 4 = Y+, 5 = Y-
    uint8_t neighbor_occluded = 0;

    // Check X- neighbors
    if (world_get_block(renderer->world, x - 1, y, z) != BLOCK_AIR) {
        neighbor_occluded |= (1 << 0);  // X- face blocked
    }
    // Check X+ neighbors
    if (world_get_block(renderer->world, x + 1, y, z) != BLOCK_AIR) {
        neighbor_occluded |= (1 << 1);  // X+ face blocked
    }
    // Check Z- neighbors
    if (world_get_block(renderer->world, x, y, z - 1) != BLOCK_AIR) {
        neighbor_occluded |= (1 << 2);  // Z- face blocked
    }
    // Check Z+ neighbors
    if (world_get_block(renderer->world, x, y, z + 1) != BLOCK_AIR) {
        neighbor_occluded |= (1 << 3);  // Z+ face blocked
    }
    // Check Y+ neighbors (top)
    if (y + 1 <= 15 && world_get_block(renderer->world, x, y + 1, z) != BLOCK_AIR) {
        neighbor_occluded |= (1 << 4);  // Y+ face blocked
    }
    // Check Y- neighbors (bottom)
    if (y > 0 && world_get_block(renderer->world, x, y - 1, z) != BLOCK_AIR) {
        neighbor_occluded |= (1 << 5);  // Y- face blocked
    }

    // Debug: Log first voxel
    static bool first_voxel = true;
    int faces_rendered = 0;

    // Render visible faces with occlusion culling
    // Uses precomputed global arrays: g_voxel_faces, g_voxel_normals, g_voxel_face_shades
    for (int f = 0; f < 6; f++) {
        // Back-face culling: dot product of face normal with camera direction
        float dot = g_voxel_normals[f][0] * cam_dx + g_voxel_normals[f][1] * cam_dy + g_voxel_normals[f][2] * cam_dz;

        if (dot > 0) {  // Face is visible (camera is in front of this face)
            // OPTIMIZATION: Fast bitmask check instead of repeated world_get_block() calls
            if (neighbor_occluded & (1 << f)) {
                // This face is occluded by adjacent block
                continue;
            }

            const int* face = g_voxel_faces[f];
            float fx0 = screen_x[face[0]], fy0 = screen_y[face[0]];
            float fx1 = screen_x[face[1]], fy1 = screen_y[face[1]];
            float fx2 = screen_x[face[2]], fy2 = screen_y[face[2]];
            float fx3 = screen_x[face[3]], fy3 = screen_y[face[3]];

            // Skip if any corner is behind camera (project_point returned false)
            if (corner_depth[face[0]] < 0.1f || corner_depth[face[1]] < 0.1f ||
                corner_depth[face[2]] < 0.1f || corner_depth[face[3]] < 0.1f) {
                continue;
            }

            // Additional validation: skip if projected coordinates are invalid or extreme
            // This prevents "triangle spikes" when vertices are near camera plane
            const float MAX_COORD = 10000.0f;  // Reasonable screen coordinate limit
            if (fabsf(fx0) > MAX_COORD || fabsf(fy0) > MAX_COORD ||
                fabsf(fx1) > MAX_COORD || fabsf(fy1) > MAX_COORD ||
                fabsf(fx2) > MAX_COORD || fabsf(fy2) > MAX_COORD ||
                fabsf(fx3) > MAX_COORD || fabsf(fy3) > MAX_COORD) {
                continue;
            }

            // Skip if the quad bounding box is too small (likely a degenerate quad)
            float min_x = fminf(fminf(fx0, fx1), fminf(fx2, fx3));
            float max_x = fmaxf(fmaxf(fx0, fx1), fmaxf(fx2, fx3));
            float min_y = fminf(fminf(fy0, fy1), fminf(fy2, fy3));
            float max_y = fmaxf(fmaxf(fy0, fy1), fmaxf(fy2, fy3));

            float quad_width = max_x - min_x;
            float quad_height = max_y - min_y;

            // Skip tiny quads (likely artifacts)
            if (quad_width < 0.5f || quad_height < 0.5f) {
                continue;
            }

            // Texture coordinates for this face
            int tx0, ty0, tx1, ty1;
            if (f < 4) {  // Vertical faces
                tx0 = 0; ty0 = 0; tx1 = 7; ty1 = 7;
            } else {  // Top/bottom faces
                tx0 = 0; ty0 = 0; tx1 = 7; ty1 = 7;
            }

            draw_textured_quad(renderer, fx0, fy0, fx1, fy1, fx2, fy2, fx3, fy3,
                             tx0, ty0, tx1, ty1, texture, g_voxel_face_shades[f]);
            faces_rendered++;
        }
    }

    if (first_voxel && faces_rendered > 0) {
        ESP_LOGI(TAG, "First voxel at (%d,%d,%d): %d faces rendered, visible_corners=%d",
                 x, y, z, faces_rendered, visible_corners);
        ESP_LOGI(TAG, "  Camera direction: (%.2f, %.2f, %.2f)", cam_dx, cam_dy, cam_dz);
        first_voxel = false;
    }
}

/**
 * @brief Draw a single voxel as wireframe
 * @param renderer Renderer context
 * @param x, y, z World coordinates of voxel
 * @param color Wireframe color
 */
static void draw_voxel_wireframe(renderer_t* renderer, int32_t x, int32_t y, int32_t z, uint16_t color) {
    if (!renderer || !renderer->framebuffer) {
        return;
    }

    // Project 3D voxel position to 2D screen space
    // This is a simple orthographic projection for debugging

    // Calculate offset from camera
    float dx = x - renderer->camera->x;
    float dy = y - renderer->camera->y;
    float dz = z - renderer->camera->z;

    // Rotate by camera yaw
    float cos_yaw = cosf_fast(-renderer->camera->yaw);
    float sin_yaw = sinf_fast(-renderer->camera->yaw);

    float rx = dx * cos_yaw - dz * sin_yaw;
    float rz = dx * sin_yaw + dz * cos_yaw;

    // Skip if behind camera
    if (rz < 0.1f) {
        return;
    }

    // Simple perspective projection
    float fov_scale = renderer->height / 2.0f;
    float screen_x = renderer->width / 2 + (rx / rz) * fov_scale;
    float screen_y = renderer->height / 2 - (dy / rz) * fov_scale;

    // Calculate voxel size on screen (decreases with distance)
    float voxel_screen_size = fov_scale / rz;

    // Skip if off screen
    if (screen_x + voxel_screen_size < 0 || screen_x - voxel_screen_size >= renderer->width ||
        screen_y + voxel_screen_size < 0 || screen_y - voxel_screen_size >= renderer->height) {
        return;
    }

    // Draw wireframe edges (cube outline)
    int32_t x1 = (int32_t)(screen_x - voxel_screen_size / 2);
    int32_t x2 = (int32_t)(screen_x + voxel_screen_size / 2);
    int32_t y1 = (int32_t)(screen_y - voxel_screen_size / 2);
    int32_t y2 = (int32_t)(screen_y + voxel_screen_size / 2);

    // Clamp to screen bounds
    if (x1 < 0) x1 = 0;
    if (x2 >= renderer->width) x2 = renderer->width - 1;
    if (y1 < 0) y1 = 0;
    if (y2 >= renderer->height) y2 = renderer->height - 1;

    // Draw edges
    for (int32_t ix = x1; ix <= x2; ix++) {
        // Top and bottom edges
        renderer->framebuffer[y1 * renderer->width + ix] = color;
        renderer->framebuffer[y2 * renderer->width + ix] = color;
    }
    for (int32_t iy = y1; iy <= y2; iy++) {
        // Left and right edges
        renderer->framebuffer[iy * renderer->width + x1] = color;
        renderer->framebuffer[iy * renderer->width + x2] = color;
    }
}

/**
 * @brief Render wireframe view of all voxels
 * @param renderer Renderer context
 */
static void renderer_render_wireframe(renderer_t* renderer) {
    if (!renderer || !renderer->framebuffer || !renderer->world) {
        return;
    }

    // Iterate through all blocks in the world
    voxel_world_t* world = renderer->world;
    int32_t render_distance = 20;  // Only render nearby blocks

    int32_t cam_x = (int32_t)floorf(renderer->camera->x);
    int32_t cam_z = (int32_t)floorf(renderer->camera->z);

    for (int32_t x = cam_x - render_distance; x <= cam_x + render_distance; x++) {
        for (int32_t z = cam_z - render_distance; z <= cam_z + render_distance; z++) {
            // Get highest block at this (x, z)
            int32_t highest_y = world_get_height(world, x, z);

            // Draw all blocks from Y=0 to highest_y
            for (int32_t y = 0; y <= highest_y; y++) {
                block_type_t block = world_get_block(world, x, y, z);
                if (block != BLOCK_AIR) {
                    // Choose color based on block type
                    uint16_t color;
                    switch (block) {
                        case BLOCK_GRASS:   color = RGB565(0, 200, 0); break;
                        case BLOCK_DIRT:    color = RGB565(139, 69, 19); break;
                        case BLOCK_STONE:   color = RGB565(128, 128, 128); break;
                        case BLOCK_QUARTZ:  color = RGB565(255, 255, 255); break;
                        case BLOCK_GLASS:   color = RGB565(200, 200, 255); break;
                        case BLOCK_TERRACOTTA: color = RGB565(200, 100, 100); break;
                        case BLOCK_WATER:   color = RGB565(0, 100, 255); break;
                        default:            color = COLOR_WIREFRAME_EDGE; break;
                    }

                    draw_voxel_wireframe(renderer, x, y, z, color);
                }
            }
        }
    }
}

/**
 * @brief Render 3D voxels with textures (true 3D renderer)
 * @param renderer Renderer context
 */
static void renderer_render_3d(renderer_t* renderer) {
    if (!renderer || !renderer->framebuffer || !renderer->world) {
        return;
    }

    voxel_world_t* world = renderer->world;
    int32_t render_distance = 20;

    int32_t cam_x = (int32_t)floorf(renderer->camera->x);
    int32_t cam_z = (int32_t)floorf(renderer->camera->z);

    // Debug: Log first frame
    static bool first_frame = true;
    if (first_frame) {
        ESP_LOGI(TAG, "3D Renderer: Camera at (%.1f, %.1f, %.1f), yaw=%.2f, pitch=%.2f",
                 renderer->camera->x, renderer->camera->y, renderer->camera->z,
                 renderer->camera->yaw, renderer->camera->pitch);
        first_frame = false;
    }

    // Collect all visible voxels
    typedef struct {
        int32_t x, y, z;
        block_type_t block;
        float depth;
    } visible_voxel_t;

    visible_voxel_t voxels[4096];  // Up to 4096 visible voxels
    int voxel_count = 0;

    for (int32_t x = cam_x - render_distance; x <= cam_x + render_distance; x++) {
        for (int32_t z = cam_z - render_distance; z <= cam_z + render_distance; z++) {
            int32_t highest_y = world_get_height(world, x, z);

            for (int32_t y = 0; y <= highest_y; y++) {
                block_type_t block = world_get_block(world, x, y, z);
                if (block != BLOCK_AIR) {
                    if (voxel_count < 4096) {
                        voxels[voxel_count].x = x;
                        voxels[voxel_count].y = y;
                        voxels[voxel_count].z = z;
                        voxels[voxel_count].block = block;
                        voxels[voxel_count].depth = 0.0f;  // Will be calculated
                        voxel_count++;
                    }
                }
            }
        }
    }

    // ESP_LOGI(TAG, "3D Renderer: Collected %d voxels", voxel_count);  // Disabled - too verbose

    // Sort voxels by depth (far to near) for painter's algorithm
    // OPTIMIZATION: Use insertion sort instead of bubble sort (1.5-2x faster)
    // Largest distance first, smallest distance last
    // Use epsilon tolerance to prevent flickering (z-fighting)
    const float epsilon = 0.01f;  // Tolerance for distance comparison (optimized for 32-bit)

    // OPTIMIZATION: Fixed-point distance sorting (1.2-1.5x faster on RISC-V)
    // Convert camera position to fixed-point once
    const fixed_t sort_cam_x = FIXED_FROM_FLOAT(renderer->camera->x);
    const fixed_t sort_cam_y = FIXED_FROM_FLOAT(renderer->camera->y);
    const fixed_t sort_cam_z = FIXED_FROM_FLOAT(renderer->camera->z);

    // Fixed-point epsilon for comparison (0.01 in 16.16 format)
    const fixed_t epsilon_fixed = FIXED_FROM_FLOAT(epsilon);

    // Insertion sort: O(n²) but with much better constants than bubble sort
    // For partially sorted data (typical in 3D rendering), it's significantly faster
    for (int i = 1; i < voxel_count; i++) {
        visible_voxel_t key = voxels[i];
        int j = i - 1;

        // Calculate distance for key element once (using fixed-point)
        fixed_t key_dx = FIXED_FROM_INT(key.x) - sort_cam_x;
        fixed_t key_dy = FIXED_FROM_INT(key.y) - sort_cam_y;
        fixed_t key_dz = FIXED_FROM_INT(key.z) - sort_cam_z;
        fixed_t key_dist_sq = fixed_mul(key_dx, key_dx) +
                             fixed_mul(key_dy, key_dy) +
                             fixed_mul(key_dz, key_dz);

        // Move elements that should come after key to one position ahead
        while (j >= 0) {
            // Calculate distance for current element (using fixed-point)
            fixed_t v_dx = FIXED_FROM_INT(voxels[j].x) - sort_cam_x;
            fixed_t v_dy = FIXED_FROM_INT(voxels[j].y) - sort_cam_y;
            fixed_t v_dz = FIXED_FROM_INT(voxels[j].z) - sort_cam_z;
            fixed_t v_dist_sq = fixed_mul(v_dx, v_dx) +
                               fixed_mul(v_dy, v_dy) +
                               fixed_mul(v_dz, v_dz);

            fixed_t dist_diff = v_dist_sq - key_dist_sq;
            bool should_swap_here = false;

            // v (current element) closer than key (should swap)
            if (dist_diff < -epsilon_fixed) {
                should_swap_here = true;
            } else if (dist_diff >= -epsilon_fixed && dist_diff <= epsilon_fixed) {
                // Distances are nearly equal - use deterministic tiebreaker
                // Sort by position (Y, then X, then Z) for consistent ordering
                if (voxels[j].y > key.y ||
                    (voxels[j].y == key.y && voxels[j].x > key.x) ||
                    (voxels[j].y == key.y && voxels[j].x == key.x && voxels[j].z > key.z)) {
                    should_swap_here = true;
                }
            }

            if (!should_swap_here) {
                break;  // Found correct position
            }

            // Move element one position ahead
            voxels[j + 1] = voxels[j];
            j--;
        }

        // Place key in its correct position
        voxels[j + 1] = key;
    }

    // Render voxels back-to-front
    for (int i = 0; i < voxel_count; i++) {
        draw_voxel_3d(renderer, voxels[i].x, voxels[i].y, voxels[i].z, voxels[i].block);
    }
}
