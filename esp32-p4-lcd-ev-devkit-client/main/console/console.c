/**
 * @file console.c
 * @brief Quake-style console overlay implementation
 *
 * Features:
 * - Circular log buffer (1000 lines)
 * - Smooth slide animation (150ms)
 * - Command system with help
 * - Color-coded log levels
 * - Shared between desktop and ESP32
 */

#include "console.h"
#include <string.h>
#include <stdio.h>
#include <math.h>

// Platform-specific includes
#if __DESKTOP_BUILD__
    #include "raylib.h"
    static inline uint32_t get_tick_ms(void) {
        return (uint32_t)(GetTime() * 1000.0);
    }
#else
    #include "esp_log.h"
    #include "esp_timer.h"
    #include "freertos/FreeRTOS.h"
    #include "freertos/task.h"
    #include "esp_heap_caps.h"
    #define get_tick_ms() (xTaskGetTickCount() * portTICK_PERIOD_MS)

    // ESP32-P4 color definition (RGB565 or RGBA)
    typedef struct {
        uint8_t r;
        uint8_t g;
        uint8_t b;
        uint8_t a;
    } Color;
#endif

#define TAG "Console"

// Log buffer configuration
// Reduced for ESP32-P4 to conserve internal RAM
#if CONFIG_IDF_TARGET_ESP32P4
    #define LOG_BUFFER_SIZE 1000   // Full buffer size, but allocated in PSRAM
#else
    #define LOG_BUFFER_SIZE 1000  // Full buffer for desktop
#endif
#define LOG_LINE_LENGTH 256
#define VISIBLE_LINES 25

// Log entry
typedef struct {
    char text[LOG_LINE_LENGTH];
    log_level_t level;
    char tag[32];
    uint32_t timestamp;
} log_entry_t;

// Command registration
#define MAX_COMMANDS 32
typedef struct {
    char name[32];
    console_command_func_t func;
    char help[128];
} command_t;

// Static state
#if CONFIG_IDF_TARGET_ESP32P4
// Allocate console buffers in PSRAM to save internal RAM
static log_entry_t* log_buffer = NULL;  // Dynamically allocated from PSRAM
static int log_write_index = 0;
static int log_count = 0;

static command_t* commands = NULL;  // Also in PSRAM
static int command_count = 0;

static console_state_t console = {
    .visible = false,
    .slide_progress = 0.0f,
    .auto_hide = false,
    .scroll_offset = 0,
    .last_log_count = 0,
    .last_toggle_time = 0,
    .input_buffer = "",
    .input_cursor = 0,
};

// Simple 5x7 bitmap font for digits (same as hello.c)
static const char digit_font[10][35] = {
    // 0
    {0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0},
    // 1
    {0,0,1,0,0, 0,1,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,1,1,1,0},
    // 2
    {0,1,1,1,0, 1,0,0,0,1, 0,0,0,0,1, 0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0, 1,1,1,1,1},
    // 3
    {0,1,1,1,0, 1,0,0,0,1, 0,0,0,0,1, 0,0,1,1,0, 0,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0},
    // 4
    {0,0,0,1,0, 0,0,1,1,0, 0,1,0,1,0, 1,0,0,1,0, 1,1,1,1,1, 0,0,0,1,0, 0,0,0,1,0},
    // 5
    {1,1,1,1,1, 1,0,0,0,0, 1,1,1,1,0, 0,0,0,0,1, 0,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0},
    // 6
    {0,1,1,1,0, 1,0,0,0,0, 1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0},
    // 7
    {1,1,1,1,1, 0,0,0,0,1, 0,0,0,1,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0},
    // 8
    {0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0},
    // 9
    {0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,1, 0,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0}
};

