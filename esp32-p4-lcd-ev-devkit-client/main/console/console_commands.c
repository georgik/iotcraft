/**
 * @file console_commands.c
 * @brief Built-in console commands implementation
 */

#include "console_commands.h"
#include "console.h"
#include <stdlib.h>
#include <stddef.h>

// Platform-specific includes
#if __DESKTOP_BUILD__
    #include "raylib.h"
#else
    #include "esp_system.h"
    #include "esp_heap_caps.h"
#endif

#define TAG "ConsoleCmd"

//=============================================================================
// Built-in Commands
//=============================================================================

/**
 * @brief Show available commands or help for specific command
 */
void cmd_help(const char* args) {
    if (args[0] == '\0') {
        // Show all commands
        console_log(LOG_LEVEL_INFO, "CONSOLE", "Available commands:");
        console_log(LOG_LEVEL_INFO, "CONSOLE", "  help [cmd]  - Show help (all commands or specific)");
        console_log(LOG_LEVEL_INFO, "CONSOLE", "  clear       - Clear console log");
        console_log(LOG_LEVEL_INFO, "CONSOLE", "  status      - Show system status");
        console_log(LOG_LEVEL_INFO, "CONSOLE", "  quit        - Exit application");
    } else {
        // TODO: Show help for specific command
        console_log(LOG_LEVEL_INFO, "CONSOLE", "No specific help available yet");
    }
}

/**
 * @brief Clear console log
 */
void cmd_clear(const char* args) {
    (void)args;  // Unused
    console_clear();
    console_log(LOG_LEVEL_INFO, "CONSOLE", "Console cleared");
}

/**
 * @brief Show system status
 */
void cmd_status(const char* args) {
    (void)args;  // Unused

    #if __DESKTOP_BUILD__
        int fps = GetFPS();
        int screen_w = GetScreenWidth();
        int screen_h = GetScreenHeight();

        console_log(LOG_LEVEL_INFO, "CONSOLE", "=== System Status ===");
        console_log(LOG_LEVEL_INFO, "CONSOLE", "FPS: %d", fps);
        console_log(LOG_LEVEL_INFO, "CONSOLE", "Resolution: %dx%d", screen_w, screen_h);
        console_log(LOG_LEVEL_INFO, "CONSOLE", "Frame time: %.2f ms", 1000.0f / fps);
    #else
        // ESP32-P4 status
        console_log(LOG_LEVEL_INFO, "CONSOLE", "=== System Status ===");
        console_log(LOG_LEVEL_INFO, "CONSOLE", "Chip: ESP32-P4");
        console_log(LOG_LEVEL_INFO, "CONSOLE", "Free heap: %d bytes", esp_get_free_heap_size());
        console_log(LOG_LEVEL_INFO, "CONSOLE", "Min free heap: %d bytes", esp_get_minimum_free_heap_size());
    #endif
}

/**
 * @brief Quit application
 */
void cmd_quit(const char* args) {
    (void)args;  // Unused
    console_log(LOG_LEVEL_INFO, "CONSOLE", "Exiting...");
    #if __DESKTOP_BUILD__
        // Request graceful shutdown
        exit(0);
    #else
        // ESP32: Restart or deep sleep
        esp_restart();
    #endif
}

//=============================================================================
// Registration
//=============================================================================

void console_register_builtin_commands(void) {
    console_register_command("help", cmd_help, "Show available commands");
    console_register_command("clear", cmd_clear, "Clear console log");
    console_register_command("status", cmd_status, "Show system status");
    console_register_command("quit", cmd_quit, "Exit application");
}
