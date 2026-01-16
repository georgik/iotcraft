/**
 * @file network_init.c
 * @brief Network connectivity implementation - Ethernet and WiFi (ESP-Hosted) for ESP32-P4
 */

#include "network_init.h"
#include "esp_eth.h"
#include "esp_event.h"
#include "esp_netif.h"
#include "esp_log.h"
#include "driver/gpio.h"
#include <string.h>

// ESP-Hosted and WiFi support (can be disabled via idf_component.yml)
#ifdef CONFIG_ESP_HOSTED_ENABLED
    #include "esp_hosted.h"
    #include "esp_wifi.h"
    #define HAS_WIFI_SUPPORT 1
#else
    #define HAS_WIFI_SUPPORT 0
#endif

#if CONFIG_BOARD_ESP32_P4_FUNCTION_EV
#include "esp32p4/rom/ets_sys.h"
#include "soc/soc_caps.h"
#endif

static const char* TAG = "Network";

#define ETH_PHY_ADDR        1
#define ETH_PHY_RST_GPIO    -1  // No reset GPIO
#define ETH_MDC_GPIO        13
#define ETH_MDIO_GPIO       12
#define MAX_IP_LEN          16

static esp_netif_t* g_eth_netif = NULL;
#if HAS_WIFI_SUPPORT
static esp_netif_t* g_wifi_netif = NULL;
#endif
static bool g_connected = false;
static bool g_got_ip = false;
static network_got_ip_callback_t g_got_ip_callback = NULL;

#if HAS_WIFI_SUPPORT
// WiFi credentials
#define DEFAULT_WIFI_SSID     "iotcraft"
#define DEFAULT_WIFI_PASSWORD "iotcraft123"

// WiFi init task synchronization
static TaskHandle_t g_wifi_init_task_handle = NULL;
static volatile bool g_wifi_init_done = false;
static volatile bool g_wifi_init_success = false;

// Forward declarations
static void wifi_event_handler(void* arg, esp_event_base_t event_base,
                                int32_t event_id, void* event_data);
static void got_ip_event_handler(void* arg, esp_event_base_t event_base,
                                  int32_t event_id, void* event_data);

/**
 * @brief WiFi initialization task with timeout protection
 *
 * This task runs esp_wifi_init() which can block indefinitely if ESP32-C6
 * co-processor is not responding. We run it in a separate task so we can
 * detect if it's taking too long and continue without WiFi.
 */
static void wifi_init_task(void* arg) {
    ESP_LOGI(TAG, "[WiFi Task] Starting WiFi initialization...");

    // IMPORTANT: Initialize ESP-Hosted FIRST before WiFi
    ESP_LOGI(TAG, "[WiFi Task] Initializing ESP-Hosted (ESP32-C6 SDIO transport)...");
    esp_err_t hosted_ret = esp_hosted_init();

    if (hosted_ret != ESP_OK) {
        ESP_LOGW(TAG, "[WiFi Task] ⚠ ESP-Hosted initialization failed: %s", esp_err_to_name(hosted_ret));
        ESP_LOGW(TAG, "[WiFi Task] ⚠ Check ESP32-C6 firmware and SDIO connection");
        g_wifi_init_done = true;
        g_wifi_init_success = false;
        vTaskDelete(NULL);
        return;
    }

    ESP_LOGI(TAG, "[WiFi Task] ✓ ESP-Hosted initialized successfully");

    // Create WiFi station interface
    g_wifi_netif = esp_netif_create_default_wifi_sta();

    // Initialize WiFi with default configuration
    wifi_init_config_t wifi_cfg = WIFI_INIT_CONFIG_DEFAULT();
    esp_err_t ret = esp_wifi_init(&wifi_cfg);

    if (ret == ESP_OK) {
        ESP_LOGI(TAG, "[WiFi Task] ✓ WiFi initialized successfully");

        // Register WiFi event handlers
        ESP_ERROR_CHECK(esp_event_handler_instance_register(WIFI_EVENT,
                                                            ESP_EVENT_ANY_ID,
                                                            &wifi_event_handler,
                                                            NULL,
                                                            NULL));
        ESP_ERROR_CHECK(esp_event_handler_instance_register(IP_EVENT,
                                                            IP_EVENT_STA_GOT_IP,
                                                            &got_ip_event_handler,
                                                            NULL,
                                                            NULL));

        // Configure WiFi
        wifi_config_t wifi_config = {
            .sta = {
                .ssid = DEFAULT_WIFI_SSID,
                .password = DEFAULT_WIFI_PASSWORD,
                .threshold.authmode = WIFI_AUTH_WPA2_PSK,
            },
        };

        ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_STA));
        ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_STA, &wifi_config));
        ESP_ERROR_CHECK(esp_wifi_start());

        ESP_LOGI(TAG, "[WiFi Task] ✓ WiFi started, connecting to '%s'...", DEFAULT_WIFI_SSID);
        g_wifi_init_success = true;
    } else {
        ESP_LOGW(TAG, "[WiFi Task] ⚠ WiFi initialization failed: %s", esp_err_to_name(ret));
        ESP_LOGW(TAG, "[WiFi Task] ⚠ WiFi co-processor (ESP32-C6) may not be present or not have firmware");
        ESP_LOGW(TAG, "[WiFi Task] ⚠ Continuing with Ethernet only");
        g_wifi_init_success = false;
    }

    g_wifi_init_done = true;
    g_wifi_init_task_handle = NULL;
    vTaskDelete(NULL);
}
#endif // HAS_WIFI_SUPPORT

