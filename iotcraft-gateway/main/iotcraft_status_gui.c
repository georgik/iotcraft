#include "iotcraft_gateway.h"
#include "esp_log.h"
#include "esp_system.h"
#include "esp_heap_caps.h"
#include "esp_timer.h"
#include "esp_task_wdt.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_freertos_hooks.h"
#include "esp_netif.h"
#include "esp_wifi.h"
#include <pthread.h>

// SDL3 includes
#include "SDL3/SDL.h"
#include "SDL3_ttf/SDL_ttf.h"
#include "bsp/esp-bsp.h"

static const char *TAG = "IOTCRAFT_GUI";
static bool gui_running = false;
static pthread_t gui_pthread;

// System monitoring globals
static float cpu_usage[2] = {0.0f, 0.0f}; // CPU0, CPU1
static size_t free_heap = 0;
static size_t total_heap = 0;
static uint32_t uptime_seconds = 0;

// Service status globals (updated from other modules)
typedef struct {
    bool dhcp_active;
    bool mqtt_active; 
    bool mdns_active;
    bool http_active;
    int connected_clients;
    int mqtt_connections;
    char wifi_ssid[32];
    char wifi_password[64];
    char gateway_ip[16];
    char sta_ip[16];
    bool sta_connected;
} gateway_status_t;

static gateway_status_t current_status = {
    .dhcp_active = true,
    .mqtt_active = false,
    .mdns_active = false, 
    .http_active = false,
    .connected_clients = 0,
    .mqtt_connections = 0,
    .wifi_ssid = "iotcraft",
    .wifi_password = "iotcraft123",
    .gateway_ip = "192.168.4.1",
    .sta_ip = "N/A",
    .sta_connected = false
};

// CPU monitoring task
static void cpu_monitor_task(void *param)
{
    uint32_t idle_count[2];
    uint32_t total_count[2];
    uint32_t last_idle[2] = {0, 0};
    uint32_t last_total[2] = {0, 0};
    
    while (gui_running) {
        // Get task run time stats for CPU usage calculation
        TaskStatus_t *task_array;
        UBaseType_t task_count;
        uint32_t total_run_time;
        
        task_count = uxTaskGetNumberOfTasks();
        task_array = pvPortMalloc(task_count * sizeof(TaskStatus_t));
        
        if (task_array != NULL) {
            task_count = uxTaskGetSystemState(task_array, task_count, &total_run_time);
            
            idle_count[0] = idle_count[1] = 0;
            total_count[0] = total_count[1] = 0;
            
            for (UBaseType_t i = 0; i < task_count; i++) {
                int core = 0; // Core ID not available in current API
                if (core >= 0 && core < 2) {
                    total_count[core] += task_array[i].ulRunTimeCounter;
                    if (strstr(task_array[i].pcTaskName, "IDLE") != NULL) {
                        idle_count[core] += task_array[i].ulRunTimeCounter;
                    }
                }
            }
            
            // Calculate CPU usage percentage
            for (int core = 0; core < 2; core++) {
                uint32_t idle_diff = idle_count[core] - last_idle[core];
                uint32_t total_diff = total_count[core] - last_total[core];
                
                if (total_diff > 0) {
                    cpu_usage[core] = 100.0f - ((float)idle_diff / total_diff * 100.0f);
                } else {
                    cpu_usage[core] = 0.0f;
                }
                
                last_idle[core] = idle_count[core];
                last_total[core] = total_count[core];
            }
            
            vPortFree(task_array);
        }
        
        // Update memory stats
        free_heap = esp_get_free_heap_size();
        total_heap = heap_caps_get_total_size(MALLOC_CAP_DEFAULT);
        
        // Update uptime
        uptime_seconds = esp_timer_get_time() / 1000000;
        
        vTaskDelay(pdMS_TO_TICKS(1000)); // Update every second
    }
    
    vTaskDelete(NULL);
}

