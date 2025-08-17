# Desktop Device Client

A desktop MQTT device client that simulates ESP32 device functionality for IoTCraft testing.

## Overview

This application acts as a virtual IoT device that mimics the behavior of the ESP32-C6 client but runs on desktop systems. It's designed to speed up testing of MQTT functionality without needing physical ESP32 devices.

## Features

- **Device Announcement**: Automatically announces itself on the `devices/announce` topic when connecting
- **Player Emulation**: Simulates multiplayer players with configurable movement patterns
- **Graceful Shutdown**: Sends offline announcement when CTRL+C is pressed, removing device from 3D world
- **Light Control**: Responds to ON/OFF commands on `home/{device_id}/light` topic
- **Position Updates**: Handles position updates from desktop client on `home/{device_id}/position/set` topic  
- **Device Types**: Supports both "lamp" and "door" device types
- **Movement Patterns**: Static, circular, and random movement patterns for player simulation
- **Enhanced Logging**: Comprehensive logging with emojis for easy status tracking
- **Configurable**: All parameters can be customized via command line arguments

## Usage

### Basic Usage

```bash
# Start with default settings (generates random device ID)
cargo run

# Start with specific device ID
cargo run -- --device-id my-test-lamp

# Start as a door device
cargo run -- --device-type door --device-id test-door-01

# Specify custom position
cargo run -- --x 5.0 --y 1.0 --z 10.0

# Connect to different MQTT broker
cargo run -- --host 192.168.1.100 --port 1883
```

### Command Line Options

```
Options:
  -d, --device-id <DEVICE_ID>           Device ID (if not provided, generates a random one)
  -t, --device-type <DEVICE_TYPE>       Device type (lamp or door) [default: lamp]
      --host <HOST>                     MQTT broker host [default: localhost]
      --port <PORT>                     MQTT broker port [default: 1883]
      --x <X>                           Initial X position [default: 1]
      --y <Y>                           Initial Y position [default: 0.5]
      --z <Z>                           Initial Z position [default: 2]
      --emulate-player                  Enable player emulation (publishes player poses)
      --player-id <PLAYER_ID>           Player ID for multiplayer (if not provided, generates a random one)
      --player-name <PLAYER_NAME>       Player name for multiplayer (defaults to system username)
      --world-id <WORLD_ID>             World ID for multiplayer [default: default]
      --movement-pattern <PATTERN>      Movement pattern (static, circle, random) [default: static]
  -h, --help                            Print help
```

## MQTT Topics

The desktop device client uses the same MQTT topic structure as the ESP32-C6 client:

- **`devices/announce`** - Device registration announcements (publishes)
- **`home/{device_id}/light`** - Light control commands (subscribes)
- **`home/{device_id}/position/set`** - Position updates (subscribes)

## Device Announcement Format

When connecting, the client publishes a JSON announcement:

```json
{
  "device_id": "desktop-a1b2c3d4",
  "device_type": "lamp",
  "state": "online",
  "location": {
    "x": 1.0,
    "y": 0.5, 
    "z": 2.0
  }
}
```

## Testing Workflow

1. **Start MQTT broker**:
   ```bash
   cd ../mqtt-server && cargo run
   ```

2. **Start desktop client**:
   ```bash
   cd ../desktop-client && cargo run
   ```

3. **Start virtual device**:
   ```bash
   cd ../desktop-device-client && cargo run
   ```

4. **Observe**:
   - The virtual device should appear in the desktop client's 3D world
   - Click on the device to toggle its light state
   - Drag the device to test position updates
   - Use console commands like `list` to see registered devices

## Player Emulation

The client can also emulate multiplayer players for testing the multiplayer system:

```bash
# Static player (stays in one place)
cargo run -- --emulate-player --player-name "TestPlayer" --x 10 --y 2 --z 10

# Player moving in a circle
cargo run -- --emulate-player --movement-pattern circle --player-name "CircleBot"

# Player with random movement
cargo run -- --emulate-player --movement-pattern random --player-name "RandomBot"

# Custom world and player settings
cargo run -- --emulate-player --world-id "test-world" --player-id "bot-001" --player-name "TestBot"
```

### Player Movement Patterns

- **`static`** (default): Player stays at initial position
- **`circle`**: Player moves in a 5-unit radius circle
- **`random`**: Player changes direction randomly every 3 seconds

### Player MQTT Topics

When player emulation is enabled, the client publishes to:
- **`iotcraft/worlds/{world_id}/players/{player_id}/pose`** - Player position and orientation updates (10 Hz)

### Player Pose Format

```json
{
  "player_id": "player-a1b2c3d4",
  "player_name": "TestPlayer",
  "pos": [10.0, 2.0, 10.0],
  "yaw": 1.57,
  "pitch": 0.0,
  "ts": 1692345678901
}
```

## Multiple Virtual Devices

You can run multiple instances to simulate multiple devices:

```bash
# Terminal 1 - Lamp device
cargo run -- --device-id virtual-lamp-01 --x 1 --z 1

# Terminal 2 - Door device  
cargo run -- --device-type door --device-id virtual-door-01 --x 3 --z 5

# Terminal 3 - Player emulation
cargo run -- --emulate-player --movement-pattern circle --player-name "CircleBot"

# Terminal 4 - Combined device and player
cargo run -- --device-id virtual-lamp-02 --emulate-player --player-name "LampBot" --x 5 --z 1
```

## Graceful Shutdown

The desktop device client supports graceful shutdown:

- **Press CTRL+C** to initiate shutdown
- The client will:
  1. Send an offline announcement to `devices/announce` topic
  2. Remove the device from the desktop client's 3D world
  3. Clean up MQTT connections
  4. Exit gracefully

Example shutdown log:
```
🛑 Received CTRL+C, initiating graceful shutdown...
📤 Offline announcement sent successfully
✅ Graceful shutdown completed
```

## Logging

Set the `RUST_LOG` environment variable to control log levels:

```bash
RUST_LOG=debug cargo run
RUST_LOG=info cargo run   # Default level
RUST_LOG=warn cargo run
```

The client uses emojis for easy visual parsing:
- 🚀 Startup and initialization
- 🔌 Device registration/deregistration
- 💡 Light state changes
- 📍 Position updates
- 📨 MQTT message reception
- ⚠️ Warnings and errors

## Integration with IoTCraft

This virtual device client is fully compatible with:

- Desktop client device management
- Console commands (`list`, `blink`, etc.)
- Device positioning system
- MQTT diagnostic tools
- MCP integration for WARP terminal

The virtual devices behave identically to physical ESP32 devices from the perspective of the desktop client.