/**
 * @brief WiFi event handler
 */
#if HAS_WIFI_SUPPORT
static void wifi_event_handler(void* arg, esp_event_base_t event_base,
                                int32_t event_id, void* event_data) {
    if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_START) {
        ESP_LOGI(TAG, "WiFi started, connecting to AP...");
        esp_wifi_connect();
    } else if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_CONNECTED) {
        ESP_LOGI(TAG, "WiFi connected to AP");
    } else if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_DISCONNECTED) {
        ESP_LOGW(TAG, "WiFi disconnected, retrying...");
        esp_wifi_connect();
    }
}
#endif

/**
 * @brief Ethernet event handler
 */
static void eth_event_handler(void* arg, esp_event_base_t event_base,
                               int32_t event_id, void* event_data) {
    uint8_t mac_addr[6] = {0};
    esp_eth_handle_t eth_handle = *(esp_eth_handle_t*)arg;

    switch (event_id) {
        case ETHERNET_EVENT_CONNECTED:
            esp_eth_ioctl(eth_handle, ETH_CMD_G_MAC_ADDR, mac_addr);
            ESP_LOGI(TAG, "Ethernet Link Up");
            ESP_LOGI(TAG, "Ethernet HW Addr %02x:%02x:%02x:%02x:%02x:%02x",
                     mac_addr[0], mac_addr[1], mac_addr[2], mac_addr[3], mac_addr[4], mac_addr[5]);
            g_connected = true;
            break;

        case ETHERNET_EVENT_DISCONNECTED:
            ESP_LOGW(TAG, "Ethernet Link Down");
            g_connected = false;
            g_got_ip = false;
            break;

        case ETHERNET_EVENT_START:
            ESP_LOGI(TAG, "Ethernet Started");
            break;

        case ETHERNET_EVENT_STOP:
            ESP_LOGI(TAG, "Ethernet Stopped");
            g_connected = false;
            g_got_ip = false;
            break;

        default:
            break;
    }
}

/**
 * @brief IP event handler
 */
static void got_ip_event_handler(void* arg, esp_event_base_t event_base,
                                  int32_t event_id, void* event_data) {
    ip_event_got_ip_t* event = (ip_event_got_ip_t*)event_data;
    const esp_netif_ip_info_t* ip_info = &event->ip_info;

    ESP_LOGI(TAG, "Ethernet Got IP Address");
    ESP_LOGI(TAG, "~~~~~~~~~~~");
    ESP_LOGI(TAG, "IP:" IPSTR, IP2STR(&ip_info->ip));
    ESP_LOGI(TAG, "MASK:" IPSTR, IP2STR(&ip_info->netmask));
    ESP_LOGI(TAG, "GW:" IPSTR, IP2STR(&ip_info->gw));
    ESP_LOGI(TAG, "~~~~~~~~~~~");

    g_got_ip = true;

    // Invoke user callback if registered
    if (g_got_ip_callback != NULL) {
        char ip_str[MAX_IP_LEN];
        snprintf(ip_str, sizeof(ip_str), IPSTR, IP2STR(&ip_info->ip));
        g_got_ip_callback(ip_str);
    }
}

