#include "iotcraft_gateway.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include "lwip/sockets.h"
#include "lwip/inet.h"
#include "lwip/dns.h"            // For DNS if needed
#include "esp_log.h"
#include "esp_wifi.h"
#include "esp_event.h"
#include "esp_system.h"
#include "esp_vfs.h"
#include "esp_littlefs.h"
#include "nvs_flash.h"
#include "esp_netif.h"
#include "cJSON.h"
#include "lwip/ip_addr.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "lwip/pbuf.h"
#include "lwip/netif.h"
#include "lwip/etharp.h"
#include "lwip/err.h"
#include "esp_netif_private.h"

// Use the esp_netif NAT API provided by ESP-IDF:
#include "lwip/lwip_napt.h"

extern struct netif *esp_netif_get_netif_impl(esp_netif_t *esp_netif);

static const char *TAG = "CUSTOM_DHCP_SERVER";

#define MAX_RESERVATIONS 10
#define MAX_DYNAMIC_LEASES 32

/* Reservation table entry: client MAC and reserved IP */
typedef struct {
    uint8_t mac[6];
    ip4_addr_t reserved_ip;
} dhcp_reservation_t;

static dhcp_reservation_t reservation_table[MAX_RESERVATIONS];
static int reservation_count = 0;

#define WIFI_CONFIG_FILE "/assets/wifi_config.json"
#define DHCP_RESERVATIONS_FILE "/assets/dhcp_reservations.json"

/* Define our own AP configuration type */
typedef struct {
    char ssid[32];
    char password[64];
} my_ap_config_t;

static my_ap_config_t wifi_ap_config = {
    .ssid = "iotcraft",
    .password = "iotcraft123"
};

/* STA (parent/upstream) credentials */
static char sta_ssid[32] = "Default_STA_SSID";
static char sta_password[64] = "Default_STA_Password";

/* Global pointers to AP and STA netifs */
static esp_netif_t *g_ap_netif = NULL;
static esp_netif_t *g_sta_netif = NULL;

/* Mount LittleFS */
static esp_err_t mount_littlefs(void)
{
    esp_vfs_littlefs_conf_t conf = {
        .base_path = "/assets",
        .partition_label = "assets",
        .format_if_mount_failed = false,
        .dont_mount = false,
    };

    esp_err_t ret = esp_vfs_littlefs_register(&conf);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to mount LittleFS: %s", esp_err_to_name(ret));
    } else {
        ESP_LOGI(TAG, "LittleFS mounted successfully");
    }
    return ret;
}

/* Load Wi‑Fi configuration from JSON file */
static esp_err_t load_wifi_config(void)
{
    FILE *f = fopen(WIFI_CONFIG_FILE, "r");
    if (f == NULL) {
        ESP_LOGE(TAG, "Failed to open %s, using default config", WIFI_CONFIG_FILE);
        return ESP_FAIL;
    }
    char buffer[512];
    size_t bytes_read = fread(buffer, 1, sizeof(buffer) - 1, f);
    buffer[bytes_read] = '\0';
    fclose(f);

    ESP_LOGI(TAG, "WiFi config file content: %s", buffer);
    cJSON *json = cJSON_Parse(buffer);
    if (json == NULL) {
        ESP_LOGE(TAG, "Error parsing WiFi config JSON");
        return ESP_FAIL;
    }

    // Parse AP configuration
    cJSON *ap = cJSON_GetObjectItemCaseSensitive(json, "ap");
    if (cJSON_IsObject(ap)) {
        cJSON *ap_ssid = cJSON_GetObjectItemCaseSensitive(ap, "ssid");
        cJSON *ap_password = cJSON_GetObjectItemCaseSensitive(ap, "password");
        if (cJSON_IsString(ap_ssid) && (ap_ssid->valuestring != NULL)) {
            strncpy(wifi_ap_config.ssid, ap_ssid->valuestring, sizeof(wifi_ap_config.ssid) - 1);
        }
        if (cJSON_IsString(ap_password) && (ap_password->valuestring != NULL)) {
            strncpy(wifi_ap_config.password, ap_password->valuestring, sizeof(wifi_ap_config.password) - 1);
        }
    }

    // Parse STA (parent) configuration
    cJSON *sta = cJSON_GetObjectItemCaseSensitive(json, "sta");
    if (cJSON_IsObject(sta)) {
        cJSON *s_ssid = cJSON_GetObjectItemCaseSensitive(sta, "ssid");
        cJSON *s_password = cJSON_GetObjectItemCaseSensitive(sta, "password");
        if (cJSON_IsString(s_ssid) && (s_ssid->valuestring != NULL)) {
            strncpy(sta_ssid, s_ssid->valuestring, sizeof(sta_ssid) - 1);
        }
        if (cJSON_IsString(s_password) && (s_password->valuestring != NULL)) {
            strncpy(sta_password, s_password->valuestring, sizeof(sta_password) - 1);
        }
    }

    cJSON_Delete(json);
    ESP_LOGI(TAG, "Loaded WiFi AP config: SSID=%s, Password=%s", wifi_ap_config.ssid, wifi_ap_config.password);
    ESP_LOGI(TAG, "Loaded WiFi STA config: SSID=%s, Password=%s", sta_ssid, sta_password);
    return ESP_OK;
}

