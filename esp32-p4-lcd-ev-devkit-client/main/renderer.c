/**
 * @file renderer.c
 * @brief Raycasting renderer implementation
 */

#include "renderer.h"
#include "world.h"
#include "trig_lut.h"
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <math.h>
#include <esp_log.h>

static const char* TAG = "Renderer";

// Wireframe mode (toggle with F1)
static bool g_wireframe_mode = false;

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

    // Allocate framebuffer (RGB565 = 2 bytes per pixel)
    renderer->framebuffer = (uint16_t*)malloc(width * height * sizeof(uint16_t));
    if (!renderer->framebuffer) {
        ESP_LOGE(TAG, "Failed to allocate framebuffer");
        return false;
    }

    renderer->width = width;
    renderer->height = height;
    renderer->camera = camera;
    renderer->world = world;

    ESP_LOGI(TAG, "Renderer initialized: %dx%d", width, height);
    return true;
}

void renderer_free(renderer_t* renderer) {
    if (!renderer) {
        return;
    }

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

        // Debug: Log ray angles for first few columns
        static int angle_debug_count = 0;
        if (angle_debug_count < 5) {
            ESP_LOGI(TAG, "ANGLE_DEBUG #%d: x=%d, camera_x=%.4f, yaw=%.4f, fov=%.4f, ray_angle=%.4f",
                     angle_debug_count, x, camera_x, renderer->camera->yaw, renderer->camera->fov, ray_angle);
            angle_debug_count++;
        }

        // Ray direction (use fast lookup tables)
        // Calculate 3D ray direction from yaw and pitch
        float ray_dir_x = cosf_fast(ray_angle) * cosf_fast(renderer->camera->pitch);
        float ray_dir_y = sinf_fast(renderer->camera->pitch);  // Up/down component
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

        // Perform 3D DDA (Digital Differential Analyzer)
        bool hit = false;
        int32_t side = 0;  // 0=NS, 1=EW, 2=Top/Bottom
        block_type_t block_type = BLOCK_AIR;
        int32_t max_steps = 50;  // Prevent infinite loops
        int32_t steps = 0;
        int32_t hit_y = 0;  // Y-coordinate of hit block

        while (!hit && steps < max_steps) {
            // Determine which face we'll hit BEFORE stepping
            if (side_dist_x < side_dist_z) {
                if (side_dist_x < side_dist_y) {
                    // X is closest - will hit North/South face
                    side = 0;
                    side_dist_x += delta_dist_x;
                    map_x += step_x;
                } else {
                    // Y is closest - will hit Top/Bottom face
                    side = 2;
                    side_dist_y += delta_dist_y;
                    map_y += step_y;
                }
            } else {
                if (side_dist_z < side_dist_y) {
                    // Z is closest - will hit East/West face
                    side = 1;
                    side_dist_z += delta_dist_z;
                    map_z += step_z;
                } else {
                    // Y is closest - will hit Top/Bottom face
                    side = 2;
                    side_dist_y += delta_dist_y;
                    map_y += step_y;
                }
            }

            // Now check for block at NEW position after stepping
            block_type = world_get_block(renderer->world, map_x, map_y, map_z);
            if (block_type != BLOCK_AIR) {
                hit = true;
                hit_y = map_y;
                break;
            }

            steps++;
        }

        // Calculate distance to wall (perpendicular to avoid fisheye)
        float perp_wall_dist;
        float wall_x;  // Position on the wall for texture mapping

        // Debug: Log first few hits to see which faces are detected
        static int hit_debug_count = 0;
        if (hit_debug_count < 10) {
            ESP_LOGI(TAG, "HIT_DEBUG #%d: pos=(%d,%d,%d), side=%d (0=NS,1=EW,2=TB), type=%d",
                     hit_debug_count, map_x, map_y, map_z, side, block_type);
            hit_debug_count++;
        }

        // Special handling for top/bottom faces (side=2)
        if (side == 2) {
            // We hit the top or bottom of a block
            // Calculate distance to this horizontal face
            perp_wall_dist = (map_y - renderer->camera->y + (1 - step_y) / 2.0f) / ray_dir_y;

            // Skip if behind camera
            if (perp_wall_dist <= 0.0f) {
                continue;
            }

            // For horizontal faces, draw as a horizontal strip
            // Calculate the vertical position on screen
            // y_offset: positive = below camera, negative = above camera
            float y_offset = (renderer->camera->y - hit_y) / perp_wall_dist;
            int32_t v_center = renderer->height / 2 + (int32_t)(y_offset * renderer->height / 2.0f);

            // Draw a horizontal strip (top/bottom face of block)
            // Calculate strip height based on perspective
            // A full block at this distance would occupy: block_size / distance * screen_height
            int32_t strip_height = (int32_t)(1.0f / perp_wall_dist * renderer->height);
            if (strip_height < 1) strip_height = 1;
            if (strip_height > renderer->height / 2) strip_height = renderer->height / 2;

            int32_t draw_start = v_center - strip_height / 2;
            if (draw_start < 0) draw_start = 0;
            int32_t draw_end = v_center + strip_height / 2;
            if (draw_end >= renderer->height) draw_end = renderer->height - 1;

            // Debug: Log horizontal face rendering
            static int horiz_debug_count = 0;
            if (horiz_debug_count < 5) {
                ESP_LOGI(TAG, "HORIZ_DEBUG #%d: y_offset=%.2f, v_center=%d, strip_h=%d, range=%d..%d, dist=%.2f",
                         horiz_debug_count, y_offset, v_center, strip_height, draw_start, draw_end, perp_wall_dist);
                horiz_debug_count++;
            }

            // Get texture for this block
            const uint16_t* texture = textures[block_type];
            if (!texture) {
                continue;
            }

            // Texture coordinates (top-down view)
            float hit_x = renderer->camera->x + perp_wall_dist * ray_dir_x;
            float hit_z = renderer->camera->z + perp_wall_dist * ray_dir_z;

            float tex_x_frac = hit_x - floorf(hit_x);
            float tex_z_frac = hit_z - floorf(hit_z);

            int32_t tex_x_base = (int32_t)(tex_x_frac * TEXTURE_SIZE) & (TEXTURE_SIZE - 1);
            int32_t tex_y_base = (int32_t)(tex_z_frac * TEXTURE_SIZE) & (TEXTURE_SIZE - 1);

            // Draw the horizontal strip
            for (int32_t ty = draw_start; ty <= draw_end; ty++) {
                // Use same texture for entire strip (top-down view)
                uint16_t texel = texture[tex_y_base * TEXTURE_SIZE + tex_x_base];

                // Apply shading (top faces are bright)
                texel = shade_color(texel, 1.0f);

                // Draw
                renderer->framebuffer[ty * renderer->width + x] = texel;
            }

            walls_drawn++;
            continue;  // Done with this column
        }

        // Normal X or Z face hit
        if (side == 0) {
            // Hit X-facing wall
            perp_wall_dist = (map_x - renderer->camera->x + (1 - step_x) / 2.0f) / ray_dir_x;
            wall_x = renderer->camera->z + perp_wall_dist * ray_dir_z;
        } else {
            // Hit Z-facing wall
            perp_wall_dist = (map_z - renderer->camera->z + (1 - step_z) / 2.0f) / ray_dir_z;
            wall_x = renderer->camera->x + perp_wall_dist * ray_dir_x;
        }

        // Skip walls behind camera (negative distance)
        if (perp_wall_dist <= 0.0f) {
            continue;
        }

        // Calculate height of line to draw (classic Wolf3D formula)
        int32_t line_height = (int32_t)(renderer->height / perp_wall_dist);

        // Calculate vertical centering based on hit position relative to camera
        // If we hit something above camera (hit_y > camera_y), shift wall up
        // If we hit something below camera (hit_y < camera_y), shift wall down
        float y_offset = (hit_y - renderer->camera->y) / perp_wall_dist;
        int32_t v_center = renderer->height / 2 - (int32_t)(y_offset * renderer->height / 2.0f);

        // Calculate draw positions (centered vertically on screen)
        int32_t draw_start = -line_height / 2 + v_center;
        if (draw_start < 0) draw_start = 0;
        int32_t draw_end = line_height / 2 + v_center;
        if (draw_end >= renderer->height) draw_end = renderer->height - 1;

        // Draw the textured column
        if (hit && block_type != BLOCK_AIR) {
            // Get texture for this block type
            const uint16_t* texture = textures[block_type];
            if (!texture) {
                continue;  // Skip if no texture (shouldn't happen)
            }

            // Calculate texture X coordinate
            wall_x -= floorf(wall_x);  // Get fractional part
            int32_t tex_x = (int32_t)(wall_x * TEXTURE_SIZE);
            if (side == 0 && ray_dir_x > 0) tex_x = TEXTURE_SIZE - 1 - tex_x;
            if (side == 1 && ray_dir_z < 0) tex_x = TEXTURE_SIZE - 1 - tex_x;
            tex_x = tex_x & (TEXTURE_SIZE - 1);  // Clamp to 0-7

            // Calculate texture Y mapping (fixed-point 8.8)
            // We need to stretch the 8-pixel texture over the column height
            int32_t column_height = draw_end - draw_start + 1;
            if (column_height < 1) column_height = 1;  // Prevent division by zero

            // Use larger multiplier to avoid losing precision with small columns
            int32_t tex_y_step = (TEXTURE_SIZE << 12) / column_height;  // Fixed-point 12.4 for more precision
            int32_t tex_y_start = 0;  // Start from top of texture

            // Debug: Log first few wall calculations
            static int wall_debug_count = 0;
            if (wall_debug_count < 5) {
                ESP_LOGI(TAG, "WALL_DEBUG #%d: col_x=%d, dist=%.2f, line_height=%d", wall_debug_count, x, perp_wall_dist, line_height);
                ESP_LOGI(TAG, "  draw_start=%d, draw_end=%d, column_height=%d", draw_start, draw_end, column_height);
                ESP_LOGI(TAG, "  TEXTURE_SIZE=%d, (TEXTURE_SIZE << 12)=%d", TEXTURE_SIZE, TEXTURE_SIZE << 12);
                ESP_LOGI(TAG, "  tex_y_step=%d (0x%x), tex_y_start=%d", tex_y_step, tex_y_step, tex_y_start);
                ESP_LOGI(TAG, "  Expected: Step should map %d pixels to 8 texture rows", column_height);
                wall_debug_count++;
            }

            // Calculate shading (different sides have different brightness)
            float shade_factor;
            if (side == 0) {
                shade_factor = 0.85f;  // X-facing walls
            } else if (side == 1) {
                shade_factor = 0.7f;   // Z-facing walls
            } else {
                shade_factor = 1.0f;   // Y-facing walls (ceilings/floors) - brightest
            }

            // Calculate fog
            float fog_factor = 0.0f;
            if (perp_wall_dist > 10.0f) {
                fog_factor = (perp_wall_dist - 10.0f) / 20.0f;
                if (fog_factor > 1.0f) fog_factor = 1.0f;
            }

            // Draw textured column
            draw_textured_column(renderer, x, draw_start, draw_end,
                               tex_x, texture, tex_y_start, tex_y_step,
                               shade_factor, COLOR_SKY, fog_factor);
            walls_drawn++;
        }
    }

    // Debug log every 30 frames (only log if walls were drawn)
    static int32_t frame_count = 0;
    if (++frame_count % 30 == 0 && walls_drawn > 0) {
        ESP_LOGI(TAG, "Rendered %d walls/%d cols", walls_drawn, (end_col - start_col));
    }
}

/**
 * @brief Render wireframe view of all voxels (forward declaration)
 */
static void renderer_render_wireframe(renderer_t* renderer);

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
        // Normal textured rendering
        renderer_render_columns(renderer, 0, renderer->width);
    }
}

const uint16_t* renderer_get_framebuffer(const renderer_t* renderer) {
    if (!renderer) {
        return NULL;
    }
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
