cmake_minimum_required(VERSION 3.16)

# Set BUILD_BOARD for SDL3 BSP selection
set(ENV{BUILD_BOARD} "espressif__esp-box-3_noglib")

# Set SDKCONFIG_DEFAULTS for ESP-BOX-3 configuration
if(NOT DEFINED SDKCONFIG_DEFAULTS)
    set(SDKCONFIG_DEFAULTS "sdkconfig.defaults.esp-box-3;sdkconfig.defaults")
endif()

include($ENV{IDF_PATH}/tools/cmake/project.cmake)
project(iotcraft-gateway)

get_filename_component(configName "${CMAKE_BINARY_DIR}" NAME)
list(APPEND EXTRA_COMPONENT_DIRS "${CMAKE_SOURCE_DIR}/components/esp_littlefs")
littlefs_create_partition_image(assets assets FLASH_IN_PROJECT)
