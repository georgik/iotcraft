/**
 * @file console_commands.h
 * @brief Built-in console commands
 */

#ifndef CONSOLE_COMMANDS_H
#define CONSOLE_COMMANDS_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Register all built-in console commands
 */
void console_register_builtin_commands(void);

// Built-in command functions
void cmd_help(const char* args);
void cmd_clear(const char* args);
void cmd_status(const char* args);
void cmd_quit(const char* args);

#ifdef __cplusplus
}
#endif

#endif // CONSOLE_COMMANDS_H
