#include "iotcraft_gateway.h"
#include "esp_log.h"
#include "esp_http_server.h"
#include "cJSON.h"
#include <sys/param.h>  // For MIN macro

static const char *TAG = "IOTCRAFT_HTTP";
static httpd_handle_t http_server = NULL;

// HTML template for the configuration page
static const char* config_html = 
"<!DOCTYPE html>\n"
"<html>\n"
"<head>\n"
"    <title>IoTCraft Gateway Configuration</title>\n"
"    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n"
"    <style>\n"
"        body { font-family: Arial, sans-serif; margin: 20px; background-color: #f5f5f5; }\n"
"        .container { max-width: 800px; margin: 0 auto; background-color: white; padding: 20px; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }\n"
"        h1 { color: #2c3e50; text-align: center; }\n"
"        .status-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px; margin: 20px 0; }\n"
"        .status-card { background-color: #ecf0f1; padding: 15px; border-radius: 5px; text-align: center; }\n"
"        .status-active { background-color: #d5f4e6; }\n"
"        .status-inactive { background-color: #fadbd8; }\n"
"        .form-section { margin: 20px 0; padding: 15px; border: 1px solid #ddd; border-radius: 5px; }\n"
"        input, textarea, select { width: 100%; padding: 8px; margin: 5px 0; border: 1px solid #ddd; border-radius: 3px; box-sizing: border-box; }\n"
"        button { background-color: #3498db; color: white; padding: 10px 20px; border: none; border-radius: 5px; cursor: pointer; margin: 5px; }\n"
"        button:hover { background-color: #2980b9; }\n"
"        .save-btn { background-color: #27ae60; }\n"
"        .save-btn:hover { background-color: #219a52; }\n"
"        .form-row { display: flex; gap: 10px; align-items: center; }\n"
"        .form-row label { min-width: 120px; }\n"
"        .password-field { position: relative; }\n"
"        .toggle-password { position: absolute; right: 10px; top: 50%; transform: translateY(-50%); cursor: pointer; background: #f0f0f0; border: 1px solid #ccc; padding: 2px 6px; font-size: 12px; border-radius: 3px; }\n"
"        .toggle-password:hover { background: #e0e0e0; }\n"
"    </style>\n"
"</head>\n"
"<body>\n"
"    <div class=\"container\">\n"
"        <h1>IoTCraft Gateway</h1>\n"
"        <div class=\"status-grid\">\n"
"            <div class=\"status-card status-active\">\n"
"                <h3>WiFi Router</h3>\n"
"                <p>Active - DHCP Running</p>\n"
"            </div>\n"
"            <div class=\"status-card status-active\">\n"
"                <h3>MQTT Broker</h3>\n"
"                <p>Port 1883 - Ready</p>\n"
"            </div>\n"
"            <div class=\"status-card status-active\">\n"
"                <h3>DNS Service</h3>\n"
"                <p>iotcraft-gateway.local</p>\n"
"            </div>\n"
"            <div class=\"status-card status-active\">\n"
"                <h3>Configuration</h3>\n"
"                <p>Web Interface Active</p>\n"
"            </div>\n"
"        </div>\n"
"        \n"
"        <div class=\"form-section\">\n"
"            <h3>WiFi Access Point Configuration</h3>\n"
"            <form id=\"apForm\">\n"
"                <div class=\"form-row\">\n"
"                    <label for=\"ap_ssid\">Network Name (SSID):</label>\n"
"                    <input type=\"text\" id=\"ap_ssid\" name=\"ap_ssid\" value=\"iotcraft\" maxlength=\"31\" required>\n"
"                </div>\n"
"                <div class=\"form-row\">\n"
"                    <label for=\"ap_password\">Password:</label>\n"
"                    <div class=\"password-field\">\n"
"                        <input type=\"password\" id=\"ap_password\" name=\"ap_password\" value=\"iotcraft123\" minlength=\"8\" maxlength=\"63\" required>\n"
"                        <button type=\"button\" class=\"toggle-password\" onclick=\"togglePassword('ap_password')\">Show</button>\n"
"                    </div>\n"
"                </div>\n"
"                <button type=\"submit\" class=\"save-btn\">Save AP Configuration</button>\n"
"            </form>\n"
"        </div>\n"
"        \n"
"        <div class=\"form-section\">\n"
"            <h3>Parent Network Configuration</h3>\n"
"            <p><small>Connect this gateway to an existing WiFi network for internet access</small></p>\n"
"            <form id=\"staForm\">\n"
"                <div class=\"form-row\">\n"
"                    <label for=\"sta_ssid\">Network Name (SSID):</label>\n"
"                    <input type=\"text\" id=\"sta_ssid\" name=\"sta_ssid\" value=\"\" maxlength=\"31\">\n"
"                </div>\n"
"                <div class=\"form-row\">\n"
"                    <label for=\"sta_password\">Password:</label>\n"
"                    <div class=\"password-field\">\n"
"                        <input type=\"password\" id=\"sta_password\" name=\"sta_password\" value=\"\" maxlength=\"63\">\n"
"                        <button type=\"button\" class=\"toggle-password\" onclick=\"togglePassword('sta_password')\">Show</button>\n"
"                    </div>\n"
"                </div>\n"
"                <button type=\"submit\" class=\"save-btn\">Save Parent Network</button>\n"
"            </form>\n"
"        </div>\n"
"        \n"
"        <div class=\"form-section\">\n"
"            <h3>Network Information</h3>\n"
"            <p><strong>Gateway IP:</strong> 192.168.4.1</p>\n"
"            <p><strong>DHCP Range:</strong> 192.168.4.2 - 192.168.4.254</p>\n"
"            <p><strong>MQTT Broker:</strong> iotcraft-gateway.local:1883</p>\n"
"            <p><strong>DNS Names:</strong></p>\n"
"            <ul>\n"
"                <li>iotcraft-gateway.local (this interface)</li>\n"
"                <li>iotcraft-gateway.local:1883 (MQTT broker)</li>\n"
"            </ul>\n"
"        </div>\n"
"        \n"
"        <div class=\"form-section\">\n"
"            <h3>Quick Actions</h3>\n"
"            <button onclick=\"location.reload()\">Refresh Status</button>\n"
"            <button onclick=\"showMqttHelp()\">MQTT Topics</button>\n"
"            <button onclick=\"showHelp()\">Help</button>\n"
"            <button onclick=\"restartGateway()\" style=\"background-color: #e74c3c;\">Restart Gateway</button>\n"
"        </div>\n"
"    </div>\n"
"    \n"
"    <script>\n"
"    function togglePassword(fieldId) {\n"
"        const field = document.getElementById(fieldId);\n"
"        const button = event.target;\n"
"        if (field.type === 'password') {\n"
"            field.type = 'text';\n"
"            button.textContent = 'Hide';\n"
"        } else {\n"
"            field.type = 'password';\n"
"            button.textContent = 'Show';\n"
"        }\n"
"    }\n"
"    \n"
"    function showMqttHelp() {\n"
"        alert('MQTT Topics:\\n' +\n"
"              'iotcraft/worlds/+/info - World information\\n' +\n"
"              'iotcraft/worlds/+/data - World data\\n' +\n"
"              'iotcraft/devices/+/status - Device status\\n' +\n"
"              'iotcraft/gateway/status - Gateway status');\n"
"    }\n"
"    \n"
"    function showHelp() {\n"
"        alert('IoTCraft Gateway Help:\\n' +\n"
"              '1. Connect IoTCraft clients to this WiFi network\\n' +\n"
"              '2. Clients will auto-discover the MQTT broker\\n' +\n"
"              '3. Use parent network for internet access\\n' +\n"
"              '4. Access this interface at iotcraft-gateway.local');\n"
"    }\n"
"    \n"
"    function restartGateway() {\n"
"        if (confirm('Are you sure you want to restart the gateway? This will disconnect all clients.')) {\n"
"            fetch('/api/restart', {method: 'POST'}).then(() => {\n"
"                alert('Gateway is restarting. Please wait 30 seconds then refresh this page.');\n"
"            });\n"
"        }\n"
"    }\n"
"    \n"
"    document.getElementById('apForm').addEventListener('submit', function(e) {\n"
"        e.preventDefault();\n"
"        const formData = new FormData(this);\n"
"        const data = Object.fromEntries(formData);\n"
"        \n"
"        fetch('/api/config/ap', {\n"
"            method: 'POST',\n"
"            headers: {'Content-Type': 'application/json'},\n"
"            body: JSON.stringify(data)\n"
"        }).then(response => response.json())\n"
"          .then(data => {\n"
"              if (data.success) {\n"
"                  alert('AP configuration saved! The gateway will restart to apply changes.');\n"
"              } else {\n"
"                  alert('Error saving configuration: ' + data.error);\n"
"              }\n"
"          });\n"
"    });\n"
"    \n"
"    document.getElementById('staForm').addEventListener('submit', function(e) {\n"
"        e.preventDefault();\n"
"        const formData = new FormData(this);\n"
"        const data = Object.fromEntries(formData);\n"
"        \n"
"        fetch('/api/config/sta', {\n"
"            method: 'POST',\n"
"            headers: {'Content-Type': 'application/json'},\n"
"            body: JSON.stringify(data)\n"
"        }).then(response => response.json())\n"
"          .then(data => {\n"
"              if (data.success) {\n"
"                  alert('Parent network configuration saved! The gateway will restart to apply changes.');\n"
"              } else {\n"
"                  alert('Error saving configuration: ' + data.error);\n"
"              }\n"
"          });\n"
"    });\n"
"    </script>\n"
"</body>\n"
"</html>";

