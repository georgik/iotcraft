# ESP32-P4 IoTCraft Client

3D voxel world rendering client for ESP32-P4 Function EV Board with MIPI-DSI display.

## Overview

This project implements a real-time 3D voxel rendering engine on ESP32-P4, featuring:
- True 3D raycasting engine with multi-core rendering (Core 0 + Core 1)
- Ethernet and WiFi (ESP-Hosted) network connectivity
- MQTT integration for IoT device management and multiplayer
- USB keyboard input support with comprehensive key mapping
- On-device console overlay for diagnostics
- SD card texture loading with automatic fallback
- IoT device visualization and control (lamps, etc.)

## Hardware Requirements

- ESP32-P4 Function EV Board
- MIPI-DSI display (1024x600)
- Ethernet connection (recommended) or ESP32-C6 co-processor for WiFi
- USB keyboard (optional, for full control)
- microSD card (optional, for high-resolution textures)

## Features

### 3D Rendering
- Custom raycasting engine optimized for ESP32-P4
- Multi-core rendering: Core 0 collects visible voxels, Core 1 renders them
- PPA hardware scaler for display upscaling (2x performance boost)
- Wireframe debug mode (F6)
- Debug block visualization (F5) for testing 3D projection
- Occlusion culling and heightmap optimization
- Fixed-point math for CPU-efficient calculations

### Texture System
- **SD Card Support**: Load 16x16 RGB565 textures from microSD card
  - Automatic SD card detection on boot
  - Failsafe fallback to embedded 8x8 textures if SD card unavailable
  - Support for custom texture packs
- **Embedded Textures**: 8x8 procedural textures (grass, dirt, stone, quartz, glass, terracotta, water)
- **Dynamic Sizing**: Automatic upsaling of 8x8 textures to 16x16

### Network & MQTT
- **Dual Network Support**:
  - Ethernet (primary): 100Mbps with DHCP
  - WiFi (via ESP32-C6): ESP-Hosted SDIO transport
- **SD Card + ESP-Hosted Coexistence**: Properly configured to work simultaneously
- MQTT client for device communication
- Automatic device discovery via `devices/announce` topic
- Device position tracking and visualization

### IoT Device Integration
- Automatic device discovery and tracking (up to 32 devices)
- Real-time device visualization in 3D world
- Blink control via F4 key or console commands
- State synchronization (online/offline/blinking)
- Texture changes based on device state
- Position updates via MQTT

### Console System
- In-game console overlay (F3 to toggle)
- Network diagnostics (IP, MQTT status, WiFi signal)
- Device announcements and status
- Command execution system with extensible API
- Scrollable history with live updates

## Building

### Prerequisites

```bash
. $HOME/esp/esp-idf/export.sh  # or your ESP-IDF path
```

### Configuration

```bash
idf.py menuconfig
```

Key configuration options:
- `Board Configuration` → Select `ESP32-P4 Function EV Board`
- `Component Configuration` → `ESP-Hosted WiFi support` → Enable/Disable as needed
- `Component config` → `ESP32P4-Specific` → `Support for psram` → Enabled

### Build & Flash

```bash
idf.py build
idf.py flash monitor
```

### Flash Size

Current binary size: ~1.06 MB (66% of 3MB app partition free)

## Controls

### Movement
- `W/A/S/D` - Move forward/left/backward/right
- `Q/E` - Fly up/down
- `Arrow Keys` - Rotate camera view (up/down/left/right)

### Display & Debug
- `F3` - Toggle console overlay
- `F4` - Toggle blink mode for all lamps
- `F5` - Toggle debug block visualization (stone block in front of camera)
- `F6` - Toggle wireframe debug mode
- `N/M` - Decrease/increase display brightness (10% steps)

### Console Commands
Press `F3` to open console, then type:
- `help` - Show available commands
- `clear` - Clear console output
- `list` - List known devices
- `version` - Display firmware version information

## SD Card Texture Loading

### Supported Format
- **Filesystem**: FAT32
- **Texture Format**: RGB565 (16-bit color, 512 bytes per 16x16 texture)
- **Directory Structure**: `/sdcard/iotcraft/textures/`

### Required Files
```
/sdcard/iotcraft/textures/
├── grass.rgb565
├── dirt.rgb565
├── stone.rgb565
├── quartz_block.rgb565
├── glass_pane.rgb565
├── cyan_terracotta.rgb565
└── water.rgb565
```

