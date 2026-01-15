/**
 * @file world.c
 * @brief Voxel world management implementation
 */

#include "world.h"
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <esp_log.h>

static const char* TAG = "World";

#define HASH_TABLE_SIZE 4096  // Power of 2 for fast modulo
#define MAX_VOXELS 8192       // Maximum voxels in world
#define WORLDSIZE 256         // Heightmap covers -128 to +127
#define WORLD_OFFSET 128      // Offset to map negative coords to 0..255

// Spatial hash function for 3D coordinates
static uint32_t hash_position(int32_t x, int32_t y, int32_t z) {
    // Use well-known spatial hash constants
    uint32_t h1 = (uint32_t)(x * 73856093);
    uint32_t h2 = (uint32_t)(y * 19349663);
    uint32_t h3 = (uint32_t)(z * 83492791);
    return (h1 ^ h2 ^ h3) & (HASH_TABLE_SIZE - 1);
}

// Hash table entry
typedef struct hash_entry {
    voxel_t voxel;
    struct hash_entry* next;
    bool used;
} hash_entry_t;

static hash_entry_t* g_hash_table[HASH_TABLE_SIZE] = {NULL};

bool world_init(voxel_world_t* world) {
    if (!world) {
        ESP_LOGE(TAG, "Null world pointer");
        return false;
    }

    // Initialize hash table to NULL
    for (int i = 0; i < HASH_TABLE_SIZE; i++) {
        g_hash_table[i] = NULL;
    }

    // Allocate heightmap (256x256 = 65KB in PSRAM)
    world->heightmap = (int32_t*)calloc(WORLDSIZE * WORLDSIZE, sizeof(int32_t));
    if (!world->heightmap) {
        ESP_LOGE(TAG, "Failed to allocate heightmap");
        return false;
    }

    // Initialize heightmap to -1 (no blocks)
    for (int32_t i = 0; i < WORLDSIZE * WORLDSIZE; i++) {
        world->heightmap[i] = -1;
    }

    world->count = 0;
    world->capacity = MAX_VOXELS;
    world->world_size = WORLDSIZE;
    world->world_offset = WORLD_OFFSET;

    ESP_LOGI(TAG, "World initialized with hash table size %d and 256x256 heightmap", HASH_TABLE_SIZE);
    return true;
}

void world_free(voxel_world_t* world) {
    if (!world) {
        return;
    }

    // Free heightmap
    if (world->heightmap) {
        free(world->heightmap);
        world->heightmap = NULL;
    }

    // Free hash table entries
    for (int i = 0; i < HASH_TABLE_SIZE; i++) {
        hash_entry_t* entry = g_hash_table[i];
        while (entry) {
            hash_entry_t* next = entry->next;
            free(entry);
            entry = next;
        }
        g_hash_table[i] = NULL;
    }

    world->count = 0;

    ESP_LOGI(TAG, "World freed");
}

block_type_t world_get_block(const voxel_world_t* world, int32_t x, int32_t y, int32_t z) {
    if (!world) {
        return BLOCK_AIR;
    }

    // Hash table lookup - O(1) average case
    uint32_t hash = hash_position(x, y, z);
    hash_entry_t* entry = g_hash_table[hash];

    while (entry) {
        if (entry->used &&
            entry->voxel.x == x &&
            entry->voxel.y == y &&
            entry->voxel.z == z) {
            return entry->voxel.type;
        }
        entry = entry->next;
    }

    return BLOCK_AIR;
}