// Handler for the root page
static esp_err_t root_get_handler(httpd_req_t *req)
{
    ESP_LOGI(TAG, "Serving configuration page");
    
    httpd_resp_set_type(req, "text/html");
    httpd_resp_send(req, config_html, strlen(config_html));
    
    return ESP_OK;
}

// Handler for status API
static esp_err_t status_get_handler(httpd_req_t *req)
{
    ESP_LOGI(TAG, "Serving status API");
    
    // Create JSON response with gateway status
    cJSON *json = cJSON_CreateObject();
    cJSON *services = cJSON_CreateObject();
    
    cJSON_AddBoolToObject(services, "dhcp", true);  // Always true if we got here
    cJSON_AddBoolToObject(services, "mqtt", true);  // TODO: get actual MQTT status
    cJSON_AddBoolToObject(services, "mdns", true);  // TODO: get actual mDNS status
    cJSON_AddBoolToObject(services, "http", true);  // Always true if we got here
    
    cJSON_AddItemToObject(json, "services", services);
    cJSON_AddStringToObject(json, "gateway_ip", "192.168.4.1");
    cJSON_AddStringToObject(json, "mqtt_broker", "iotcraft-gateway.local:1883");
    cJSON_AddStringToObject(json, "version", "1.0.0");
    
    char *json_string = cJSON_Print(json);
    
    httpd_resp_set_type(req, "application/json");
    httpd_resp_send(req, json_string, strlen(json_string));
    
    free(json_string);
    cJSON_Delete(json);
    
    return ESP_OK;
}