### Failsafe Design
- If SD card is not present: Uses embedded 8x8 textures
- If texture file is missing: Uses embedded fallback for that block
- If SD card is corrupted: Falls back to embedded textures automatically
- No user intervention required - automatic detection on boot

### ESP-Hosted Compatibility
The SD card driver is specifically configured to work alongside ESP-Hosted WiFi:
- Uses SDMMC Slot 0 (SD card) independently from Slot 1 (ESP-Hosted)
- Proper LDO power control configuration
- IO MUX pin mode for optimal performance
- No conflicts with ESP32-C6 WiFi co-processor

See `esp-hosted-sdcard-problem.txt` for technical details on SDMMC configuration.

## MQTT Integration

### Topics

#### Subscribed
- `devices/announce` - Device discovery messages (auto-detect IoT devices)
- `home/{device_id}/position/set` - Device position updates (JSON coordinates)
- `iotcraft/worlds/{world_id}/state/blocks/*` - World synchronization

#### Published
- `home/{device_id}/light` - Lamp control (payload: "ON" or "OFF")
- `iotcraft/worlds/{world_id}/state/blocks/placed` - Block placement events
- `iotcraft/worlds/{world_id}/state/blocks/removed` - Block removal events

### Device Message Format

Device announcement JSON:
```json
{
  "device_id": "esp32-c6-791767f7",
  "device_type": "lamp",
  "state": "online",
  "location": {
    "x": 1.0,
    "y": 0.5,
    "z": 2.0
  }
}
```

## Architecture

### Core Components

- `hello.c` - Main application entry point and game loop
- `renderer.c` - 3D raycasting engine with multi-core support
- `world.c` - Voxel world storage and heightmap optimization
- `camera.c` - First-person camera controller
- `device_manager.c` - IoT device tracking and visualization
- `iotcraft_mqtt.c` - MQTT client and event handling
- `network_init.c` - Ethernet and WiFi (ESP-Hosted) initialization
- `sdcard_init.c` - SD card with ESP-Hosted coexistence support
- `texture_loader.c` - Texture loading from SD card or embedded fallback
- `console/` - On-screen console overlay system

### Multi-Core Rendering

The renderer uses FreeRTOS tasks to parallelize work:
- **Core 0 (Main Task)**: Game logic, input handling, voxel collection
- **Core 1 (Renderer Task)**: Sorts voxels by depth, renders textured quads

Synchronization via binary semaphores ensures proper coordination.

### Memory Management

- **Internal SRAM**: Small allocations (< 200KB)
  - Input state, camera data, world metadata
- **PSRAM** (32MB external): Framebuffer and texture buffers
  - RGB565 framebuffer at 512x300 = ~300KB
  - Texture cache: 7 textures × 512 bytes = 3.5KB
- Careful stack sizing to fit within available memory
- Static allocation where possible to avoid heap fragmentation

### Performance Optimizations

#### Rendering
- Heightmap-based occlusion culling (skip underground blocks)
- Face culling (skip faces hidden by adjacent blocks)
- Multi-core voxel collection and sorting
- Fixed-point trigonometry lookup tables (sin/cos)
- Reduced rendering resolution (512x300 → 1024x600 via PPA scaler)
- Static const face data for block types (compiled into Flash)
- Fixed-point projection math (2-3x faster than float on RISC-V)

#### Network
- Device updates limited to 250ms intervals (reduces race conditions)
- Asynchronous MQTT operations
- Non-blocking USB HID keyboard polling

#### Texture System
- Texture size dynamically detected (8x8 or 16x16)
- Nearest-neighbor upscaling (no interpolation overhead)
- Direct RGB565 format (no conversion needed)

## Performance

### Benchmarks
- **Frame Rate**: 15-20 FPS at 512x300 (upscaled to 1024x600)
- **Rendering Distance**: 20 blocks
- **Voxel Count**: ~2000 visible voxels per frame
- **Binary Size**: 1.06 MB (66% flash free)

### Performance Tips
- Reduce view distance in camera setup if needed
- Disable WiFi if not using (saves ~50ms boot time)
- Use wireframe mode (F6) to debug rendering bottlenecks
- Monitor console output for performance warnings

## Troubleshooting

### Boot Issues
- **Device hangs during boot**: Check ESP-Hosted initialization
- **No SD card detected**: Normal - falls back to embedded textures
- **App partition too small**: Increase in `partition_table` to 3MB minimum

### Network Issues
- **Ethernet not working**: Check cable connection, verify DHCP server at 192.168.4.1
- **WiFi not connecting**: Ensure ESP32-C6 is connected, check ESP-Hosted logs
- **MQTT connection fails**: Verify broker address and port in menuconfig

