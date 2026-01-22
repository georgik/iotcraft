/**
 * @file console.h
 * @brief Quake-style console overlay for debugging and user interaction
 *
 * Shared between desktop-light and ESP32-P4
 */

#ifndef CONSOLE_H
#define CONSOLE_H

#include <stdbool.h>
#include <stdint.h>
#include <stdarg.h>
#include <stdio.h>

#ifdef __cplusplus
extern "C" {
#endif

// Log levels
typedef enum {
    LOG_LEVEL_INFO,
    LOG_LEVEL_WARN,
    LOG_LEVEL_ERROR,
    LOG_LEVEL_DEBUG
} log_level_t;

// Console state
typedef struct {
    bool visible;
    float slide_progress;  // 0.0 = hidden, 1.0 = fully visible
    bool auto_hide;
    int scroll_offset;
    uint32_t last_log_count;
    uint32_t last_toggle_time;
    char input_buffer[256];
    int input_cursor;
} console_state_t;

/**
 * @brief Initialize console subsystem
 */
void console_init(void);

/**
 * @brief Shutdown console subsystem
 */
void console_shutdown(void);

/**
 * @brief Log a message to console
 * @param level Log level (INFO/WARN/ERROR/DEBUG)
 * @param tag Tag for the message (e.g., "Network", "Renderer")
 * @param format Printf-style format string
 * @param ... Format arguments
 */
void console_log(log_level_t level, const char* tag, const char* format, ...);

/**
 * @brief Clear all console log entries
 */
void console_clear(void);

/**
 * @brief Toggle console visibility (slide up/down)
 */
void console_toggle(void);

/**
 * @brief Show console (slide down)
 */
void console_show(void);

/**
 * @brief Hide console (slide up)
 */
void console_hide(void);

/**
 * @brief Check if console is visible
 * @return true if console is visible or sliding
 */
bool console_is_visible(void);

/**
 * @brief Set auto-hide flag
 * @param auto_hide If true, console will auto-hide after init
 */
void console_set_auto_hide(bool auto_hide);

/**
 * @brief Update console animation (call every frame)
 */
void console_update(void);

/**
 * @brief Render console overlay (call after rendering 3D scene)
 * @param framebuffer RGB565 framebuffer to draw to (NULL on desktop)
 * @param width Framebuffer width
 * @param height Framebuffer height
 */
void console_render(void* framebuffer, int width, int height);

/**
 * @brief Register a command
 * @param name Command name
 * @param func Command function
 * @param help Help text
 */
typedef void (*console_command_func_t)(const char* args);
void console_register_command(const char* name, console_command_func_t func, const char* help);

/**
 * @brief Execute a command string
 * @param command_line Command to execute (e.g., "help", "pos 1 2 3")
 */
void console_execute(const char* command_line);

/**
 * @brief Register all built-in commands
 */
void console_register_builtin_commands(void);

/**
 * @brief Draw text directly to framebuffer
 * @param text Text string to draw
 * @param x X position
 * @param y Y position
 * @param color Text color (RGB)
 * @param framebuffer RGB565 framebuffer to draw to
 * @param fb_width Framebuffer width
 * @param fb_height Framebuffer height
 */
void console_draw_text(const char* text, int x, int y, unsigned char r, unsigned char g, unsigned char b,
                       void* framebuffer, int fb_width, int fb_height);

#ifdef __cplusplus
}
#endif

#endif // CONSOLE_H
