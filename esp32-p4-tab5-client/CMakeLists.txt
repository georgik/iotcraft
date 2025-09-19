cmake_minimum_required(VERSION 3.29)

if(CONFIG_SDL_BSP_ESP32_P4_FUNCTION_EV)
    message(STATUS "Detected ESP32-P4 Function EV Board configuration")
elseif(CONFIG_SDL_BSP_M5STACK_TAB5)
    message(STATUS "Detected M5Stack Tab5 (ESP32-P4) configuration")
elseif(CONFIG_SDL_BSP_ESP32_C6_DEVKIT)
    message(STATUS "Detected ESP32-C6 DevKit configuration")
elseif(CONFIG_SDL_BSP_ESP32_C3_LCDKIT)
    message(STATUS "Detected ESP32-C3 LCDkit configuration")
elseif(CONFIG_SDL_BSP_ESP_BOX_3)
    message(STATUS "Detected ESP32-S3-BOX-3 configuration")
elseif(CONFIG_SDL_BSP_ESP_BOX)
    message(STATUS "Detected ESP-Box configuration")
elseif(CONFIG_SDL_BSP_M5STACK_CORE_S3)
    message(STATUS "Detected M5Stack Core S3 configuration")
elseif(CONFIG_SDL_BSP_M5_ATOM_S3)
    message(STATUS "Detected M5 Atom S3 configuration")
else()
    message(STATUS "Board configuration will be detected at runtime")
endif()

set(COMPONENTS
    main
) # "Trim" the build. Include the minimal set of components; main and anything it depends on.

include($ENV{IDF_PATH}/tools/cmake/project.cmake)
project(iotcraft-esp32-p4-tab5-client)

get_filename_component(configName "${CMAKE_BINARY_DIR}" NAME)
list(APPEND EXTRA_COMPONENT_DIRS "${CMAKE_SOURCE_DIR}/components/esp_littlefs")
list(APPEND EXTRA_COMPONENT_DIRS "${CMAKE_SOURCE_DIR}/components")
littlefs_create_partition_image(assets assets FLASH_IN_PROJECT)