/* Parse a MAC address string into a 6-byte array */
static esp_err_t parse_mac_string(const char *mac_str, uint8_t mac[6])
{
    if (sscanf(mac_str, "%hhx:%hhx:%hhx:%hhx:%hhx:%hhx",
               &mac[0], &mac[1], &mac[2], &mac[3], &mac[4], &mac[5]) != 6) {
        return ESP_FAIL;
    }
    return ESP_OK;
}

/* Load DHCP reservations (MAC-to-IP mapping) from JSON file */
static esp_err_t load_dhcp_reservations(void)
{
    FILE *f = fopen(DHCP_RESERVATIONS_FILE, "r");
    if (f == NULL) {
        ESP_LOGE(TAG, "Failed to open %s", DHCP_RESERVATIONS_FILE);
        return ESP_FAIL;
    }
    char buffer[512];
    size_t bytes_read = fread(buffer, 1, sizeof(buffer) - 1, f);
    buffer[bytes_read] = '\0';
    fclose(f);

    ESP_LOGI(TAG, "DHCP reservations file content: %s", buffer);
    cJSON *json = cJSON_Parse(buffer);
    if (json == NULL) {
        ESP_LOGE(TAG, "Error parsing DHCP reservations JSON");
        return ESP_FAIL;
    }

    cJSON *reservations = cJSON_GetObjectItemCaseSensitive(json, "reservations");
    if (!cJSON_IsArray(reservations)) {
        ESP_LOGE(TAG, "Reservations is not an array");
        cJSON_Delete(json);
        return ESP_FAIL;
    }

    reservation_count = 0;
    cJSON *reservation = NULL;
    cJSON_ArrayForEach(reservation, reservations) {
        if (reservation_count >= MAX_RESERVATIONS) {
            ESP_LOGW(TAG, "Maximum reservations reached");
            break;
        }
        cJSON *mac = cJSON_GetObjectItemCaseSensitive(reservation, "mac");
        cJSON *ip = cJSON_GetObjectItemCaseSensitive(reservation, "ip");
        if (cJSON_IsString(mac) && (mac->valuestring != NULL) &&
            cJSON_IsString(ip) && (ip->valuestring != NULL)) {

            if (parse_mac_string(mac->valuestring, reservation_table[reservation_count].mac) == ESP_OK) {
                if (inet_aton(ip->valuestring, (struct in_addr *)&reservation_table[reservation_count].reserved_ip)) {
                    ESP_LOGI(TAG, "Loaded reservation: MAC=%s, IP=%s", mac->valuestring, ip->valuestring);
                    reservation_count++;
                } else {
                    ESP_LOGE(TAG, "Invalid IP format: %s", ip->valuestring);
                }
            } else {
                ESP_LOGE(TAG, "Invalid MAC format: %s", mac->valuestring);
            }
        }
    }
    cJSON_Delete(json);
    return ESP_OK;
}

