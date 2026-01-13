/**
 * @file world_template.c
 * @brief World template parser implementation
 */

#include "world_template.h"
#include "world.h"
#include "camera.h"
#include "esp_log.h"
#include <string.h>
#include <stdlib.h>
#include <stdio.h>
#include <math.h>

static const char* TAG = "WorldTemplate";

// Embedded medieval world template
static const char medieval_template[] =
    "# Medieval World Template\n"
    "# Creates a medieval-themed world with castle, village, and forest\n"
    "\n"
    "# Set initial camera position for good overview\n"
    "tp -15 2 0\n"
    "look -45 -10\n"
    "\n"
    "# Create base terrain (smaller than default for more intimate feel)\n"
    "wall grass -30 0 -30 30 0 30\n"
    "\n"
    "# Build a castle in the center-north area\n"
    "# Castle foundation\n"
    "wall stone -8 1 -15 8 1 -8\n"
    "wall stone -8 2 -15 8 2 -8\n"
    "\n"
    "# Castle walls (outer perimeter)\n"
    "wall stone -10 3 -17 10 3 -17  # North wall\n"
    "wall stone -10 3 -6 10 3 -6    # South wall\n"
    "wall stone -10 3 -17 -10 3 -6  # West wall\n"
    "wall stone 10 3 -17 10 3 -6    # East wall\n"
    "\n"
    "# Castle towers at corners\n"
    "wall stone -10 4 -17 -8 8 -15  # Northwest tower\n"
    "wall stone 8 4 -17 10 8 -15    # Northeast tower\n"
    "wall stone -10 4 -8 -8 8 -6    # Southwest tower\n"
    "wall stone 8 4 -8 10 8 -6      # Southeast tower\n"
    "\n"
    "# Castle gate (opening in south wall)\n"
    "wall grass -2 3 -6 2 3 -6\n"
    "wall grass -2 4 -6 2 4 -6\n"
    "\n"
    "# Village houses scattered around\n"
    "# House 1 - East of castle\n"
    "wall stone 15 1 -5 19 1 -1\n"
    "wall stone 15 2 -5 19 2 -1\n"
    "wall stone 15 3 -5 19 3 -5  # Front wall\n"
    "wall stone 15 3 -1 19 3 -1  # Back wall\n"
    "wall stone 15 3 -5 15 3 -1  # Left wall\n"
    "wall stone 19 3 -5 19 3 -1  # Right wall\n"
    "place quartz_block 17 4 -3   # Roof accent\n"
    "\n"
    "# House 2 - Southwest area\n"
    "wall stone -18 1 8 -14 1 12\n"
    "wall stone -18 2 8 -14 2 12\n"
    "wall stone -18 3 8 -18 3 12  # Front wall\n"
    "wall stone -14 3 8 -14 3 12  # Back wall\n"
    "wall stone -18 3 8 -14 3 8   # Left wall\n"
    "wall stone -18 3 12 -14 3 12 # Right wall\n"
    "\n"
    "# House 3 - Southeast area\n"
    "wall stone 12 1 10 16 1 14\n"
    "wall stone 12 2 10 16 2 14\n"
    "wall stone 12 3 10 16 3 10  # Front wall\n"
    "wall stone 12 3 14 16 3 14  # Back wall\n"
    "wall stone 12 3 10 12 3 14  # Left wall\n"
    "wall stone 16 3 10 16 3 14  # Right wall\n"
    "\n"
    "# Forest area (using different heights for variety)\n"
    "# Tree stumps and logs\n"
    "place stone -25 1 -25\n"
    "place stone -23 1 -22\n"
    "place stone -20 1 -28\n"
    "place stone 22 1 -20\n"
    "place stone 25 1 -18\n"
    "place stone 28 1 -25\n"
    "\n"
    "# Hills for terrain variety\n"
    "wall dirt -25 1 15 -20 2 20\n"
    "wall grass -25 3 15 -20 3 20\n"
    "\n"
    "wall dirt 20 1 20 25 3 25\n"
    "wall grass 20 4 20 25 4 25\n"
    "\n"
    "# Medieval decorations\n"
    "place cyan_terracotta 0 1 -12  # Castle courtyard center\n"
    "place glass_pane 0 5 -12       # Castle banner pole\n"
    "\n"
    "# Small river\n"
    "wall water -5 1 25 5 1 28\n"
    "wall stone -6 1 24 6 1 24      # River bank\n"
    "wall stone -6 1 29 6 1 29      # River bank\n"
    "\n"
    "# Give medieval-appropriate starting items\n"
    "give stone 64\n"
    "give dirt 32\n"
    "give grass 32\n"
    "give quartz_block 24\n"
    "give glass_pane 16\n"
    "give water 12\n"
;

// Block type name to enum conversion
static block_type_t parse_block_type(const char* name) {
    if (strcmp(name, "grass") == 0) return BLOCK_GRASS;
    if (strcmp(name, "dirt") == 0) return BLOCK_DIRT;
    if (strcmp(name, "stone") == 0) return BLOCK_STONE;
    if (strcmp(name, "quartz_block") == 0) return BLOCK_QUARTZ;
    if (strcmp(name, "glass_pane") == 0) return BLOCK_GLASS;
    if (strcmp(name, "cyan_terracotta") == 0) return BLOCK_TERRACOTTA;
    if (strcmp(name, "water") == 0) return BLOCK_WATER;
    ESP_LOGW(TAG, "Unknown block type: %s", name);
    return BLOCK_GRASS;  // Default
}