bool world_set_block(voxel_world_t* world, int32_t x, int32_t y, int32_t z, block_type_t type) {
    if (!world) {
        return false;
    }

    uint32_t hash = hash_position(x, y, z);
    hash_entry_t* entry = g_hash_table[hash];
    hash_entry_t* prev_entry = NULL;

    // Search for existing entry at this position
    while (entry) {
        if (entry->used &&
            entry->voxel.x == x &&
            entry->voxel.y == y &&
            entry->voxel.z == z) {

            // Found existing block
            if (type == BLOCK_AIR) {
                // Remove block: unlink from chain
                if (prev_entry) {
                    prev_entry->next = entry->next;
                } else {
                    g_hash_table[hash] = entry->next;
                }
                free(entry);
                world->count--;

                // Update heightmap for this (x,z) column
                world_update_heightmap(world, x, z);
            } else {
                // Update existing block type
                entry->voxel.type = type;

                // Update heightmap if this block is higher than current
                int32_t hm_x = x + world->world_offset;
                int32_t hm_z = z + world->world_offset;
                if (hm_x >= 0 && hm_x < world->world_size &&
                    hm_z >= 0 && hm_z < world->world_size) {
                    int32_t idx = hm_z * world->world_size + hm_x;
                    if (y > world->heightmap[idx]) {
                        world->heightmap[idx] = y;
                    }
                }
            }
            return true;
        }
        prev_entry = entry;
        entry = entry->next;
    }

    // Add new block
    if (type == BLOCK_AIR) {
        return true;  // Nothing to add
    }

    // Check capacity
    if (world->count >= MAX_VOXELS) {
        ESP_LOGE(TAG, "World at maximum capacity");
        return false;
    }

    // Create new hash entry
    hash_entry_t* new_entry = (hash_entry_t*)malloc(sizeof(hash_entry_t));
    if (!new_entry) {
        ESP_LOGE(TAG, "Failed to allocate hash entry");
        return false;
    }

    new_entry->voxel.x = x;
    new_entry->voxel.y = y;
    new_entry->voxel.z = z;
    new_entry->voxel.type = type;
    new_entry->used = true;
    new_entry->next = g_hash_table[hash];
    g_hash_table[hash] = new_entry;

    // Update heightmap if this block is higher than current
    int32_t hm_x = x + world->world_offset;
    int32_t hm_z = z + world->world_offset;
    if (hm_x >= 0 && hm_x < world->world_size &&
        hm_z >= 0 && hm_z < world->world_size) {
        int32_t idx = hm_z * world->world_size + hm_x;
        if (y > world->heightmap[idx]) {
            world->heightmap[idx] = y;
        }
    }

    world->count++;
    return true;
}

void world_generate_test_terrain(voxel_world_t* world) {
    if (!world) {
        return;
    }

    ESP_LOGI(TAG, "Generating test terrain...");

    // Generate a flat grass plane at y=0, size 20x20
    int32_t size = 10;
    int32_t height = 0;

    for (int32_t x = -size; x <= size; x++) {
        for (int32_t z = -size; z <= size; z++) {
            world_set_block(world, x, height, z, BLOCK_GRASS);
        }
    }

    // Add some blocks for visual reference
    world_set_block(world, 0, 1, 0, BLOCK_STONE);
    world_set_block(world, 1, 1, 0, BLOCK_DIRT);
    world_set_block(world, 0, 1, 1, BLOCK_QUARTZ);

    ESP_LOGI(TAG, "Test terrain generated: %d blocks", world->count);
}

bool world_get_target_block(const voxel_world_t* world, const camera_t* camera,
                             int32_t* target_x, int32_t* target_y, int32_t* target_z)
{
    if (!world || !camera || !target_x || !target_y || !target_z) {
        return false;
    }

    // Simple raycast along camera direction to find targeted block
    // Bevy-compatible coordinate system: X+ = right, Y+ = up, Z- = forward
    float ray_dir_x = -sinf(camera->yaw) * cosf(camera->pitch);
    float ray_dir_y = sinf(camera->pitch);
    float ray_dir_z = -cosf(camera->yaw) * cosf(camera->pitch);

    // Step along ray (simple DDA-like approach)
    float max_distance = 10.0f;  // Maximum reach distance
    float step_size = 0.1f;
    int32_t last_x = (int32_t)floorf(camera->x);
    int32_t last_y = (int32_t)floorf(camera->y);
    int32_t last_z = (int32_t)floorf(camera->z);

    for (float d = 0.0f; d < max_distance; d += step_size) {
        float pos_x = camera->x + ray_dir_x * d;
        float pos_y = camera->y + ray_dir_y * d;
        float pos_z = camera->z + ray_dir_z * d;

        int32_t block_x = (int32_t)floorf(pos_x);
        int32_t block_y = (int32_t)floorf(pos_y);
        int32_t block_z = (int32_t)floorf(pos_z);

        // Check if we've entered a new block
        if (block_x != last_x || block_y != last_y || block_z != last_z) {
            block_type_t type = world_get_block(world, block_x, block_y, block_z);
            if (type != BLOCK_AIR) {
                *target_x = block_x;
                *target_y = block_y;
                *target_z = block_z;
                return true;
            }
            last_x = block_x;
            last_y = block_y;
            last_z = block_z;
        }
    }

    return false;
}