// Simple 5x7 bitmap font for uppercase letters
static const char letter_font[26][35] = {
    // A
    {0,0,1,0,0, 0,1,0,1,0, 1,0,0,0,1, 1,1,1,1,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1},
    // B
    {1,1,1,1,0, 1,0,0,0,1, 1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,0},
    // C
    {0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,1, 0,1,1,1,0},
    // D
    {1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,0},
    // E
    {1,1,1,1,1, 1,0,0,0,0, 1,1,1,1,0, 1,0,0,0,0, 1,0,0,0,0, 1,1,1,1,1},
    // F
    {1,1,1,1,1, 1,0,0,0,0, 1,1,1,1,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0},
    // G
    {0,1,1,1,1, 1,0,0,0,0, 1,0,1,1,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0},
    // H
    {1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1},
    // I
    {1,1,1,1,1, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 1,1,1,1,1},
    // J
    {0,0,1,1,1, 0,0,0,1,0, 0,0,0,1,0, 0,0,0,1,0, 1,0,0,1,0, 0,1,1,0,0},
    // K
    {1,0,0,1,0, 1,0,1,0,0, 1,1,0,0,0, 1,0,1,0,0, 1,0,0,1,0, 1,0,0,0,1},
    // L
    {1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0, 1,0,0,0,0, 1,1,1,1,1},
    // M
    {1,0,0,0,1, 1,1,0,1,1, 1,0,1,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1},
    // N
    {1,0,0,0,1, 1,1,0,0,1, 1,0,1,0,1, 1,0,0,1,1, 1,0,0,0,1, 1,0,0,0,1},
    // O
    {0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0},
    // P
    {1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,0, 1,0,0,0,0, 1,0,0,0,0},
    // Q
    {0,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,0,1,0,1, 1,0,0,1,0, 0,1,1,0,1},
    // R
    {1,1,1,1,0, 1,0,0,0,1, 1,0,0,0,1, 1,1,1,1,0, 1,0,1,0,0, 1,0,0,0,1},
    // S
    {0,1,1,1,1, 1,0,0,0,0, 0,1,1,1,0, 0,0,0,0,1, 1,0,0,0,0, 1,1,1,1,0},
    // T
    {1,1,1,1,1, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0},
    // U
    {1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,1,1,0},
    // V
    {1,0,0,0,1, 1,0,0,0,1, 1,0,0,0,1, 0,1,0,1,0, 0,1,0,1,0, 0,0,1,0,0},
    // W
    {1,0,0,0,1, 1,0,0,0,1, 1,0,1,0,1, 1,0,1,0,1, 1,1,0,1,1, 1,0,0,0,1},
    // X
    {1,0,0,0,1, 0,1,0,1,0, 0,0,1,0,0, 0,0,1,0,0, 0,1,0,1,0, 1,0,0,0,1},
    // Y
    {1,0,0,0,1, 0,1,0,1,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0, 0,0,1,0,0},
    // Z
    {1,1,1,1,1, 0,0,0,0,1, 0,0,0,1,0, 0,0,1,0,0, 0,1,0,0,0, 1,1,1,1,1}
};
#else
// Desktop can use static allocation
static log_entry_t log_buffer[LOG_BUFFER_SIZE];
static int log_write_index = 0;
static int log_count = 0;

static command_t commands[MAX_COMMANDS];
static int command_count = 0;

static console_state_t console = {
    .visible = false,
    .slide_progress = 0.0f,
    .auto_hide = false,
    .scroll_offset = 0,
    .last_log_count = 0,
    .last_toggle_time = 0,
    .input_buffer = "",
    .input_cursor = 0,
};
#endif

// Forward declarations
static Color get_color_for_level(log_level_t level);
static void render_text(const char* text, int x, int y, Color color,
                       uint16_t* fb, int fb_w, int fb_h);

//=============================================================================
// Initialization
//=============================================================================

