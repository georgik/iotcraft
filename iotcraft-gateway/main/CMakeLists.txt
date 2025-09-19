idf_component_register(
        SRCS 
            "esp32-dhcp-server.c"
            "iotcraft_mqtt.c"
            "iotcraft_mdns.c"
            "iotcraft_http.c"
            "iotcraft_status_gui.c"
        INCLUDE_DIRS "."
)
target_include_directories(${COMPONENT_LIB} PUBLIC "$ENV{IDF_PATH}/components/esp_netif/private_include")