// Format uptime as HH:MM:SS
static void format_uptime(uint32_t seconds, char *buffer, size_t size)
{
    uint32_t hours = seconds / 3600;
    uint32_t minutes = (seconds % 3600) / 60;
    uint32_t secs = seconds % 60;
    snprintf(buffer, size, "%02lu:%02lu:%02lu", hours, minutes, secs);
}

// Get STA IP address if connected
static void update_sta_ip_status(void)
{
    esp_netif_t *sta_netif = esp_netif_get_handle_from_ifkey("WIFI_STA_DEF");
    if (sta_netif) {
        esp_netif_ip_info_t ip_info;
        if (esp_netif_get_ip_info(sta_netif, &ip_info) == ESP_OK) {
            if (ip_info.ip.addr != 0) {
                // STA is connected and has an IP
                snprintf(current_status.sta_ip, sizeof(current_status.sta_ip), 
                        IPSTR, IP2STR(&ip_info.ip));
                current_status.sta_connected = true;
            } else {
                // STA interface exists but no IP assigned
                strncpy(current_status.sta_ip, "Connecting...", sizeof(current_status.sta_ip) - 1);
                current_status.sta_connected = false;
            }
        } else {
            // Failed to get IP info
            strncpy(current_status.sta_ip, "Error", sizeof(current_status.sta_ip) - 1);
            current_status.sta_connected = false;
        }
    } else {
        // STA interface not found
        strncpy(current_status.sta_ip, "N/A", sizeof(current_status.sta_ip) - 1);
        current_status.sta_connected = false;
    }
}

// Draw status indicator circle
static void draw_status_circle(SDL_Renderer *renderer, float x, float y, float radius, bool active)
{
    SDL_Color color = active ? (SDL_Color){0, 255, 0, 255} : (SDL_Color){255, 0, 0, 255};
    SDL_SetRenderDrawColor(renderer, color.r, color.g, color.b, color.a);
    
    // Draw filled circle (simplified)
    for (int dy = -radius; dy <= radius; dy++) {
        for (int dx = -radius; dx <= radius; dx++) {
            if (dx*dx + dy*dy <= radius*radius) {
                SDL_RenderPoint(renderer, x + dx, y + dy);
            }
        }
    }
}

// Draw progress bar
static void draw_progress_bar(SDL_Renderer *renderer, float x, float y, float width, float height, float percentage)
{
    // Background
    SDL_SetRenderDrawColor(renderer, 64, 64, 64, 255);
    SDL_FRect bg_rect = {x, y, width, height};
    SDL_RenderFillRect(renderer, &bg_rect);
    
    // Progress fill
    SDL_Color fill_color = {0, 255, 0, 255};
    if (percentage > 80.0f) fill_color = (SDL_Color){255, 165, 0, 255}; // Orange
    if (percentage > 95.0f) fill_color = (SDL_Color){255, 0, 0, 255};   // Red
    
    SDL_SetRenderDrawColor(renderer, fill_color.r, fill_color.g, fill_color.b, fill_color.a);
    SDL_FRect fill_rect = {x, y, width * (percentage / 100.0f), height};
    SDL_RenderFillRect(renderer, &fill_rect);
    
    // Border
    SDL_SetRenderDrawColor(renderer, 128, 128, 128, 255);
    SDL_RenderRect(renderer, &bg_rect);
}

// Draw text helper
static void draw_text_at(SDL_Renderer *renderer, TTF_Font *font, const char *text, float x, float y, SDL_Color color)
{
    SDL_Surface *surface = TTF_RenderText_Blended(font, text, 0, color);
    if (!surface) return;
    
    SDL_Texture *texture = SDL_CreateTextureFromSurface(renderer, surface);
    if (!texture) {
        SDL_DestroySurface(surface);
        return;
    }
    
    float text_width, text_height;
    SDL_GetTextureSize(texture, &text_width, &text_height);
    SDL_FRect dest_rect = {x, y, (float)text_width, (float)text_height};
    SDL_RenderTexture(renderer, texture, NULL, &dest_rect);
    
    SDL_DestroyTexture(texture);
    SDL_DestroySurface(surface);
}