/* Check for a reserved IP for a given client MAC address */
ip4_addr_t get_reserved_ip_for_client(const uint8_t *client_mac)
{
    ip4_addr_t reserved_ip;
    IP4_ADDR(&reserved_ip, 0, 0, 0, 0);
    for (int i = 0; i < reservation_count; i++) {
        if (memcmp(client_mac, reservation_table[i].mac, 6) == 0) {
            ESP_LOGI(TAG, "Reservation match for client %02X:%02X:%02X:%02X:%02X:%02X: Reserved IP = " IPSTR,
                     client_mac[0], client_mac[1], client_mac[2],
                     client_mac[3], client_mac[4], client_mac[5],
                     IP2STR(&reservation_table[i].reserved_ip));
            return reservation_table[i].reserved_ip;
        }
    }
    return reserved_ip;
}

/* Dynamic lease table for non-reserved clients */
typedef struct {
    uint8_t mac[6];
    uint32_t ip; // in network order
} dynamic_lease_t;

static dynamic_lease_t dynamic_leases[MAX_DYNAMIC_LEASES] = {0};

static uint32_t get_dynamic_lease(const uint8_t *client_mac) {
    for (int i = 0; i < MAX_DYNAMIC_LEASES; i++) {
        if (memcmp(dynamic_leases[i].mac, client_mac, 6) == 0) {
            return dynamic_leases[i].ip;
        }
    }
    return 0;
}

static void set_dynamic_lease(const uint8_t *client_mac, uint32_t ip) {
    for (int i = 0; i < MAX_DYNAMIC_LEASES; i++) {
        if (memcmp(dynamic_leases[i].mac, client_mac, 6) == 0) {
            dynamic_leases[i].ip = ip;
            return;
        }
    }
    for (int i = 0; i < MAX_DYNAMIC_LEASES; i++) {
        if (dynamic_leases[i].ip == 0) {
            memcpy(dynamic_leases[i].mac, client_mac, 6);
            dynamic_leases[i].ip = ip;
            return;
        }
    }
}

/* DHCP packet structure (RFC 2131) */
typedef struct __attribute__((packed)) {
    uint8_t op;       /* 1 = BOOTREQUEST, 2 = BOOTREPLY */
    uint8_t htype;
    uint8_t hlen;
    uint8_t hops;
    uint32_t xid;
    uint16_t secs;
    uint16_t flags;
    uint32_t ciaddr;
    uint32_t yiaddr;
    uint32_t siaddr;
    uint32_t giaddr;
    uint8_t chaddr[16];
    char sname[64];
    char file[128];
    uint8_t options[312]; // Options field
} dhcp_packet_t;

/* DHCP Message Types */
#define DHCPDISCOVER 1
#define DHCPOFFER    2
#define DHCPREQUEST  3
#define DHCPACK      5

/* Parse DHCP Message Type (Option 53) */
static int get_dhcp_message_type(const uint8_t *options, size_t length) {
    if (length < 4) {
        ESP_LOGE(TAG, "Options length too short");
        return -1;
    }
    if (!(options[0] == 0x63 && options[1] == 0x82 &&
          options[2] == 0x53 && options[3] == 0x63)) {
        ESP_LOGE(TAG, "Magic cookie not found in DHCP options");
        return -1;
    }
    size_t i = 4;
    while (i < length) {
        uint8_t option_type = options[i];
        if (option_type == 255)
            break;
        if (option_type == 0) { i++; continue; }
        if (i + 1 >= length)
            break;
        uint8_t option_len = options[i+1];
        if (option_type == 53 && option_len == 1)
            return options[i+2];
        i += 2 + option_len;
    }
    return -1;
}

