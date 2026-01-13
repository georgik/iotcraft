/**
 * @file cli_options.c
 * @brief Command-line interface options implementation
 */

#include "cli_options.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

int cli_parse_options(int argc, char* argv[], cli_options_t* options)
{
    // Set defaults
    memset(options, 0, sizeof(cli_options_t));
    options->duration_seconds = 5;
    options->width = 320;
    options->height = 240;
    options->verbose = false;
    options->interactive = false;

    // Camera defaults (use NaN to indicate "not set")
    options->cam_x = NAN;
    options->cam_y = NAN;
    options->cam_z = NAN;
    options->cam_yaw = NAN;
    options->cam_pitch = NAN;

    // Parse arguments
    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--help") == 0 || strcmp(argv[i], "-h") == 0) {
            cli_print_usage(argv[0]);
            return -1;
        }
        else if (strcmp(argv[i], "--interactive") == 0 || strcmp(argv[i], "-i") == 0) {
            options->interactive = true;
        }
        else if (strcmp(argv[i], "--chessboard") == 0) {
            options->chessboard = true;
        }
        else if (strcmp(argv[i], "--cam-pos") == 0) {
            if (i + 3 >= argc) {
                fprintf(stderr, "Error: --cam-pos requires X Y Z arguments\n");
                return -1;
            }
            options->cam_x = atof(argv[++i]);
            options->cam_y = atof(argv[++i]);
            options->cam_z = atof(argv[++i]);
        }
        else if (strcmp(argv[i], "--cam-rot") == 0) {
            if (i + 2 >= argc) {
                fprintf(stderr, "Error: --cam-rot requires YAW PITCH arguments\n");
                return -1;
            }
            options->cam_yaw = atof(argv[++i]);
            options->cam_pitch = atof(argv[++i]);
        }
        else if (strcmp(argv[i], "--screenshot") == 0) {
            if (i + 1 >= argc) {
                fprintf(stderr, "Error: --screenshot requires a filename argument\n");
                return -1;
            }
            options->screenshot_mode = true;
            strncpy(options->screenshot_path, argv[++i], sizeof(options->screenshot_path) - 1);
        }
        else if (strcmp(argv[i], "--duration") == 0) {
            if (i + 1 >= argc) {
                fprintf(stderr, "Error: --duration requires a seconds argument\n");
                return -1;
            }
            options->duration_seconds = atoi(argv[++i]);
            if (options->duration_seconds < 1) {
                fprintf(stderr, "Error: duration must be at least 1 second\n");
                return -1;
            }
        }
        else if (strcmp(argv[i], "--headless") == 0) {
            options->headless = true;
        }
        else if (strcmp(argv[i], "--verbose") == 0 || strcmp(argv[i], "-v") == 0) {
            options->verbose = true;
        }
        else if (strcmp(argv[i], "--size") == 0) {
            if (i + 2 >= argc) {
                fprintf(stderr, "Error: --size requires WIDTH and HEIGHT arguments\n");
                return -1;
            }
            options->width = atoi(argv[++i]);
            options->height = atoi(argv[++i]);
            if (options->width <= 0 || options->height <= 0) {
                fprintf(stderr, "Error: size must be positive\n");
                return -1;
            }
        }
        else {
            fprintf(stderr, "Error: Unknown option '%s'\n", argv[i]);
            cli_print_usage(argv[0]);
            return -1;
        }
    }

    return 0;
}

void cli_print_usage(const char* program_name)
{
    printf("\n");
    printf("╔════════════════════════════════════════════════════════════╗\n");
    printf("║           IotCraft Desktop Simulator                        ║\n");
    printf("║                                                            ║\n");
    printf("║  Usage:                                                   ║\n");
    printf("║    %s [OPTIONS]                                     ║\n", program_name);
    printf("║                                                            ║\n");
    printf("║  Options:                                                  ║\n");
    printf("║    -i, --interactive    Enable keyboard controls            ║\n");
    printf("║    --chessboard         Enable chessboard test pattern      ║\n");
    printf("║    --cam-pos X Y Z      Set camera position                ║\n");
    printf("║    --cam-rot YAW PITCH  Set camera rotation (radians)      ║\n");
    printf("║    --screenshot FILE    Save screenshot and exit           ║\n");
    printf("║    --duration SECONDS   Run for N seconds (default: 5)      ║\n");
    printf("║    --headless           Don't show window (for screenshots) ║\n");
    printf("║    --size WIDTH HEIGHT  Resolution (default: 320x240)       ║\n");
    printf("║    --verbose, -v        Enable verbose logging             ║\n");
    printf("║    --help, -h           Show this help message             ║\n");
    printf("║                                                            ║\n");
    printf("║  Examples:                                                ║\n");
    printf("║    %s -i                                         ║\n", program_name);
    printf("║        Interactive mode (WASD+Arrows)                     ║\n");
    printf("║                                                            ║\n");
    printf("║    %s --cam-pos 0 2 10 --cam-rot 1.57 0              ║\n", program_name);
    printf("║        Set camera to specific position and angle         ║\n");
    printf("║                                                            ║\n");
    printf("║    %s --screenshot output.png                     ║\n", program_name);
    printf("║        Run for 5 seconds, save screenshot, exit           ║\n");
    printf("║                                                            ║\n");
    printf("║    %s --size 640x480 --verbose                    ║\n", program_name);
    printf("║        High-resolution mode with debug output              ║\n");
    printf("║                                                            ║\n");
    printf("╚════════════════════════════════════════════════════════════╝\n");
    printf("\n");
}

void cli_print_defaults(void)
{
    printf("Default options:\n");
    printf("  Resolution:   320x240\n");
    printf("  Duration:     5 seconds\n");
    printf("  Interactive:  disabled\n");
    printf("  Chessboard:   disabled\n");
    printf("  Camera:       auto-positioned\n");
    printf("  Screenshot:   disabled\n");
    printf("  Headless:     disabled\n");
    printf("  Verbose:      disabled\n");
    printf("\n");
    printf("Keyboard controls (when -i used):\n");
    printf("  WASD          Move forward/left/backward/right\n");
    printf("  Space/Shift   Move up/down\n");
    printf("  Left/Right    Rotate yaw\n");
    printf("  Up/Down       Rotate pitch\n");
    printf("  ESC           Exit\n");
    printf("\n");
}