// Handler for AP configuration
static esp_err_t config_ap_post_handler(httpd_req_t *req)
{
    ESP_LOGI(TAG, "Received AP configuration request");
    
    // Buffer for receiving POST data
    char content[512];
    size_t recv_size = MIN(req->content_len, sizeof(content) - 1);
    
    int ret = httpd_req_recv(req, content, recv_size);
    if (ret <= 0) {
        httpd_resp_send_err(req, HTTPD_400_BAD_REQUEST, "Failed to receive data");
        return ESP_FAIL;
    }
    content[ret] = '\0';
    
    // Parse JSON
    cJSON *json = cJSON_Parse(content);
    if (!json) {
        httpd_resp_send_err(req, HTTPD_400_BAD_REQUEST, "Invalid JSON");
        return ESP_FAIL;
    }
    
    cJSON *ssid = cJSON_GetObjectItemCaseSensitive(json, "ap_ssid");
    cJSON *password = cJSON_GetObjectItemCaseSensitive(json, "ap_password");
    
    if (!cJSON_IsString(ssid) || !cJSON_IsString(password)) {
        cJSON_Delete(json);
        httpd_resp_send_err(req, HTTPD_400_BAD_REQUEST, "Missing SSID or password");
        return ESP_FAIL;
    }
    
    // TODO: Save AP configuration to file and apply
    ESP_LOGI(TAG, "New AP config - SSID: %s, Password: %s", ssid->valuestring, password->valuestring);
    
    // Send success response
    cJSON *response = cJSON_CreateObject();
    cJSON_AddBoolToObject(response, "success", true);
    cJSON_AddStringToObject(response, "message", "AP configuration saved");
    
    char *response_string = cJSON_Print(response);
    httpd_resp_set_type(req, "application/json");
    httpd_resp_send(req, response_string, strlen(response_string));
    
    free(response_string);
    cJSON_Delete(response);
    cJSON_Delete(json);
    
    return ESP_OK;
}