/* Build a DHCP reply packet and return total length */
static size_t build_dhcp_reply(const dhcp_packet_t *request, dhcp_packet_t *reply, uint32_t offered_ip, uint8_t dhcp_msg_type) {
    reply->op = 2; // BOOTREPLY
    reply->htype = request->htype;
    reply->hlen = request->hlen;
    reply->hops = 0;
    reply->xid = request->xid;
    reply->secs = 0;
    reply->flags = request->flags;
    reply->ciaddr = 0;
    reply->yiaddr = offered_ip;
    reply->siaddr = inet_addr("192.168.4.1");
    reply->giaddr = 0;
    memcpy(reply->chaddr, request->chaddr, 16);
    memset(reply->sname, 0, sizeof(reply->sname));
    memset(reply->file, 0, sizeof(reply->file));

    uint8_t *opt = reply->options;
    // Magic cookie
    memcpy(opt, "\x63\x82\x53\x63", 4);
    opt += 4;
    // Echo Option 61 (Client Identifier)
    *opt++ = 61;
    *opt++ = 7;
    *opt++ = 1;
    memcpy(opt, request->chaddr, 6);
    opt += 6;
    // DHCP Message Type (Option 53)
    *opt++ = 53; *opt++ = 1; *opt++ = dhcp_msg_type;
    // Server Identifier (Option 54)
    *opt++ = 54; *opt++ = 4;
    uint32_t server_ip = inet_addr("192.168.4.1");
    memcpy(opt, &server_ip, 4); opt += 4;
    // Lease Time (Option 51)
    *opt++ = 51; *opt++ = 4;
    uint32_t lease_time = htonl(3600);
    memcpy(opt, &lease_time, 4); opt += 4;
    // Renewal Time (Option 58)
    *opt++ = 58; *opt++ = 4;
    uint32_t renewal_time = htonl(1800);
    memcpy(opt, &renewal_time, 4); opt += 4;
    // Rebinding Time (Option 59)
    *opt++ = 59; *opt++ = 4;
    uint32_t rebinding_time = htonl(3150);
    memcpy(opt, &rebinding_time, 4); opt += 4;
    // Subnet Mask (Option 1)
    *opt++ = 1; *opt++ = 4;
    uint32_t subnet_mask = inet_addr("255.255.255.0");
    memcpy(opt, &subnet_mask, 4); opt += 4;
    // Router (Option 3)
    *opt++ = 3; *opt++ = 4;
    memcpy(opt, &server_ip, 4); opt += 4;
    // DNS Server (Option 6) -- Using 8.8.8.8
    *opt++ = 6; *opt++ = 4;
    uint32_t dns_ip = inet_addr("8.8.8.8");
    memcpy(opt, &dns_ip, 4); opt += 4;
    // End Option
    *opt++ = 255;

    size_t options_len = (size_t)(opt - reply->options);
    return 236 + options_len;
}

/* Global dynamic IP counter */
static uint32_t dynamic_ip_current;

/* Gratuitous ARP function */
static void send_gratuitous_arp(uint32_t offered_ip, const uint8_t *client_mac, esp_netif_t *ap_netif) {
    #define ETH_HDR_LEN 14
    #define ARP_HDR_LEN 28
    #define ARP_PKT_LEN (ETH_HDR_LEN + ARP_HDR_LEN)
    uint8_t arp_pkt[ARP_PKT_LEN];
    memset(arp_pkt, 0, sizeof(arp_pkt));

    struct {
        uint8_t dest[6];
        uint8_t src[6];
        uint16_t type;
    } __attribute__((packed)) *eth = (void *)arp_pkt;
    memset(eth->dest, 0xff, 6);
    memcpy(eth->src, client_mac, 6);
    eth->type = htons(0x0806);

    struct {
        uint16_t hwtype;
        uint16_t proto;
        uint8_t hwlen;
        uint8_t protolen;
        uint16_t opcode;
        uint8_t shwaddr[6];
        uint8_t sipaddr[4];
        uint8_t thwaddr[6];
        uint8_t tipaddr[4];
    } __attribute__((packed)) *arp = (void *)(arp_pkt + ETH_HDR_LEN);
    arp->hwtype = htons(1);
    arp->proto = htons(0x0800);
    arp->hwlen = 6;
    arp->protolen = 4;
    arp->opcode = htons(2);
    memcpy(arp->shwaddr, client_mac, 6);
    memcpy(arp->sipaddr, &offered_ip, 4);
    memcpy(arp->thwaddr, client_mac, 6);
    memcpy(arp->tipaddr, &offered_ip, 4);

    struct netif *lwip_netif = esp_netif_get_netif_impl(ap_netif);
    if (!lwip_netif) {
        ESP_LOGE(TAG, "Failed to get lwIP netif");
        return;
    }
    struct pbuf *p = pbuf_alloc(PBUF_RAW, ARP_PKT_LEN, PBUF_POOL);
    if (!p) {
        ESP_LOGE(TAG, "Failed to allocate pbuf for ARP packet");
        return;
    }
    pbuf_take(p, arp_pkt, ARP_PKT_LEN);
    err_t err = lwip_netif->linkoutput(lwip_netif, p);
    if (err != ERR_OK) {
        ESP_LOGE(TAG, "Failed to send gratuitous ARP, err: %d", err);
    } else {
        ESP_LOGI(TAG, "Sent gratuitous ARP for IP: %s", inet_ntoa(*(struct in_addr *)&offered_ip));
    }
    pbuf_free(p);
}

