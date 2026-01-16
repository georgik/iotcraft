idf_component_register(
    SRCS
        "hello.c"
        "board_init.c"
        "world.c"
        "camera.c"
        "renderer.c"
        "input.c"
        "game.c"
        "usb_keyboard.c"
        "iotcraft_mqtt.c"
        "network_init.c"
        # "wifi_init.c"  # TODO: Implement WiFi after ESP-Hosted is tested
        "world_template.c"
        "trig_lut.c"
        "ppa_helper.c"
        "brightness.c"
        # Console overlay
        "console/console.c"
        "console/console_commands.c"
    INCLUDE_DIRS "." "console"
    PRIV_REQUIRES
        georgik__raylib
        usb_host_hid
        mqtt
        esp_eth
        esp_wifi
        esp_netif
        esp_driver_ppa
        driver
        # esp_raylib_port is pulled in via georgik__raylib dependency
        # esp_wifi_remote and esp_hosted are pulled in via idf_component.yml
)