### SD Card Issues
- **SD card not detected**: Check card is formatted as FAT32
- **"no available sd host controller"**: SDMMC already initialized by ESP-Hosted (use provided workaround)
- **Textures not loading**: Verify file structure matches required format

### Display Issues
- **Screen too dark**: Press `M` key multiple times to increase brightness
- **Flickering artifacts**: Reduce PPA scale divisor or disable hardware scaling
- **Wrong colors**: Check RGB565 byte order (big-endian vs little-endian)

### Input Issues
- **F5/F6 keys not working**: Check serial monitor for key codes (some keyboards use non-standard codes)
- **USB keyboard not detected**: Verify USB host initialization in logs
- **Keys repeating**: Adjust USB HID polling interval

### Performance Issues
- **Low frame rate**: Reduce view distance or disable ESP-Hosted WiFi
- **Memory warnings**: Check heap memory in console, reduce texture cache
- **Crashes during rendering**: Disable multi-core rendering as fallback

## Development

### Adding New Device Types

1. Add device type to `device_type_t` enum in `device_manager.h`
2. Update JSON parsing in `iotcraft_mqtt.c` (parse_device_announcement)
3. Add rendering logic in `device_manager.c` (device_manager_update)

### Adding Console Commands

1. Define command handler in `console_commands.c`
2. Register in `console_register_builtin_commands()`
3. Update help text if needed

### Adding Textures

#### Embedded Textures
Edit `block_textures.c` and add 8x8 RGB565 texture array:
```c
const uint16_t texture_custom[64] = { ... };
```

#### SD Card Textures
1. Create 16x16 PNG image
2. Convert to RGB565 format (use provided conversion tool)
3. Save as `/sdcard/iotcraft/textures/custom.rgb565`
4. Update `texture_filenames[]` array in `texture_loader.c`

### Network Protocols

MQTT is handled by the ESP-IDF MQTT component. For custom protocols:
- Use `esp_netif` for network interface
- Implement in `network_init.c` for initialization
- Register event handlers for connection state changes

### Key Mapping

To add new key mappings:
1. Add key code to `iotcraft_key_code_t` enum in `input.h`
2. Add handler in `game_handle_key()` function in `game.c`
3. Update controls section in README

## Technical Documentation

### SD Card + ESP-Hosted Integration
See `esp-hosted-sdcard-problem.txt` for detailed technical documentation on:
- SDMMC peripheral architecture on ESP32-P4
- ESP-Hosted host initialization requirements
- LDO power control configuration
- IO MUX vs GPIO Matrix pin routing
- Common pitfalls and solutions

### Optimization Guides
- `OPTIMIZATION_PLAN.md` - Performance optimization roadmap
- `PPA_SCALING_IMPLEMENTATION.md` - Hardware upscaling implementation
- `FIXED_POINT_OPTIMIZATION.md` - Fixed-point math optimization
- `OCCLUSION_CULLING_OPTIMIZATION.md` - Visibility optimization

### Debug Features
- `WIREFRAME_DEBUG_MODE.md` - Wireframe rendering documentation
- `DEBUGGING_GUIDE.md` - Debugging techniques and tools
- `DEBUGGING_QUICKSTART.md` - Quick reference for common issues

## References

- [ESP32-P4 Technical Reference Manual](https://www.espressif.com/sites/default/files/documentation/esp32-p4_technical_reference_manual_en.pdf)
- [ESP-Hosted WiFi Stack](https://github.com/espressif/esp-hosted)
- [ESP32-P4 Function EV Board Guide](https://docs.espressif.com/projects/esp-dev-kits/en/latest/esp32p4/esp32-p4-function-ev-board/user_guide.html)
- ESP-IDF Programming Guide (v6.1)

## License

See LICENSE file in project root.

## Changelog

### Recent Updates
- **SD Card Texture Loading**: Automatic loading with embedded fallback
- **F5 Debug Block**: Visual debugging aid for 3D projection testing
- **F6 Wireframe Mode**: Easy toggle for wireframe rendering
- **Brightness Control**: Display brightness adjustment via N/M keys
- **ESP-Hosted Coexistence**: Proper SDMMC configuration for WiFi + SD card
- **Multi-Core Rendering**: Parallel voxel processing on Core 0 and Core 1
- **PPA Hardware Upscaling**: 2x display scaling with minimal CPU overhead
- **Fixed-Point Math**: CPU-efficient trigonometry and projection
- **Console System**: In-game diagnostics and command execution
