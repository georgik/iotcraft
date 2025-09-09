# IoTCraft ESP32-P4 Tab5 Client

**A high-performance voxel-based IoTCraft game client for the M5Stack Tab5, built with Swift and SDL3**

![Platform](https://img.shields.io/badge/platform-ESP32--P4-blue.svg)
![Swift](https://img.shields.io/badge/Swift-Embedded-orange.svg)
![SDL3](https://img.shields.io/badge/SDL-3.0-green.svg)
![ESP-IDF](https://img.shields.io/badge/ESP--IDF-6.0-red.svg)
![Status](https://img.shields.io/badge/status-Working-brightgreen.svg)

This is a specialized IoTCraft game client designed specifically for the **M5Stack Tab5** ESP32-P4 tablet. It features a top-down voxel world renderer with touchscreen interaction, IoT device control, and real-time world updates.

## ✨ Features

- 🎮 **Interactive IoTCraft World**: Top-down voxel world with block placement and IoT device interaction
- 📱 **Touch Interface**: Full capacitive touch support with GT911 controller
- 🖥️ **High-Resolution Display**: 720x1280 MIPI-DSI display with SDL3 hardware acceleration
- ⚡ **High Performance**: 200MHz PSRAM, 256KB L2 cache, dual-core RISC-V at 360MHz
- 🏠 **IoT Integration**: Control and monitor IoT devices within the game world
- 🌍 **Real-time Updates**: Dynamic world state with device status changes
- 🎨 **Rich Graphics**: SDL3-powered rendering with bitmap textures and TTF fonts
- 📁 **Asset System**: LittleFS-based asset storage for textures, fonts, and resources

## 🛠️ Requirements

- **Swift 6.1+** - [Download from swift.org](https://www.swift.org/install/)
- **ESP-IDF 6.0** - [ESP-IDF Installation Guide](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/get-started/)
- **M5Stack Tab5** - [Product Page](https://shop.m5stack.com/products/m5stack-tab5-esp32-p4-tablet)

## 🚀 Quick Start

### 1. Configure Build Environment

```bash
# Setup ESP-IDF
source /path/to/esp-idf/export.sh

# Optional: Set specific Swift toolchain (for development builds)
export TOOLCHAINS=$(plutil -extract CFBundleIdentifier raw /Library/Developer/Toolchains/swift-DEVELOPMENT-SNAPSHOT-2024-10-30-a.xctoolchain/Info.plist)
```

### 2. Build and Flash

```bash
# Build the project
idf.py @boards/m5stack_tab5.cfg build

# Flash to M5Stack Tab5
idf.py @boards/m5stack_tab5.cfg flash monitor
```

### 3. Interact with IoTCraft World

- **Touch the screen** to interact with IoT devices
- **Observe device states** changing in real-time
- **View the voxel world** rendered with different block types
- **Monitor console output** for device status updates

## 🎯 Target Hardware

**This project is specifically optimized for the M5Stack Tab5:**

| Specification | Details |
|---------------|--------|
| **MCU** | ESP32-P4 RISC-V Dual-Core @ 360MHz |
| **Memory** | 768KB SRAM + 32MB PSRAM @ 200MHz |
| **Display** | 5" IPS LCD, 720×1280, MIPI-DSI Interface |
| **Touch** | GT911 Capacitive Touch Controller |
| **Storage** | 16MB Flash + LittleFS for Assets |
| **Cache** | 256KB L2 Cache, 128B Cache Lines |
| **Graphics** | SDL3 with Hardware Acceleration |
| **Status** | ✅ **Fully Working** |

## 📋 Project Structure

```
esp32-p4-tab5-client/
├── main/                    # Swift application code
│   ├── Main.swift          # IoTCraft game logic & SDL3 integration
│   └── FileSystem.swift    # LittleFS asset management
├── components/
│   ├── sdl/                # Customized SDL3 component (ESP-IDF 6 compatible)
│   └── m5stack_tab5/       # M5Stack Tab5 BSP component
├── assets/                 # Game assets (fonts, textures)
├── boards/                 # Board-specific configurations
└── sdkconfig.*             # ESP-IDF configuration files
```

## 🎮 IoTCraft Game Features

### World Rendering
- **Top-down voxel view** with multiple block types:
  - 🌱 Grass blocks (forest green)
  - 🟫 Dirt blocks (saddle brown) 
  - ⚫ Stone blocks (gray)
  - ⬜ Quartz blocks (beige)
  - 🔷 Glass panes (light blue)
  - 🔷 Cyan terracotta (cadet blue)

### IoT Device Integration
- **Smart Lamps** 💡: Touch to toggle on/off (yellow when on)
- **Smart Doors** 🚪: Interactive door controls (brown)
- **Sensors** 📡: Environmental monitoring devices (green)
- **Real-time Status**: Devices change state automatically
- **Online/Offline Indication**: Visual feedback for device connectivity

### Touch Controls
- **Device Interaction**: Touch IoT devices to control them
- **Visual Feedback**: Devices highlight when touched
- **Capacitive Touch**: GT911 controller with precise input

## 🔧 Technical Implementation

### ESP-IDF 6.0 Compatibility Fixes
- ✅ **Native pthread support**: Removed duplicate SDL pthread declarations
- ✅ **Updated SPIRAM settings**: `CONFIG_FREERTOS_TASK_CREATE_ALLOW_EXT_MEM=y`
- ✅ **SDL3 driver fixes**: Corrected VideoBootStrap initialization
- ✅ **BSP integration**: Working M5Stack Tab5 display initialization

### Performance Optimizations
- 🚀 **200MHz PSRAM**: Critical for smooth graphics performance
- 🎯 **256KB L2 Cache**: Optimized cache configuration for ESP32-P4
- ⚡ **Swift Embedded**: No string interpolation runtime dependencies
- 🖥️ **Hardware Acceleration**: SDL3 with ESP-IDF video drivers

### Memory Configuration
```ini
# Critical PSRAM settings for high performance
CONFIG_SPIRAM=y
CONFIG_SPIRAM_SPEED_200M=y
CONFIG_SPIRAM_USE_MALLOC=y
CONFIG_SPIRAM_MALLOC_RESERVE_INTERNAL=32768
CONFIG_CACHE_L2_CACHE_256KB=y
CONFIG_CACHE_L2_CACHE_LINE_128B=y
```

## 🐛 Troubleshooting

### Common Issues

**Display not initializing:**
- Ensure proper PSRAM configuration (200MHz)
- Verify M5Stack Tab5 BSP component is properly loaded
- Check MIPI-DSI initialization in console output

**Touch not working:**
- GT911 touch controller should initialize automatically
- Check I2C communication in console logs
- Verify touch coordinates are being detected

**Build errors:**
- Ensure ESP-IDF 6.0 is properly installed
- Verify Swift 6.1+ toolchain
- Check component dependencies are resolved

**Performance issues:**
- PSRAM **must** run at 200MHz for smooth graphics
- Monitor memory usage in console
- Ensure L2 cache is properly configured

## 📚 Development Notes

### Key Architectural Decisions
- **Embedded Swift**: No standard library dependencies, optimized for microcontrollers
- **SDL3 Integration**: Direct hardware-accelerated graphics rendering
- **Component Architecture**: Modular design with customizable SDL and BSP components
- **Asset Management**: LittleFS filesystem for efficient asset storage
- **Memory Management**: Heap allocation preferred over stack for large objects

### ESP-IDF 6.0 Migration
This project has been fully updated for ESP-IDF 6.0 compatibility, including:
- Native pthread support (removed custom implementations)
- Updated SPIRAM configuration names
- Fixed SDL3 video driver initialization
- Resolved component dependency conflicts

## 🤝 Contributing

Contributions are welcome! Key areas for improvement:
- Additional IoT device types
- Enhanced graphics and effects
- Network multiplayer features
- More interactive world elements

## 📄 License

This project is part of the IoTCraft ecosystem. See the main IoTCraft repository for licensing information.

## 🙏 Credits

- **IoTCraft Project**: Part of the larger IoTCraft voxel game ecosystem
- **M5Stack**: Tab5 hardware platform and BSP components
- **Espressif**: ESP-IDF framework and ESP32-P4 platform
- **SDL3**: Cross-platform graphics and input library
- **Swift Community**: Embedded Swift development
- **Assets**: FreeSans.ttf font and game textures