// Main GUI thread
static void* gui_thread(void* args)
{
    ESP_LOGI(TAG, "Starting IoTCraft Gateway Status GUI");
    
    if (SDL_Init(SDL_INIT_VIDEO | SDL_INIT_EVENTS) == false) {
        ESP_LOGE(TAG, "Unable to initialize SDL: %s", SDL_GetError());
        return NULL;
    }
    
    // Initialize TTF
    if (TTF_Init() == false) {
        ESP_LOGE(TAG, "Unable to initialize SDL_ttf: %s", SDL_GetError());
        SDL_Quit();
        return NULL;
    }
    
    SDL_Window *window = SDL_CreateWindow("IoTCraft Gateway", BSP_LCD_H_RES, BSP_LCD_V_RES, 0);
    if (!window) {
        ESP_LOGE(TAG, "Failed to create window: %s", SDL_GetError());
        TTF_Quit();
        SDL_Quit();
        return NULL;
    }
    
    SDL_Renderer *renderer = SDL_CreateRenderer(window, NULL);
    if (!renderer) {
        ESP_LOGE(TAG, "Failed to create renderer: %s", SDL_GetError());
        SDL_DestroyWindow(window);
        TTF_Quit();
        SDL_Quit();
        return NULL;
    }
    
    // SDL_InitFS() is not needed in SDL3
    
    // Load fonts
    TTF_Font *title_font = TTF_OpenFont("/assets/FreeSans.ttf", 20);
    TTF_Font *text_font = TTF_OpenFont("/assets/FreeSans.ttf", 14);
    TTF_Font *small_font = TTF_OpenFont("/assets/FreeSans.ttf", 12);
    
    if (!title_font || !text_font || !small_font) {
        ESP_LOGE(TAG, "Failed to load fonts");
        // Continue without fonts for now
    }
    
    ESP_LOGI(TAG, "SDL GUI initialized successfully");
    gui_running = true;
    
    // Start CPU monitoring task
    xTaskCreate(cpu_monitor_task, "cpu_monitor", 4096, NULL, 5, NULL);
    
    SDL_Event event;
    uint32_t last_update = SDL_GetTicks();
    
    while (gui_running) {
        while (SDL_PollEvent(&event)) {
            if (event.type == SDL_EVENT_QUIT) {
                gui_running = false;
                break;
            }
        }
        
        uint32_t current_time = SDL_GetTicks();
        if (current_time - last_update < 500) { // Update at 2 FPS to save power
            vTaskDelay(pdMS_TO_TICKS(50));
            continue;
        }
        last_update = current_time;
        
        // Update service status from other modules
        current_status.mqtt_active = iotcraft_mqtt_is_running();
        current_status.mdns_active = true;  // TODO: get from iotcraft_mdns_is_running()
        current_status.http_active = true;  // TODO: get from iotcraft_http_is_running()
        
        // Update WiFi configuration
        iotcraft_wifi_config_t wifi_config;
        if (iotcraft_get_wifi_config(&wifi_config) == ESP_OK) {
            strncpy(current_status.wifi_ssid, wifi_config.ssid, sizeof(current_status.wifi_ssid) - 1);
            current_status.wifi_ssid[sizeof(current_status.wifi_ssid) - 1] = '\0';
            strncpy(current_status.wifi_password, wifi_config.password, sizeof(current_status.wifi_password) - 1);
            current_status.wifi_password[sizeof(current_status.wifi_password) - 1] = '\0';
        }
        
        // Update STA IP address status
        update_sta_ip_status();
        
        // Clear screen with dark background
        SDL_SetRenderDrawColor(renderer, 20, 20, 30, 255);
        SDL_RenderClear(renderer);
        
        float y_pos = 10.0f;
        SDL_Color white = {255, 255, 255, 255};
        SDL_Color green = {0, 255, 0, 255};
        
        // Title - removed emoji
        if (title_font) {
            draw_text_at(renderer, title_font, "IoTCraft Gateway", 10.0f, y_pos, white);
        }
        y_pos += 35.0f;
        
        // Two-column layout setup
        float left_col_x = 10.0f;
        float right_col_x = 170.0f; // Start right column at x=170
        float left_col_y = y_pos;
        float right_col_y = y_pos;
        
        // LEFT COLUMN - Services
        if (text_font) {
            draw_text_at(renderer, text_font, "Services:", left_col_x, left_col_y, white);
        }
        left_col_y += 20.0f; // Reduced spacing
        
        // Service indicators - more compact
        float service_x = left_col_x + 8.0f;
        draw_status_circle(renderer, service_x, left_col_y + 6, 5, current_status.dhcp_active); // Smaller circle
        if (small_font) {
            draw_text_at(renderer, small_font, "DHCP", service_x + 12, left_col_y, 
                        current_status.dhcp_active ? green : white);
        }
        left_col_y += 16.0f; // Reduced spacing
        
        draw_status_circle(renderer, service_x, left_col_y + 6, 5, current_status.mqtt_active);
        if (small_font) {
            draw_text_at(renderer, small_font, "MQTT", service_x + 12, left_col_y, 
                        current_status.mqtt_active ? green : white);
        }
        left_col_y += 16.0f;
        
        draw_status_circle(renderer, service_x, left_col_y + 6, 5, current_status.mdns_active);
        if (small_font) {
            draw_text_at(renderer, small_font, "mDNS", service_x + 12, left_col_y, 
                        current_status.mdns_active ? green : white);
        }
        left_col_y += 16.0f;
        
        draw_status_circle(renderer, service_x, left_col_y + 6, 5, current_status.http_active);
        if (small_font) {
            draw_text_at(renderer, small_font, "HTTP", service_x + 12, left_col_y, 
                        current_status.http_active ? green : white);
        }
        left_col_y += 20.0f; // Reduced spacing
        
        // RIGHT COLUMN - Network Info
        if (text_font) {
            draw_text_at(renderer, text_font, "Network:", right_col_x, right_col_y, white);
        }
        right_col_y += 20.0f; // Reduced spacing
        
        if (small_font) {
            char network_info[128];
            snprintf(network_info, sizeof(network_info), "SSID: %s", current_status.wifi_ssid);
            draw_text_at(renderer, small_font, network_info, right_col_x + 8.0f, right_col_y, white);
            right_col_y += 16.0f; // Reduced spacing
            
            // Show WiFi password
            snprintf(network_info, sizeof(network_info), "Pass: %s", current_status.wifi_password);
            draw_text_at(renderer, small_font, network_info, right_col_x + 8.0f, right_col_y, white);
            right_col_y += 16.0f;
            
            // Show AP IP (Gateway IP)
            snprintf(network_info, sizeof(network_info), "AP IP: %s", current_status.gateway_ip);
            draw_text_at(renderer, small_font, network_info, right_col_x + 8.0f, right_col_y, white);
            right_col_y += 16.0f;
            
            // Show STA IP with connection status indicator
            SDL_Color sta_color = current_status.sta_connected ? green : white;
            snprintf(network_info, sizeof(network_info), "STA IP: %s", current_status.sta_ip);
            draw_text_at(renderer, small_font, network_info, right_col_x + 8.0f, right_col_y, sta_color);
            right_col_y += 16.0f;
            
            snprintf(network_info, sizeof(network_info), "Clients: %d", current_status.connected_clients);
            draw_text_at(renderer, small_font, network_info, right_col_x + 8.0f, right_col_y, white);
            right_col_y += 16.0f;
        }
        
        // Continue below both columns, use the higher y position
        y_pos = (left_col_y > right_col_y) ? left_col_y : right_col_y;
        y_pos += 10.0f; // Add some spacing
        
        // System Health - Two-column compact layout
        if (text_font) {
            draw_text_at(renderer, text_font, "System Health:", 10.0f, y_pos, white);
        }
        y_pos += 20.0f; // Reduced spacing
        
        // Reset column positions for System Health
        float sys_left_x = left_col_x + 10.0f;
        float sys_right_x = right_col_x + 10.0f;
        float sys_left_y = y_pos;
        float sys_right_y = y_pos;
        
        if (small_font) {
            char sys_text[64];
            
            // LEFT COLUMN - CPU Usage (compact)
            snprintf(sys_text, sizeof(sys_text), "CPU0: %.1f%%", cpu_usage[0]);
            draw_text_at(renderer, small_font, sys_text, sys_left_x, sys_left_y, white);
            // Compact progress bar for CPU0
            draw_progress_bar(renderer, sys_left_x + 70.0f, sys_left_y + 2, 70.0f, 10.0f, cpu_usage[0]);
            sys_left_y += 16.0f;
            
            snprintf(sys_text, sizeof(sys_text), "CPU1: %.1f%%", cpu_usage[1]);
            draw_text_at(renderer, small_font, sys_text, sys_left_x, sys_left_y, white);
            // Compact progress bar for CPU1
            draw_progress_bar(renderer, sys_left_x + 70.0f, sys_left_y + 2, 70.0f, 10.0f, cpu_usage[1]);
            sys_left_y += 20.0f;
            
            // RIGHT COLUMN - Memory and Uptime (numbers only)
            snprintf(sys_text, sizeof(sys_text), "Mem: %.0f/%.0fKB", 
                    (total_heap - free_heap) / 1024.0f, total_heap / 1024.0f);
            draw_text_at(renderer, small_font, sys_text, sys_right_x, sys_right_y, white);
            sys_right_y += 16.0f;
            
            char uptime_str[32];
            format_uptime(uptime_seconds, uptime_str, sizeof(uptime_str));
            snprintf(sys_text, sizeof(sys_text), "Up: %s", uptime_str);
            draw_text_at(renderer, small_font, sys_text, sys_right_x, sys_right_y, white);
            sys_right_y += 16.0f;
        }
        
        SDL_RenderPresent(renderer);
        vTaskDelay(pdMS_TO_TICKS(50));
    }
    
    // Cleanup
    if (title_font) TTF_CloseFont(title_font);
    if (text_font) TTF_CloseFont(text_font);
    if (small_font) TTF_CloseFont(small_font);
    
    TTF_Quit();
    SDL_DestroyRenderer(renderer);
    SDL_DestroyWindow(window);
    SDL_Quit();
    
    ESP_LOGI(TAG, "SDL GUI stopped");
    return NULL;
}