void console_init(void) {
#if CONFIG_IDF_TARGET_ESP32P4
    // Allocate console buffers from PSRAM (heap_caps_malloc with MALLOC_CAP_SPIRAM)
    // This saves ~300KB of valuable internal RAM
    if (log_buffer == NULL) {
        log_buffer = (log_entry_t*)heap_caps_malloc(LOG_BUFFER_SIZE * sizeof(log_entry_t), MALLOC_CAP_SPIRAM);
        if (log_buffer == NULL) {
            // Fallback to regular heap if PSRAM allocation fails
            log_buffer = (log_entry_t*)malloc(LOG_BUFFER_SIZE * sizeof(log_entry_t));
        }
    }
    if (commands == NULL) {
        commands = (command_t*)heap_caps_malloc(MAX_COMMANDS * sizeof(command_t), MALLOC_CAP_SPIRAM);
        if (commands == NULL) {
            commands = (command_t*)malloc(MAX_COMMANDS * sizeof(command_t));
        }
    }

    if (log_buffer) memset(log_buffer, 0, LOG_BUFFER_SIZE * sizeof(log_entry_t));
    if (commands) memset(commands, 0, MAX_COMMANDS * sizeof(command_t));
#else
    memset(log_buffer, 0, sizeof(log_buffer));
    memset(commands, 0, sizeof(commands));
#endif

    log_write_index = 0;
    log_count = 0;

    console.visible = false;
    console.slide_progress = 0.0f;
    console.auto_hide = false;
    console.scroll_offset = 0;
    console.last_log_count = 0;
    console.last_toggle_time = get_tick_ms();
    console.input_buffer[0] = '\0';
    console.input_cursor = 0;

    command_count = 0;
}

void console_shutdown(void) {
#if CONFIG_IDF_TARGET_ESP32P4
    // Free PSRAM allocations
    if (log_buffer) {
        free(log_buffer);
        log_buffer = NULL;
    }
    if (commands) {
        free(commands);
        commands = NULL;
    }
#endif
    // Nothing else to cleanup for now
}

//=============================================================================
// Logging
//=============================================================================

void console_log(log_level_t level, const char* tag, const char* format, ...) {
    log_entry_t* entry = &log_buffer[log_write_index];

    // Store timestamp
    entry->timestamp = get_tick_ms();
    entry->level = level;

    // Store tag
    if (tag) {
        strncpy(entry->tag, tag, sizeof(entry->tag) - 1);
        entry->tag[sizeof(entry->tag) - 1] = '\0';
    } else {
        entry->tag[0] = '\0';
    }

    // Format message
    va_list args;
    va_start(args, format);
    vsnprintf(entry->text, sizeof(entry->text), format, args);
    va_end(args);

    // Advance write index
    log_write_index = (log_write_index + 1) % LOG_BUFFER_SIZE;
    if (log_count < LOG_BUFFER_SIZE) {
        log_count++;
    }

    // Also log to platform-specific output
    #if __DESKTOP_BUILD__
        // Print to stdout for desktop
        const char* level_str = "INFO";
        if (level == LOG_LEVEL_WARN) level_str = "WARN";
        else if (level == LOG_LEVEL_ERROR) level_str = "ERROR";
        else if (level == LOG_LEVEL_DEBUG) level_str = "DEBUG";
        printf("[%s] [%s] %s\n", level_str, tag ? tag : "???", entry->text);
    #else
        // Also log to ESP logger
        esp_log_level_t esp_level = ESP_LOG_INFO;
        if (level == LOG_LEVEL_WARN) esp_level = ESP_LOG_WARN;
        else if (level == LOG_LEVEL_ERROR) esp_level = ESP_LOG_ERROR;
        else if (level == LOG_LEVEL_DEBUG) esp_level = ESP_LOG_DEBUG;
        ESP_LOG_LEVEL(esp_level, tag, "%s", entry->text);
    #endif
}

void console_clear(void) {
#if CONFIG_IDF_TARGET_ESP32P4
    if (log_buffer) {
        memset(log_buffer, 0, LOG_BUFFER_SIZE * sizeof(log_entry_t));
    }
#else
    memset(log_buffer, 0, sizeof(log_buffer));
#endif
    log_write_index = 0;
    log_count = 0;
}

//=============================================================================
// Visibility Control
//=============================================================================