// Handler for STA configuration
static esp_err_t config_sta_post_handler(httpd_req_t *req)
{
    ESP_LOGI(TAG, "Received STA configuration request");
    
    // Buffer for receiving POST data
    char content[512];
    size_t recv_size = MIN(req->content_len, sizeof(content) - 1);
    
    int ret = httpd_req_recv(req, content, recv_size);
    if (ret <= 0) {
        httpd_resp_send_err(req, HTTPD_400_BAD_REQUEST, "Failed to receive data");
        return ESP_FAIL;
    }
    content[ret] = '\0';
    
    // Parse JSON
    cJSON *json = cJSON_Parse(content);
    if (!json) {
        httpd_resp_send_err(req, HTTPD_400_BAD_REQUEST, "Invalid JSON");
        return ESP_FAIL;
    }
    
    cJSON *ssid = cJSON_GetObjectItemCaseSensitive(json, "sta_ssid");
    cJSON *password = cJSON_GetObjectItemCaseSensitive(json, "sta_password");
    
    if (!cJSON_IsString(ssid) || !cJSON_IsString(password)) {
        cJSON_Delete(json);
        httpd_resp_send_err(req, HTTPD_400_BAD_REQUEST, "Missing SSID or password");
        return ESP_FAIL;
    }
    
    // TODO: Save STA configuration to file and apply
    ESP_LOGI(TAG, "New STA config - SSID: %s, Password: %s", ssid->valuestring, password->valuestring);
    
    // Send success response
    cJSON *response = cJSON_CreateObject();
    cJSON_AddBoolToObject(response, "success", true);
    cJSON_AddStringToObject(response, "message", "STA configuration saved");
    
    char *response_string = cJSON_Print(response);
    httpd_resp_set_type(req, "application/json");
    httpd_resp_send(req, response_string, strlen(response_string));
    
    free(response_string);
    cJSON_Delete(response);
    cJSON_Delete(json);
    
    return ESP_OK;
}

esp_err_t iotcraft_http_server_init(void)
{
    if (http_server != NULL) {
        ESP_LOGW(TAG, "HTTP server already running");
        return ESP_OK;
    }
    
    httpd_config_t config = HTTPD_DEFAULT_CONFIG();
    config.server_port = 80;
    config.max_uri_handlers = 10;
    
    // Start the HTTP server
    esp_err_t ret = httpd_start(&http_server, &config);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to start HTTP server: %s", esp_err_to_name(ret));
        return ret;
    }
    
    // Register URI handlers
    httpd_uri_t root_uri = {
        .uri = "/",
        .method = HTTP_GET,
        .handler = root_get_handler,
        .user_ctx = NULL
    };
    httpd_register_uri_handler(http_server, &root_uri);
    
    httpd_uri_t status_uri = {
        .uri = "/api/status",
        .method = HTTP_GET,
        .handler = status_get_handler,
        .user_ctx = NULL
    };
    httpd_register_uri_handler(http_server, &status_uri);
    
    // Register WiFi configuration endpoints
    httpd_uri_t config_ap_uri = {
        .uri = "/api/config/ap",
        .method = HTTP_POST,
        .handler = config_ap_post_handler,
        .user_ctx = NULL
    };
    httpd_register_uri_handler(http_server, &config_ap_uri);
    
    httpd_uri_t config_sta_uri = {
        .uri = "/api/config/sta",
        .method = HTTP_POST,
        .handler = config_sta_post_handler,
        .user_ctx = NULL
    };
    httpd_register_uri_handler(http_server, &config_sta_uri);
    
    ESP_LOGI(TAG, "HTTP configuration server started on port 80");
    ESP_LOGI(TAG, "Access via: http://192.168.4.1/ or http://iotcraft-gateway.local/");
    
    return ESP_OK;
}

esp_err_t iotcraft_http_server_stop(void)
{
    if (http_server == NULL) {
        return ESP_OK;
    }
    
    esp_err_t ret = httpd_stop(http_server);
    http_server = NULL;
    
    ESP_LOGI(TAG, "HTTP server stopped");
    return ret;
}

bool iotcraft_http_is_running(void)
{
    return http_server != NULL;
}