static void log_hex(const char *title, const uint8_t *data, size_t len) {
    char buf[256] = {0};
    size_t pos = 0;
    for (size_t i = 0; i < len && pos < sizeof(buf)-3; i++) {
        pos += sprintf(buf + pos, "%02X ", data[i]);
    }
    ESP_LOGI(TAG, "%s: %s", title, buf);
}

/* Custom DHCP server task */
static void dhcp_server_task(void *pvParameters) {
    int sock;
    struct sockaddr_in server_addr, client_addr, dest_addr;
    socklen_t addr_len = sizeof(client_addr);
    dhcp_packet_t packet;
    dhcp_packet_t reply;
    char offered_ip_str[16];
    const int header_size = 236; // Fixed DHCP header size

    // Obtain AP IP info so we bind to the correct interface.
    esp_netif_ip_info_t ap_ip_info = {0};
    if (esp_netif_get_ip_info(g_ap_netif, &ap_ip_info) != ESP_OK) {
        ESP_LOGE(TAG, "Failed to get AP IP info");
        vTaskDelete(NULL);
        return;
    }
    ESP_LOGI(TAG, "AP IP info: " IPSTR, IP2STR(&ap_ip_info.ip));

    sock = socket(AF_INET, SOCK_DGRAM, 0);
    if (sock < 0) {
        ESP_LOGE(TAG, "Failed to create socket");
        vTaskDelete(NULL);
        return;
    }
    int broadcast = 1;
    setsockopt(sock, SOL_SOCKET, SO_BROADCAST, &broadcast, sizeof(broadcast));

    memset(&server_addr, 0, sizeof(server_addr));
    server_addr.sin_family = AF_INET;
    server_addr.sin_addr.s_addr = ap_ip_info.ip.addr;
    server_addr.sin_port = htons(67);
    if (bind(sock, (struct sockaddr *)&server_addr, sizeof(server_addr)) < 0) {
        ESP_LOGE(TAG, "Failed to bind socket on AP IP");
        close(sock);
        vTaskDelete(NULL);
        return;
    }
    ESP_LOGI(TAG, "Custom DHCP server bound to AP IP");

    memset(&dest_addr, 0, sizeof(dest_addr));
    dest_addr.sin_family = AF_INET;
    dest_addr.sin_port = htons(68);
    dest_addr.sin_addr.s_addr = htonl(INADDR_BROADCAST);

    while (1) {
        int len = recvfrom(sock, &packet, sizeof(packet), 0,
                           (struct sockaddr *)&client_addr, &addr_len);
        if (len < 0) {
            ESP_LOGE(TAG, "Failed to receive packet");
            continue;
        }
        if (len < header_size) {
            ESP_LOGE(TAG, "Received packet too short: %d bytes", len);
            continue;
        }
        size_t req_options_len = len - header_size;
        int msg_type = get_dhcp_message_type(packet.options, req_options_len);
        if (msg_type < 0) {
            ESP_LOGE(TAG, "DHCP message type not found");
            continue;
        }
        uint8_t *client_mac = packet.chaddr;
        ESP_LOGI(TAG, "Received DHCP message type %d from %02X:%02X:%02X:%02X:%02X:%02X",
                 msg_type,
                 client_mac[0], client_mac[1], client_mac[2],
                 client_mac[3], client_mac[4], client_mac[5]);

        uint32_t offered_ip = 0;
        ip4_addr_t reserved = get_reserved_ip_for_client(client_mac);
        if (!ip4_addr_isany_val(reserved)) {
            offered_ip = reserved.addr;
        } else {
            offered_ip = get_dynamic_lease(client_mac);
            if (offered_ip == 0) {
                offered_ip = dynamic_ip_current;
                set_dynamic_lease(client_mac, offered_ip);
                dynamic_ip_current = htonl(ntohl(dynamic_ip_current) + 1);
            }
        }

        memset(&reply, 0, sizeof(reply));
        uint8_t reply_type = (msg_type == DHCPDISCOVER) ? DHCPOFFER : DHCPACK;
        size_t reply_len = build_dhcp_reply(&packet, &reply, offered_ip, reply_type);
        reply.flags = htons(0x8000); // Force broadcast flag
        if (reply_len < 300) {
            reply_len = 300;
        }
        ESP_LOGI(TAG, "Sending DHCP reply with length %d bytes", (int)reply_len);
        sendto(sock, &reply, reply_len, 0, (struct sockaddr *)&dest_addr, sizeof(dest_addr));
        ESP_LOGI(TAG, "Sent DHCP reply with offered IP: %s", inet_ntop(AF_INET, &offered_ip, offered_ip_str, sizeof(offered_ip_str)));
        log_hex("DHCP Reply Packet", (uint8_t *)&reply, reply_len);

        // Announce via gratuitous ARP
        if (g_ap_netif != NULL) {
            send_gratuitous_arp(offered_ip, client_mac, g_ap_netif);
        }
    }
    close(sock);
    vTaskDelete(NULL);
}