bool world_get_place_position(const voxel_world_t* world, const camera_t* camera,
                               int32_t* place_x, int32_t* place_y, int32_t* place_z)
{
    if (!world || !camera || !place_x || !place_y || !place_z) {
        return false;
    }

    // First, find targeted block
    int32_t target_x, target_y, target_z;
    if (!world_get_target_block(world, camera, &target_x, &target_y, &target_z)) {
        // If no block targeted, place in front of camera
        // Bevy-compatible coordinate system
        float ray_dir_x = -sinf(camera->yaw) * cosf(camera->pitch);
        float ray_dir_y = sinf(camera->pitch);
        float ray_dir_z = -cosf(camera->yaw) * cosf(camera->pitch);

        *place_x = (int32_t)floorf(camera->x + ray_dir_x * 3.0f);
        *place_y = (int32_t)floorf(camera->y + ray_dir_y * 3.0f);
        *place_z = (int32_t)floorf(camera->z + ray_dir_z * 3.0f);
        return true;
    }

    // Find the empty position before the targeted block
    // Calculate direction from camera to target
    float dir_x = target_x + 0.5f - camera->x;
    float dir_y = target_y + 0.5f - camera->y;
    float dir_z = target_z + 0.5f - camera->z;

    // Normalize
    float len = sqrtf(dir_x * dir_x + dir_y * dir_y + dir_z * dir_z);
    if (len > 0.0f) {
        dir_x /= len;
        dir_y /= len;
        dir_z /= len;
    }

    // Step back from target block to find empty space
    for (float d = 0.0f; d < 2.0f; d += 0.1f) {
        int32_t test_x = (int32_t)floorf(target_x + 0.5f - dir_x * d);
        int32_t test_y = (int32_t)floorf(target_y + 0.5f - dir_y * d);
        int32_t test_z = (int32_t)floorf(target_z + 0.5f - dir_z * d);

        if (world_get_block(world, test_x, test_y, test_z) == BLOCK_AIR) {
            *place_x = test_x;
            *place_y = test_y;
            *place_z = test_z;
            return true;
        }
    }

    return false;
}
int32_t world_get_height(const voxel_world_t* world, int32_t x, int32_t z) {
    if (!world || !world->heightmap) {
        return -1;
    }

    // Map world coordinates to heightmap indices
    int32_t hm_x = x + world->world_offset;
    int32_t hm_z = z + world->world_offset;

    // Bounds check
    if (hm_x < 0 || hm_x >= world->world_size ||
        hm_z < 0 || hm_z >= world->world_size) {
        return -1;
    }

    return world->heightmap[hm_z * world->world_size + hm_x];
}

void world_update_heightmap(voxel_world_t* world, int32_t x, int32_t z) {
    if (!world || !world->heightmap) {
        return;
    }

    // Map world coordinates to heightmap indices
    int32_t hm_x = x + world->world_offset;
    int32_t hm_z = z + world->world_offset;

    // Bounds check
    if (hm_x < 0 || hm_x >= world->world_size ||
        hm_z < 0 || hm_z >= world->world_size) {
        return;
    }

    // Search all Y levels at this (x,z) to find highest block
    int32_t max_y = -1;
    for (int32_t y = 0; y < 15; y++) {
        if (world_get_block(world, x, y, z) != BLOCK_AIR) {
            max_y = y;
        }
    }

    world->heightmap[hm_z * world->world_size + hm_x] = max_y;
}