void console_toggle(void) {
    console.visible = !console.visible;
    console.last_toggle_time = get_tick_ms();

#if !__DESKTOP_BUILD__
    ESP_LOGI("Console", "Console toggle: %s", console.visible ? "SHOWING" : "HIDING");
#endif
}

void console_show(void) {
    if (!console.visible) {
        console.visible = true;
        console.last_toggle_time = get_tick_ms();
    }
}

void console_hide(void) {
    if (console.visible) {
        console.visible = false;
        console.last_toggle_time = get_tick_ms();
    }
}

bool console_is_visible(void) {
    return console.visible || console.slide_progress > 0.01f;
}

void console_set_auto_hide(bool auto_hide) {
    console.auto_hide = auto_hide;
}

//=============================================================================
// Animation
//=============================================================================

void console_update(void) {
    uint32_t now = get_tick_ms();
    uint32_t elapsed = now - console.last_toggle_time;
    float duration = 150.0f;  // 150ms slide animation

    if (console.visible && console.slide_progress < 1.0f) {
        // Sliding down (easing out)
        float t = elapsed / duration;
        if (t > 1.0f) t = 1.0f;
        // Cubic ease-out: 1 - (1-t)^3
        console.slide_progress = 1.0f - powf(1.0f - t, 3.0f);
    }
    else if (!console.visible && console.slide_progress > 0.0f) {
        // Sliding up (easing in)
        float t = elapsed / duration;
        if (t > 1.0f) t = 1.0f;
        // Cubic ease-in: (1-t)^3
        console.slide_progress = powf(1.0f - t, 3.0f);
    }
}

//=============================================================================
// Rendering
//=============================================================================

static Color get_color_for_level(log_level_t level) {
    #if __DESKTOP_BUILD__
        switch (level) {
            case LOG_LEVEL_INFO:  return WHITE;
            case LOG_LEVEL_WARN:  return YELLOW;
            case LOG_LEVEL_ERROR: return RED;
            case LOG_LEVEL_DEBUG: return GRAY;
            default:              return WHITE;
        }
    #else
        // ESP32-P4 color mapping (RGB565)
        switch (level) {
            case LOG_LEVEL_INFO:  return (Color){0xFF, 0xFF, 0xFF, 0xFF};  // White
            case LOG_LEVEL_WARN:  return (Color){0xFF, 0xFF, 0x00, 0xFF};  // Yellow
            case LOG_LEVEL_ERROR: return (Color){0xFF, 0x00, 0x00, 0xFF};  // Red
            case LOG_LEVEL_DEBUG: return (Color){0x80, 0x80, 0x80, 0xFF};  // Gray
            default:              return (Color){0xFF, 0xFF, 0xFF, 0xFF};
        }
    #endif
}

