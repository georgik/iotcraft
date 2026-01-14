/**
 * @file cli_options.h
 * @brief Command-line interface options for desktop simulator
 */

#ifndef CLI_OPTIONS_H
#define CLI_OPTIONS_H

#include <stdbool.h>

typedef struct {
    bool screenshot_mode;
    char screenshot_path[256];
    int duration_seconds;
    bool headless;
    bool verbose;
    bool chessboard;
    bool interactive;
    bool wireframe;
    int width;
    int height;

    // Camera position
    float cam_x;
    float cam_y;
    float cam_z;
    float cam_yaw;
    float cam_pitch;
} cli_options_t;

/**
 * @brief Parse command-line arguments
 * @param argc Argument count
 * @param argv Argument values
 * @param options Output structure for parsed options
 * @return 0 on success, -1 on error
 */
int cli_parse_options(int argc, char* argv[], cli_options_t* options);

/**
 * @brief Print usage information
 * @param program_name Program name (argv[0])
 */
void cli_print_usage(const char* program_name);

/**
 * @brief Print default options
 */
void cli_print_defaults(void);

#endif // CLI_OPTIONS_H
