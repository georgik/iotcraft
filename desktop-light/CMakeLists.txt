cmake_minimum_required(VERSION 3.16)
project(iotcraft_desktop C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -Wall -Wextra -g")

# Find Raylib
find_package(PkgConfig REQUIRED)
pkg_check_modules(RAYLIB REQUIRED raylib)
message(STATUS "Raylib found: ${RAYLIB_INCLUDE_DIRS}")
message(STATUS "Raylib libraries: ${RAYLIB_LIBRARIES}")

# Include directories
include_directories(
    ${RAYLIB_INCLUDE_DIRS}
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/mocks
    ../esp32-p4-lcd-ev-devkit-client/main  # Shared headers
)

# Add desktop build flag
add_definitions(-D__DESKTOP_BUILD__=1)

# Source files from ESP32 (shared code)
set(SHARED_SOURCES
    ../esp32-p4-lcd-ev-devkit-client/main/camera.c
    ../esp32-p4-lcd-ev-devkit-client/main/game.c
    ../esp32-p4-lcd-ev-devkit-client/main/input.c
    ../esp32-p4-lcd-ev-devkit-client/main/world.c
    ../esp32-p4-lcd-ev-devkit-client/main/world_template.c
    ../esp32-p4-lcd-ev-devkit-client/main/renderer.c
    ../esp32-p4-lcd-ev-devkit-client/main/trig_lut.c
)

# Mock sources (ESP-IDF replacements)
set(MOCK_SOURCES
    mocks/esp_log.c
    mocks/freertos_mock.c
)

# Main executable source
set(EXECUTABLE_SOURCES
    main.c
    cli_options.c
)

# Create executable
add_executable(iotcraft-desktop
    ${EXECUTABLE_SOURCES}
    ${SHARED_SOURCES}
    ${MOCK_SOURCES}
)

# Link libraries
target_link_libraries(iotcraft-desktop
    ${RAYLIB_LIBRARIES}
    pthread
    m
)

# Add Raylib library directory for linking
target_link_directories(iotcraft-desktop PUBLIC /opt/homebrew/lib)

# Print configuration summary
message(STATUS "")
message(STATUS "=== IotCraft Desktop Simulator Configuration ===")
message(STATUS "Build type:      ${CMAKE_BUILD_TYPE}")
message(STATUS "C Compiler:      ${CMAKE_C_COMPILER}")
message(STATUS "Raylib:          ${RAYLIB_LIBRARIES}")
message(STATUS "Shared sources:  ${SHARED_SOURCES}")
message(STATUS "==================================================")
message(STATUS "")