// Execute a wall command
static bool execute_wall(char* args[], voxel_world_t* world) {
    // wall <type> <x1> <y1> <z1> <x2> <y2> <z2>
    if (args[1] == NULL || args[2] == NULL || args[3] == NULL ||
        args[4] == NULL || args[5] == NULL || args[6] == NULL || args[7] == NULL) {
        ESP_LOGE(TAG, "Invalid wall command syntax");
        return false;
    }

    block_type_t type = parse_block_type(args[1]);
    int32_t x1 = atoi(args[2]);
    int32_t y1 = atoi(args[3]);
    int32_t z1 = atoi(args[4]);
    int32_t x2 = atoi(args[5]);
    int32_t y2 = atoi(args[6]);
    int32_t z2 = atoi(args[7]);

    // Fill rectangular prism
    for (int32_t x = x1; x <= x2; x++) {
        for (int32_t y = y1; y <= y2; y++) {
            for (int32_t z = z1; z <= z2; z++) {
                world_set_block(world, x, y, z, type);
            }
        }
    }

    return true;
}

// Execute a place command
static bool execute_place(char* args[], voxel_world_t* world) {
    // place <type> <x> <y> <z>
    if (args[1] == NULL || args[2] == NULL || args[3] == NULL || args[4] == NULL) {
        ESP_LOGE(TAG, "Invalid place command syntax");
        return false;
    }

    block_type_t type = parse_block_type(args[1]);
    int32_t x = atoi(args[2]);
    int32_t y = atoi(args[3]);
    int32_t z = atoi(args[4]);

    return world_set_block(world, x, y, z, type);
}

// Execute tp (teleport) command
static bool execute_tp(char* args[], camera_t* camera) {
    // tp <x> <y> <z>
    if (!camera || args[1] == NULL || args[2] == NULL || args[3] == NULL) {
        return false;
    }

    camera->x = (float)atoi(args[1]);
    camera->y = (float)atoi(args[2]);
    camera->z = (float)atoi(args[3]);

    return true;
}

// Execute look command
static bool execute_look(char* args[], camera_t* camera) {
    // look <yaw> <pitch> (angles in degrees)
    if (!camera || args[1] == NULL || args[2] == NULL) {
        return false;
    }

    // Convert degrees to radians
    camera->yaw = (float)atoi(args[1]) * (M_PI / 180.0f);
    camera->pitch = (float)atoi(args[2]) * (M_PI / 180.0f);

    return true;
}

bool world_load_template(const char* template_data, voxel_world_t* world, camera_t* camera) {
    if (!template_data || !world) {
        ESP_LOGE(TAG, "Invalid parameters");
        return false;
    }

    ESP_LOGI(TAG, "Loading world template...");

    // Make a copy of template data since we'll modify it
    char* template_copy = strdup(template_data);
    if (!template_copy) {
        ESP_LOGE(TAG, "Failed to allocate template copy");
        return false;
    }

    char* line = template_copy;
    char* next_line;
    int line_num = 0;
    int commands_executed = 0;

    while (line) {
        line_num++;

        // Find end of line
        next_line = strchr(line, '\n');
        if (next_line) {
            *next_line = '\0';
            next_line++;
        }

        // Trim trailing carriage return
        char* cr = strchr(line, '\r');
        if (cr) *cr = '\0';

        // Skip empty lines and comments
        if (line[0] == '\0' || line[0] == '#') {
            line = next_line;
            continue;
        }

        // Split line into tokens
        char* args[16] = {0};
        int arg_count = 0;
        char* token = strtok(line, " \t");
        while (token && arg_count < 16) {
            args[arg_count++] = token;
            token = strtok(NULL, " \t");
        }

        if (arg_count == 0) {
            line = next_line;
            continue;
        }

        // Parse commands
        bool success = true;

        if (strcmp(args[0], "wall") == 0) {
            success = execute_wall(args, world);
        } else if (strcmp(args[0], "place") == 0) {
            success = execute_place(args, world);
        } else if (strcmp(args[0], "tp") == 0 && camera) {
            success = execute_tp(args, camera);
        } else if (strcmp(args[0], "look") == 0 && camera) {
            success = execute_look(args, camera);
        } else if (strcmp(args[0], "give") == 0) {
            // Skip inventory commands for now
            success = true;
        } else {
            ESP_LOGW(TAG, "Unknown command at line %d: %s", line_num, args[0]);
        }

        if (success) {
            commands_executed++;
        }

        line = next_line;
    }

    free(template_copy);

    ESP_LOGI(TAG, "World template loaded: %d commands, %d blocks",
             commands_executed, world->count);
    return true;
}

bool world_load_medieval_template(voxel_world_t* world, camera_t* camera) {
    return world_load_template(medieval_template, world, camera);
}
