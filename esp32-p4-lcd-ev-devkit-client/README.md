# ESP32-P4 IoTCraft Client

3D voxel world rendering client for ESP32-P4 Function EV Board with MIPI-DSI display.

## Overview

This project implements a real-time 3D voxel rendering engine on ESP32-P4, featuring:
- True 3D raycasting engine with multi-core rendering (Core 0 + Core 1)
- Ethernet and WiFi (ESP-Hosted) network connectivity
- MQTT integration for IoT device management and multiplayer
- USB keyboard input support
- On-device console overlay for diagnostics
- IoT device visualization and control (lamps, etc.)

## Hardware Requirements

- ESP32-P4 Function EV Board
- MIPI-DSI display (1024x600)
- Ethernet connection (recommended) or ESP32-C6 co-processor for WiFi
- USB keyboard (optional, for full control)

## Features

### 3D Rendering
- Custom raycasting engine optimized for ESP32-P4
- Multi-core rendering: Core 0 collects visible voxels, Core 1 renders them
- PPA hardware scaler for display upscaling
- Wireframe debug mode
- Occlusion culling and heightmap optimization

### Network & MQTT
- Ethernet support with DHCP
- WiFi support via ESP32-C6 + ESP-Hosted (when available)
- MQTT client for device communication
- Automatic device discovery via `devices/announce` topic
- Device position tracking and visualization

### IoT Device Integration
- Automatic device discovery and tracking (up to 32 devices)
- Real-time device visualization in 3D world
- Blink control via F4 key or console commands
- State synchronization (online/offline/blinking)
- Texture changes based on device state

### Console System
- In-game console overlay (F3 to toggle)
- Network diagnostics (IP, MQTT status)
- Device announcements and status
- Command execution system

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

### Build & Flash

```bash
idf.py build
idf.py flash monitor
```

## Controls

### Movement
- `W/A/S/D` - Move forward/left/backward
- `Q/E` - Fly up/down
- `Arrow Keys` - Look around

### System
- `F3` - Toggle console overlay
- `F4` - Toggle blink mode for all lamps
- `ESC` - Close console (if open)
- `F1` - Toggle wireframe debug mode

### Console Commands
Press `F3` to open console, then type:
- `help` - Show available commands
- `clear` - Clear console output
- `list` - List known devices

## MQTT Integration

### Topics

#### Subscribed
- `devices/announce` - Device discovery messages
- `home/{device_id}/position/set` - Device position updates
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
- `renderer.c` - 3D raycasting engine (Core 1 task)
- `world.c` - Voxel world storage and management
- `camera.c` - First-person camera controller
- `device_manager.c` - IoT device tracking and visualization
- `iotcraft_mqtt.c` - MQTT client and event handling
- `network_init.c` - Ethernet and WiFi initialization
- `console/` - On-screen console overlay system

### Multi-Core Rendering

The renderer uses FreeRTOS tasks to parallelize work:
- **Core 0 (Main Task)**: Game logic, input handling, voxel collection
- **Core 1 (Renderer Task)**: Sorts voxels by depth and renders them

Synchronization via binary semaphores ensures proper coordination.

### Memory Management

- Internal RAM for small allocations (< 200KB)
- PSRAM for framebuffer and large buffers (32MB external)
- Careful stack sizing to fit within available memory

## Performance

### Optimizations Implemented
- Heightmap-based occlusion culling
- Face culling (skip hidden faces)
- Multi-core voxel collection and sorting
- Fixed-point trigonometry lookup tables
- Reduced rendering resolution (512x300 → 1024x600 via PPA)
- Static const face data for block types

### Performance Considerations
- Device updates limited to 250ms intervals to reduce race conditions
- World modifications minimized during rendering
- Static allocation where possible to avoid heap fragmentation

## Troubleshooting

### Boot Issues
- If device hangs during boot, check ESP-Hosted initialization
- Disable WiFi in menuconfig if ESP32-C6 is not present: `Component Configuration` → `ESP-Hosted WiFi support`

### Network Issues
- Check Ethernet cable connection
- Verify DHCP server is accessible (should be at 192.168.4.1)
- Monitor console for network status messages

### MQTT Connection Failures
- Verify broker address in `iotcraft_mqtt.c` (default: 192.168.4.1:1883)
- Check firewall settings on MQTT broker
- Use console (F3) to view connection status

### Performance Issues
- Reduce view distance in camera setup
- Disable WiFi if not needed
- Use wireframe mode (F1) to debug rendering bottlenecks

## Development

### Adding New Device Types

1. Add device type to `device_type_t` enum in `device_manager.h`
2. Update JSON parsing in `iotcraft_mqtt.c` (parse_device_announcement)
3. Add rendering logic in `device_manager.c` (device_manager_update)

### Console Commands

To add new console commands:
1. Define command handler in `console_commands.c`
2. Register in `console_register_builtin_commands()`
3. Update help text if needed

### Network Protocols

MQTT is handled by the ESP-IDF MQTT component. For custom protocols:
- Use `esp_netif` for network interface
- Implement in `network_init.c` for initialization
- Register event handlers for connection state changes


## References

- [ESP32-P4 Technical Reference Manual](https://www.espressif.com/sites/default/files/documentation/esp32-p4_technical_reference_manual_en.pdf)
- [ESP-Hosted WiFi Stack](https://github.com/espressif/esp-hosted)
- ESP-IDF Programming Guide (v6.1)