/* Initialize Wi‑Fi in AP+STA mode using esp_netif API */
static void wifi_init_ap_sta(void) {
    esp_netif_t *ap_netif = esp_netif_create_default_wifi_ap();
    esp_netif_t *sta_netif = esp_netif_create_default_wifi_sta();
    g_ap_netif = ap_netif; // AP interface for clients
    g_sta_netif = sta_netif; // STA interface for upstream

    // Set STA as default for outbound routing
    esp_netif_set_default_netif(g_sta_netif);

    wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
    ESP_ERROR_CHECK(esp_wifi_init(&cfg));

    // Configure AP (using loaded JSON config)
    wifi_config_t wifi_config_ap = {0};
    strncpy((char *)wifi_config_ap.ap.ssid, wifi_ap_config.ssid, sizeof(wifi_config_ap.ap.ssid));
    wifi_config_ap.ap.ssid_len = strlen(wifi_ap_config.ssid);
    strncpy((char *)wifi_config_ap.ap.password, wifi_ap_config.password, sizeof(wifi_config_ap.ap.password));
    wifi_config_ap.ap.channel = 1;
    wifi_config_ap.ap.max_connection = 32;
    wifi_config_ap.ap.authmode = WIFI_AUTH_WPA_WPA2_PSK;
    if (strlen(wifi_ap_config.password) == 0) {
        wifi_config_ap.ap.authmode = WIFI_AUTH_OPEN;
    }

    // Configure STA (parent network) using loaded credentials
    wifi_config_t wifi_config_sta = {0};
    strncpy((char *)wifi_config_sta.sta.ssid, sta_ssid, sizeof(wifi_config_sta.sta.ssid));
    strncpy((char *)wifi_config_sta.sta.password, sta_password, sizeof(wifi_config_sta.sta.password));

    ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_APSTA));
    ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_AP, &wifi_config_ap));
    ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_STA, &wifi_config_sta));
    ESP_ERROR_CHECK(esp_wifi_start());

    ESP_ERROR_CHECK(esp_wifi_connect());

    ESP_LOGI(TAG, "AP+STA mode started. AP SSID: %s, AP Password: %s", wifi_ap_config.ssid, wifi_ap_config.password);
    ESP_LOGI(TAG, "STA connecting to: %s", sta_ssid);

    /* Stop built-in DHCP server on AP interface */
    ESP_ERROR_CHECK(esp_netif_dhcps_stop(ap_netif));
    ESP_LOGI(TAG, "Built-in DHCP server stopped.");

    // Wait for STA to get an IP (in production use IP_EVENT_STA_GOT_IP handler)
    vTaskDelay(5000 / portTICK_PERIOD_MS);

    // Enable NAT on the AP interface using esp_netif API.
    // This call tells esp_netif to masquerade outbound packets from the AP.
    if (esp_netif_napt_enable(ap_netif) != ESP_OK) {
        ESP_LOGE(TAG, "NAPT not enabled on the AP interface");
    } else {
        ESP_LOGI(TAG, "NAPT enabled on the AP interface");
    }
}

