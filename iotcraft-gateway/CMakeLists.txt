cmake_minimum_required(VERSION 3.16)

# Set BUILD_BOARD for SDL3 BSP selection
set(ENV{BUILD_BOARD} "espressif__esp-box-3_noglib")

# Set SDKCONFIG_DEFAULTS for ESP-BOX-3 configuration
if(NOT DEFINED SDKCONFIG_DEFAULTS)
    set(SDKCONFIG_DEFAULTS "sdkconfig.defaults.esp-box-3;sdkconfig.defaults")
endif()

include($ENV{IDF_PATH}/tools/cmake/project.cmake)

# Define FD_SETSIZE to support 100 LWIP sockets + system file descriptors
# Need at least 100 sockets + 6 for system FDs = 106, using 128 for safety
add_compile_definitions(FD_SETSIZE=128)

project(iotcraft-gateway)

get_filename_component(configName "${CMAKE_BINARY_DIR}" NAME)
list(APPEND EXTRA_COMPONENT_DIRS "${CMAKE_SOURCE_DIR}/components/esp_littlefs")
littlefs_create_partition_image(assets assets FLASH_IN_PROJECT)