void console_render(void* framebuffer, int width, int height) {
    if (console.slide_progress <= 0.01f) {
        return;  // Hidden
    }

#if !__DESKTOP_BUILD__
    static int render_count = 0;
    if (render_count++ < 5) {  // Log first 5 renders only
        ESP_LOGI("Console", "Rendering console (progress=%.2f, visible=%d, fb=%p)",
                  console.slide_progress, console.visible, framebuffer);
    }
#endif

    #if __DESKTOP_BUILD__
        int screen_w = GetScreenWidth();
        int screen_h = GetScreenHeight();
        uint16_t* fb = NULL;  // Not used on desktop
    #else
        int screen_w = width;
        int screen_h = height;
        uint16_t* fb = (uint16_t*)framebuffer;
    #endif

    int console_h = (int)(screen_h * 0.6f * console.slide_progress);

    // Draw semi-transparent background
    #if __DESKTOP_BUILD__
        // Raylib: DrawRectangle with alpha works fine
        DrawRectangle(0, 0, screen_w, console_h, (Color){0, 0, 0, 200});  // 78% opacity black
    #else
        // ESP32-P4: Draw semi-transparent background directly to framebuffer
        // Use dark gray with transparency effect (blend with existing pixels)
        if (fb) {
            for (int y = 0; y < console_h && y < screen_h; y++) {
                for (int x = 0; x < screen_w; x++) {
                    int idx = y * screen_w + x;
                    uint16_t pixel = fb[idx];

                    // Extract RGB565 components
                    int r = ((pixel >> 11) & 0x1F);
                    int g = ((pixel >> 5) & 0x3F);
                    int b = (pixel & 0x1F);

                    // Darken by 75% (multiply by 0.25)
                    r = (r * 5) / 20;  // ~25% brightness
                    g = (g * 5) / 20;
                    b = (b * 5) / 20;

                    fb[idx] = (r << 11) | (g << 5) | b;
                }
            }
        }
    #endif

    // Draw log entries (bottom-up, show last VISIBLE_LINES)
    int y = console_h - 30;  // Leave space for input line
    int start_idx = (log_write_index - VISIBLE_LINES + LOG_BUFFER_SIZE) % LOG_BUFFER_SIZE;

    for (int i = 0; i < VISIBLE_LINES; i++) {
        int idx = (start_idx + i) % LOG_BUFFER_SIZE;
        log_entry_t* entry = &log_buffer[idx];

        if (entry->text[0] == '\0') continue;

        Color color = get_color_for_level(entry->level);

        // Format: [TAG] message
        char display_text[LOG_LINE_LENGTH + 64];  // Extra space for "[TAG] " prefix
        if (entry->tag[0] != '\0') {
            snprintf(display_text, sizeof(display_text), "[%s] %s", entry->tag, entry->text);
        } else {
            snprintf(display_text, sizeof(display_text), "%s", entry->text);
        }

        render_text(display_text, 10, y, color, fb, screen_w, screen_h);
        y -= 14;
    }

    // Draw input line
    #if __DESKTOP_BUILD__
        DrawRectangle(0, console_h - 25, screen_w, 25, (Color){30, 30, 30, 255});
        DrawText(">", 10, console_h - 20, 12, GREEN);
        DrawText(console.input_buffer, 25, console_h - 20, 12, WHITE);
    #else
        // ESP32-P4: Draw input line background and text
        if (fb && console_h >= 25) {
            int input_y = console_h - 25;

            // Draw input background (black)
            for (int y = input_y; y < console_h && y < screen_h; y++) {
                for (int x = 0; x < screen_w; x++) {
                    int idx = y * screen_w + x;
                    fb[idx] = 0x0000;  // Black
                }
            }

            // Draw prompt ">" using render_text
            render_text(">", 10, input_y + 5, (Color){0x00, 0xFF, 0x00, 0xFF}, fb, screen_w, screen_h);

            // Draw input buffer text
            render_text(console.input_buffer, 25, input_y + 5, (Color){0xFF, 0xFF, 0xFF, 0xFF}, fb, screen_w, screen_h);
        }
    #endif
}