/* WiFi configuration getter for other modules */
esp_err_t iotcraft_get_wifi_config(iotcraft_wifi_config_t *config)
{
    if (!config) {
        return ESP_ERR_INVALID_ARG;
    }
    
    strncpy(config->ssid, wifi_ap_config.ssid, sizeof(config->ssid) - 1);
    config->ssid[sizeof(config->ssid) - 1] = '\0';
    strncpy(config->password, wifi_ap_config.password, sizeof(config->password) - 1);
    config->password[sizeof(config->password) - 1] = '\0';
    
    return ESP_OK;
}

/* Main entry point */
void app_main(void) {
    ESP_ERROR_CHECK(nvs_flash_init());
    ESP_ERROR_CHECK(esp_netif_init());
    ESP_ERROR_CHECK(esp_event_loop_create_default());

    if (mount_littlefs() != ESP_OK) {
        ESP_LOGE(TAG, "LittleFS mount failed");
    }
    load_wifi_config();
    load_dhcp_reservations();

    wifi_init_ap_sta();

    /* Initialize dynamic IP pool starting at 192.168.4.2 */
    dynamic_ip_current = inet_addr("192.168.4.2");

    /* Start the custom DHCP server task on the AP interface */
    xTaskCreate(dhcp_server_task, "dhcp_server_task", 4096, NULL, 5, NULL);

    // Wait for DHCP server to be ready
    vTaskDelay(pdMS_TO_TICKS(2000));
    
    ESP_LOGI(TAG, "Starting IoTCraft Gateway services...");
    
    // Initialize mDNS service for service discovery
    esp_err_t ret = iotcraft_mdns_init();
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to initialize mDNS service: %s", esp_err_to_name(ret));
    }
    
    // Initialize MQTT broker
    ret = iotcraft_mqtt_broker_init();
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to initialize MQTT broker: %s", esp_err_to_name(ret));
    }
    
    // Initialize HTTP configuration server
    ret = iotcraft_http_server_init();
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to initialize HTTP server: %s", esp_err_to_name(ret));
    }
    
    // Initialize status GUI for local display
    ret = iotcraft_status_gui_init();
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to initialize status GUI: %s", esp_err_to_name(ret));
    }
    
    ESP_LOGI(TAG, "\n");
    ESP_LOGI(TAG, "IoTCraft Gateway is ready!");
    ESP_LOGI(TAG, "WiFi: %s (password: %s)", wifi_ap_config.ssid, wifi_ap_config.password);
    ESP_LOGI(TAG, "Gateway: 192.168.4.1 or iotcraft-gateway.local");
    ESP_LOGI(TAG, "MQTT: iotcraft-gateway.local:1883");
    ESP_LOGI(TAG, "Config: http://iotcraft-gateway.local/");
    ESP_LOGI(TAG, "Display: Local status GUI on ESP32-S3-BOX-3 screen");
    ESP_LOGI(TAG, "\n");
    ESP_LOGI(TAG, "Connect IoTCraft clients to this network for automatic discovery");
    ESP_LOGI(TAG, "\n");
}
