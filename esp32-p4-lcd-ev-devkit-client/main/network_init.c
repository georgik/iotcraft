/**
 * @file network_init.c
 * @brief Network connectivity implementation - Ethernet for ESP32-P4
 */

#include "network_init.h"
#include "esp_eth.h"
#include "esp_event.h"
#include "esp_netif.h"
#include "esp_log.h"
#include "driver/gpio.h"
#include <string.h>

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
static bool g_connected = false;
static bool g_got_ip = false;

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
}

esp_err_t network_init(void) {
    ESP_LOGI(TAG, "Initializing Ethernet...");

    // Create default event loop
    ESP_ERROR_CHECK(esp_event_loop_create_default());

    // Create netif for Ethernet
    esp_netif_config_t cfg = ESP_NETIF_DEFAULT_ETH();
    g_eth_netif = esp_netif_new(&cfg);

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

    // Start Ethernet driver
    ESP_ERROR_CHECK(esp_eth_start(eth_handle));

    ESP_LOGI(TAG, "Ethernet initialization complete, waiting for connection...");

    // Wait for IP connection (with timeout)
    int retries = 20;  // 20 * 500ms = 10 seconds
    while (!g_got_ip && retries > 0) {
        vTaskDelay(pdMS_TO_TICKS(500));
        retries--;
    }

    if (!g_got_ip) {
        ESP_LOGW(TAG, "Timeout waiting for Ethernet IP");
        return ESP_ERR_TIMEOUT;
    }

    ESP_LOGI(TAG, "Network ready!");
    return ESP_OK;
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