esp_err_t iotcraft_status_gui_init(void)
{
    if (gui_running) {
        ESP_LOGW(TAG, "Status GUI already running");
        return ESP_OK;
    }
    
    pthread_attr_t attr;
    pthread_attr_init(&attr);
    pthread_attr_setstacksize(&attr, 32768); // 32KB stack for SDL
    
    int ret = pthread_create(&gui_pthread, &attr, gui_thread, NULL);
    if (ret != 0) {
        ESP_LOGE(TAG, "Failed to create GUI thread: %d", ret);
        return ESP_FAIL;
    }
    
    pthread_detach(gui_pthread);
    ESP_LOGI(TAG, "Status GUI thread created");
    
    return ESP_OK;
}

esp_err_t iotcraft_status_gui_stop(void)
{
    if (!gui_running) {
        return ESP_OK;
    }
    
    gui_running = false;
    ESP_LOGI(TAG, "Status GUI stopping...");
    
    return ESP_OK;
}

bool iotcraft_status_gui_is_running(void)
{
    return gui_running;
}

// Function to update service status from other modules
esp_err_t iotcraft_status_gui_update_status(const iotcraft_status_t *status)
{
    if (status) {
        current_status.dhcp_active = status->dhcp_running;
        current_status.mqtt_active = status->mqtt_running;
        current_status.mdns_active = status->mdns_running;
        current_status.http_active = status->http_running;
        current_status.connected_clients = status->connected_clients;
        current_status.mqtt_connections = status->mqtt_connections;
    }
    return ESP_OK;
}