static void render_text(const char* text, int x, int y, Color color,
                       uint16_t* fb, int fb_w, int fb_h) {
    #if __DESKTOP_BUILD__
        DrawText(text, x, y, 10, color);
    #else
        // ESP32-P4: Draw text directly to framebuffer using simple bitmap font
        if (!fb || !text) return;

        uint16_t color565 = ((color.r >> 3) << 11) | ((color.g >> 2) << 5) | (color.b >> 3);
        int cursor_x = x;

        for (const char* c = text; *c != '\0'; c++) {
            char ch = *c;

            // Convert to uppercase for simplicity
            if (ch >= 'a' && ch <= 'z') {
                ch = ch - 'a' + 'A';
            }

            // Get glyph for character
            const char* glyph = NULL;

            if (ch >= '0' && ch <= '9') {
                glyph = digit_font[ch - '0'];
            } else if (ch >= 'A' && ch <= 'Z') {
                glyph = letter_font[ch - 'A'];
            } else if (ch == '[') {
                // Draw opening bracket
                static const char bracket_glyph[35] = {
                    1,0,0,0,0,
                    1,0,0,0,0,
                    1,0,0,0,0,
                    1,0,0,0,0,
                    1,0,0,0,0,
                    1,0,0,0,0,
                    1,0,0,0,0
                };
                glyph = bracket_glyph;
            } else if (ch == ']') {
                // Draw closing bracket
                static const char bracket_glyph[35] = {
                    0,0,0,0,1,
                    0,0,0,0,1,
                    0,0,0,0,1,
                    0,0,0,0,1,
                    0,0,0,0,1,
                    0,0,0,0,1,
                    0,0,0,0,1
                };
                glyph = bracket_glyph;
            } else if (ch == ' ') {
                cursor_x += 3;
                continue;
            } else if (ch == ':') {
                // Draw colon
                static const char colon_glyph[35] = {
                    0,0,0,0,0,
                    0,0,1,0,0,
                    0,0,1,0,0,
                    0,0,0,0,0,
                    0,0,1,0,0,
                    0,0,1,0,0,
                    0,0,0,0,0
                };
                glyph = colon_glyph;
            } else if (ch == '.') {
                // Draw period
                static const char period_glyph[35] = {
                    0,0,0,0,0,
                    0,0,0,0,0,
                    0,0,0,0,0,
                    0,0,0,0,0,
                    0,0,0,0,0,
                    0,0,1,0,0,
                    0,0,0,0,0
                };
                glyph = period_glyph;
            } else if (ch == '>') {
                // Draw greater-than sign (for prompt)
                static const char gt_glyph[35] = {
                    0,0,0,0,0,
                    0,0,0,0,0,
                    0,0,0,0,1,
                    0,0,0,1,0,
                    0,0,1,0,0,
                    0,1,0,0,0,
                    0,0,0,0,0
                };
                glyph = gt_glyph;
            } else {
                // Unknown character - draw simple block
                static const char unknown_glyph[35] = {
                    0,0,0,0,0,
                    0,0,0,0,0,
                    0,0,1,0,0,
                    0,0,0,0,0,
                    0,0,0,0,0,
                    0,0,0,0,0,
                    0,0,0,0,0
                };
                glyph = unknown_glyph;
            }

            // Draw the glyph
            if (glyph) {
                for (int py = 0; py < 7; py++) {
                    for (int px = 0; px < 5; px++) {
                        if (glyph[py * 5 + px]) {
                            int draw_x = cursor_x + px;
                            int draw_y = y + py;
                            if (draw_x < fb_w && draw_y < fb_h) {
                                fb[draw_y * fb_w + draw_x] = color565;
                            }
                        }
                    }
                }
            }

            cursor_x += 6;  // 5 pixels width + 1 pixel spacing
        }
    #endif
}

//=============================================================================
// Public Text Drawing API
//=============================================================================

void console_draw_text(const char* text, int x, int y, unsigned char r, unsigned char g, unsigned char b,
                       void* framebuffer, int fb_width, int fb_height) {
    Color color = {r, g, b, 255};
    render_text(text, x, y, color, (uint16_t*)framebuffer, fb_width, fb_height);
}

//=============================================================================
// Command System
//=============================================================================

void console_register_command(const char* name, console_command_func_t func, const char* help) {
    if (command_count < MAX_COMMANDS) {
        strncpy(commands[command_count].name, name, 31);
        commands[command_count].name[31] = '\0';
        commands[command_count].func = func;
        strncpy(commands[command_count].help, help, 127);
        commands[command_count].help[127] = '\0';
        command_count++;
    }
}

void console_execute(const char* command_line) {
    char cmd[64];
    char args[192];

    // Parse command and arguments
    int parsed = sscanf(command_line, "%63s %191[^\n]", cmd, args);

    // Find and execute command
    for (int i = 0; i < command_count; i++) {
        if (strcmp(commands[i].name, cmd) == 0) {
            commands[i].func(parsed > 1 ? args : "");
            console_log(LOG_LEVEL_INFO, "CONSOLE", "> %s", command_line);
            return;
        }
    }

    console_log(LOG_LEVEL_ERROR, "CONSOLE", "Unknown command: %s", cmd);
}