esp_err_t network_init(void) {
    ESP_LOGI(TAG, "Initializing network connectivity...");

    // Initialize TCP/IP network interface (must be called first)
    ESP_ERROR_CHECK(esp_netif_init());

    // Create default event loop
    ESP_ERROR_CHECK(esp_event_loop_create_default());

#if HAS_WIFI_SUPPORT
    // ============================================================
    // WIFI INITIALIZATION: Non-blocking with timeout
    // ============================================================
    // IMPORTANT: ESP-Hosted SDIO driver can block indefinitely if C6 is not ready
    // Solution: Run WiFi init in separate task with watchdog timeout
    ESP_LOGI(TAG, "Initializing WiFi (ESP32-C6 co-processor via ESP-Hosted)...");
    ESP_LOGI(TAG, "  Starting WiFi init in background task (10 second timeout)...");

    g_wifi_init_done = false;
    g_wifi_init_success = false;

    // Create WiFi init task (high priority to ensure it runs quickly)
    BaseType_t ret = xTaskCreatePinnedToCore(
        wifi_init_task,
        "wifi_init",
        8192,  // 8KB stack
        NULL,
        5,     // Priority
        &g_wifi_init_task_handle,
        0      // Core 0
    );

    if (ret != pdPASS) {
        ESP_LOGW(TAG, "⚠ Failed to create WiFi init task, skipping WiFi");
        goto skip_wifi;
    }

    // Wait for WiFi init to complete with 10 second timeout
    int timeout_seconds = 10;
    for (int i = 0; i < timeout_seconds * 10; i++) {
        if (g_wifi_init_done) {
            ESP_LOGI(TAG, "  WiFi init completed in %d ms", i * 100);
            break;
        }
        vTaskDelay(pdMS_TO_TICKS(100));
    }

    if (!g_wifi_init_done) {
        ESP_LOGW(TAG, "⚠ WiFi init timeout after %d seconds", timeout_seconds);
        ESP_LOGW(TAG, "  ESP32-C6 co-processor may not be present or firmware not loaded");
        ESP_LOGW(TAG, "  Continuing without WiFi (Ethernet only)");
        // Note: WiFi init task is still running in background, but we're proceeding anyway
    } else if (!g_wifi_init_success) {
        ESP_LOGW(TAG, "⚠ WiFi init failed, continuing with Ethernet only");
    }

skip_wifi:
    // ============================================================
#else
    ESP_LOGI(TAG, "WiFi support disabled (ESP-Hosted not available)");
#endif // HAS_WIFI_SUPPORT

    // Create netif for Ethernet
    esp_netif_config_t eth_cfg = ESP_NETIF_DEFAULT_ETH();
    g_eth_netif = esp_netif_new(&eth_cfg);

    // Initialize Ethernet MAC
    eth_mac_config_t mac_config = ETH_MAC_DEFAULT_CONFIG();
    eth_esp32_emac_config_t esp32_emac_config = ETH_ESP32_EMAC_DEFAULT_CONFIG();

    // Use EMAC GPIO pins for P4 Function EV board
    #if CONFIG_BOARD_ESP32_P4_FUNCTION_EV
    // ESP32-P4 EMAC configuration
    esp32_emac_config.smi_gpio.mdc_num = ETH_MDC_GPIO;
    esp32_emac_config.smi_gpio.mdio_num = ETH_MDIO_GPIO;
    #endif

    esp_eth_mac_t* mac = esp_eth_mac_new_esp32(&esp32_emac_config, &mac_config);

    // Initialize Ethernet PHY (RTL8201 or generic)
    eth_phy_config_t phy_config = ETH_PHY_DEFAULT_CONFIG();
    phy_config.phy_addr = ETH_PHY_ADDR;
    phy_config.reset_gpio_num = ETH_PHY_RST_GPIO;
    esp_eth_phy_t* phy = esp_eth_phy_new_generic(&phy_config);

    // Initialize Ethernet driver
    esp_eth_config_t eth_config = ETH_DEFAULT_CONFIG(mac, phy);
    esp_eth_handle_t eth_handle = NULL;
    ESP_ERROR_CHECK(esp_eth_driver_install(&eth_config, &eth_handle));

    // Connect Ethernet MAC to netif
    ESP_ERROR_CHECK(esp_netif_attach(g_eth_netif, esp_eth_new_netif_glue(eth_handle)));

    // Register event handlers
    ESP_ERROR_CHECK(esp_event_handler_register(ETH_EVENT, ESP_EVENT_ANY_ID, &eth_event_handler, &eth_handle));
    ESP_ERROR_CHECK(esp_event_handler_register(IP_EVENT, IP_EVENT_ETH_GOT_IP, &got_ip_event_handler, NULL));

    // Start Ethernet driver (returns immediately, DHCP happens in background)
    ESP_ERROR_CHECK(esp_eth_start(eth_handle));

    ESP_LOGI(TAG, "Ethernet started, waiting for link and DHCP...");
    return ESP_OK;
}

void network_set_got_ip_callback(network_got_ip_callback_t callback) {
    g_got_ip_callback = callback;
}

bool network_is_connected(void) {
    return g_connected && g_got_ip;
}

esp_err_t network_get_ip(char* buf, size_t buf_size) {
    if (!g_eth_netif || !g_got_ip) {
        return ESP_ERR_INVALID_STATE;
    }

    esp_netif_ip_info_t ip_info;
    esp_err_t ret = esp_netif_get_ip_info(g_eth_netif, &ip_info);
    if (ret != ESP_OK) {
        return ret;
    }

    snprintf(buf, buf_size, IPSTR, IP2STR(&ip_info.ip));
    return ESP_OK;
}
