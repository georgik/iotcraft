# IoTCraft Game Controller

## Overview

This assignment demonstrates how ESP32-C3 can act as a game controller for the IoTCraft 3D voxel world. The device uses a joystick for movement control and sends real-time position updates via MQTT to the desktop client, where a 3D player avatar moves accordingly.

**Features:**
- WiFi connectivity (espradio)
- MQTT pose publishing (10 Hz continuous updates)
- Joystick input reading (ADC on GPIO4/GPIO6)
- JSON serialization for pose messages
- Integration with IoTCraft desktop client
- Real-time 3D avatar control

## Board Support

- **ESP32-C3**: esp32c3-generic target (primary, fully supported)
- **ESP32-S3**: Not supported (known ws2812 driver issue)
- **ESP32**: Not supported (espradio requires ESP32-C3/S3)

## Hardware Configuration

### Components

- ESP32-C3 board
- Joystick module (dual-axis potentiometer)
- WiFi connection

### Joystick Wiring

| Axis   | ESP32-C3 GPIO | ADC Pin       | ADC Channel |
|--------|---------------|---------------|-------------|
| X-axis | GPIO4         | machine.ADC4  | ADC1_CH0    |
| Y-axis | GPIO6         | machine.ADC6  | ADC1_CH2    |

**Joystick Module Wiring:**
```
Joystick Module          ESP32-C3
┌─────────────┐         ┌──────────┐
│ VCC         ├─────────┤ 3.3V     │
│ GND         ├─────────┤ GND      │
│ VRX (X-axis)├─────────┤ GPIO4    │
│ VRY (Y-axis)├─────────┤ GPIO6    │
└─────────────┘         └──────────┘
```

**Note:** GPIO4 and GPIO6 use ADC1 which has no XTAL constraints on ESP32-C3. Center position: ~32760, Deadzone: ±10000.

## Network Configuration

**WiFi Credentials:**
- SSID: `tinygo`
- Password: `gophercamp`

**MQTT Broker:**
- Address: `192.168.1.100:1883` (local network)
- Protocol: MQTT v3.1.1
- QoS: QoS0 (fire and forget)

## Prerequisites

Before flashing, download required dependencies:

```bash
go mod download
```

## Building and Flashing

### ESP32-C3

```bash
tinygo flash -target esp32c3-generic main.go
```

## How It Works

### Architecture

```
Joystick (ADC) → Position Update → JSON Serialization → MQTT Publish
                                              ↓
                              Desktop Client (Bevy 3D)
                                              ↓
                              3D Voxel Avatar Movement
```

### MQTT Protocol

**1. Device Announcement:**
- Topic: `devices/announce`
- Sent: Once on startup
- Payload: JSON with device info

**2. Pose Updates:**
- Topic: `player/{player_id}/pose`
- Frequency: 10 Hz (every 100ms)
- Payload: JSON with position data

### PoseMessage Structure

```json
{
  "player_id": "esp32c3-player-A1B2C3",
  "player_name": "ESP32-C3 Player",
  "pos": [10.5, 20.0, 15.3],
  "yaw": 0.0,
  "pitch": 0.0,
  "ts": 1712345678900
}
```

**Fields:**
- `player_id`: Unique device identifier
- `player_name`: Display name
- `pos`: [x, y, z] world coordinates (float)
- `yaw`: Horizontal rotation (radians, 0 for joystick-only)
- `pitch`: Vertical rotation (radians, 0 for joystick-only)
- `ts`: Unix timestamp (milliseconds)

## Testing

### 1. Setup IoTCraft Environment

**Start MQTT broker:**
```bash
cd /Volumes/ssdt5/Users/georgik/projects/iotcraft/mqtt-server
cargo run
```

**Start desktop client:**
```bash
cd /Volumes/ssdt5/Users/georgik/projects/iotcraft/desktop-client
cargo run
```

### 2. Flash ESP32-C3

```bash
cd assignment_9
tinygo flash -target esp32c3-generic main.go
```

### 3. Monitor Device

```bash
tinygo monitor -target esp32c3-generic
```

Expected output:
```
ESP32-C3 IoTCraft Player
==========================

Initializing joystick...
Joystick ready
Connecting to WiFi...
WiFi connected!
IP Address: 192.168.1.50
Player ID: esp32c3-player-A1B2C3
TCP connected to 192.168.1.100:1883
MQTT connected
Device announcement sent

Starting game loop...
Use joystick to move in 3D world
Desktop client will spawn your avatar

Position: X=0.50 Y=20.00 Z=0.00
Position: X=1.00 Y=20.50 Z=0.00
Position: X=1.50 Y=21.00 Z=0.00
...
```

### 4. Verify Integration

**In desktop client:**
1. Player avatar appears at position (0, 20, 0)
2. Avatar moves as joystick is moved
3. Movement is smooth at 10 Hz update rate
4. Multiple players can play simultaneously

**Monitor MQTT traffic:**
```bash
# Monitor all player positions
mosquitto_sub -h 192.168.1.100 -t "player/+/pose"

# Monitor device announcements
mosquitto_sub -h 192.168.1.100 -t "devices/announce"
```

## Technical Details

### Movement Calculation

1. **Joystick Reading:**
   - Raw ADC: 0-65520
   - Center: ~32760
   - Deadzone: ±10000

2. **Direction Calculation:**
   ```go
   dirX = (rawX - center) / center  // -1.0 to 1.0
   dirY = (rawY - center) / center  // -1.0 to 1.0
   ```

3. **Position Update:**
   ```go
   posX += dirX * MOVEMENT_SPEED  // 0.2 units per update
   posY += dirY * MOVEMENT_SPEED
   ```

4. **Update Rates:**
   - Joystick read: 20 Hz (50ms)
   - Pose publish: 10 Hz (100ms)

### JSON Serialization

TinyGo doesn't have `encoding/json`, so we use manual string concatenation:

```go
poseMsg := `{"player_id":"` + playerId + `","player_name":"ESP32-C3 Player","pos":[` +
    floatToString(j.posX) + `,` +
    floatToString(j.posY) + `,` +
    floatToString(j.posZ) + `],"yaw":0.0,"pitch":0.0,"ts":` +
    uint32ToString(timestamp) + `}`
```

### Memory Usage

- MQTT buffer: 1500 bytes (MTU)
- JSON buffer: ~200 bytes per message
- Stack: Minimal (~2KB recommended)
- Heap: ~20KB for network buffers

## Customization

### Change WiFi Credentials

```go
var (
    ssid     string = "your-ssid"
    password string = "your-password"
    broker   string = "192.168.1.100:1883"
)
```

### Change Movement Speed

```go
const (
    MOVEMENT_SPEED = 0.5  // Faster movement
)
```

### Change Update Rate

```go
const (
    POSE_UPDATE_RATE = 50  // 20 Hz (faster, more bandwidth)
)
```

### Add Jump Functionality

Add a button GPIO for vertical movement:

```go
const (
    JUMP_BUTTON = machine.GPIO7
)

// In main loop:
if buttonPressed() {
    joystick.posZ += 1.0  // Jump up
}
```

### Change Starting Position

```go
func (j *JoystickInput) Init() {
    // Start at different location
    j.posX = 10.0
    j.posY = 30.0  // Higher up
    j.posZ = 15.0
}
```

## Troubleshooting

### Joystick Not Working

**Symptoms:** Position stuck at (0, 20, 0)

**Solutions:**
- Check GPIO4/GPIO6 wiring
- Verify joystick VCC connected to 3.3V
- Test with isolated joystick example (assignment_4)
- Check serial output for ADC values

### WiFi Won't Connect

**Symptoms:** Connection failed after 3 attempts

**Solutions:**
- Verify SSID and password
- Check network range
- Ensure MQTT broker is running
- Try moving closer to access point

### Avatar Not Appearing

**Symptoms:** Desktop client doesn't show player

**Solutions:**
- Verify MQTT broker is running
- Check device announcement was sent
- Monitor `devices/announce` topic
- Check desktop client console for errors
- Ensure same MQTT broker address

### Movement Laggy

**Symptoms:** Avatar movement delayed or choppy

**Solutions:**
- Check network latency (should be <10ms on local network)
- Verify pose messages being sent at 10 Hz
- Check desktop client FPS (should be 60 FPS)
- Reduce POSE_UPDATE_RATE if bandwidth issues
- Check for WiFi interference

### Position Incorrect

**Symptoms:** Avatar moves in wrong direction

**Solutions:**
- Joystick axes may be swapped
- Try swapping GPIO4 and GPIO6
- Adjust deadzone value
- Calibrate center position

## Advanced Topics

### Multiple Players

Deploy multiple ESP32-C3 controllers:
- Each generates unique player ID
- Desktop client handles multiple avatars
- All players visible in same 3D world
- No code changes needed

### Adding Rotation

Add yaw/pitch control with second joystick or buttons:

```go
// In JoystickInput
yaw   float32
pitch float32

// Update based on rotation input
yaw += rotX * 0.1  // Radians per update
pitch -= rotY * 0.1 // Inverted for natural feel
```

### Adding Buttons

Add action buttons (jump, crouch, interact):

```go
const (
    JUMP_BUTTON    = machine.GPIO7
    CROUCH_BUTTON  = machine.GPIO8
    INTERACT_BUTTON = machine.GPIO10
)

// Read buttons in game loop
if readButton(JUMP_BUTTON) {
    sendActionMessage(client, playerId, "jump")
}
```

### World Boundaries

Add position clamping to prevent going out of bounds:

```go
const (
    WORLD_MIN_X = -50.0
    WORLD_MAX_X = 50.0
    WORLD_MIN_Y = 0.0
    WORLD_MAX_Y = 100.0
    WORLD_MIN_Z = -50.0
    WORLD_MAX_Z = 50.0
)

func (j *JoystickInput) clampPosition() {
    if j.posX < WORLD_MIN_X { j.posX = WORLD_MIN_X }
    if j.posX > WORLD_MAX_X { j.posX = WORLD_MAX_X }
    // ... etc
}
```

## Comparison to Desktop Client

| Feature | ESP32-C3 Controller | Desktop Client (WASD) |
|---------|---------------------|----------------------|
| Input | Joystick (ADC) | Keyboard (WASD) |
| Hardware | $5 ESP32-C3 + $2 joystick | PC with keyboard |
| Movement | 2-axis (X/Y plane) | Full 3D + mouse look |
| Update rate | 10 Hz | 60 Hz (limited by FPS) |
| Complexity | Simple joystick read | Complex physics & collision |
| Portability | Battery powered | Requires PC |
| Cost | ~$10 total | Existing hardware |

## Comparison to Rust Implementation

| Feature | TinyGo | Rust (esp32-c6-client) |
|---------|--------|------------------------|
| Complexity | ~300 lines | ~500+ lines |
| Simplicity | Direct logic | Async/await complexity |
| Build time | Seconds | Minutes |
| Binary size | ~300KB | ~500KB |
| Features | Player control only | Player + temperature + storage |
| Development time | 1-2 hours | 4-6 hours |

TinyGo advantages:
- Simpler, easier to understand
- Faster build and iteration
- Sufficient for game controller
- Lower resource usage

Rust advantages:
- More robust error handling
- Additional features (temperature, storage)
- Better concurrency
- Production-ready

## Requirements

- TinyGo 0.41+
- Go 1.26+
- ESP32-C3 board with WiFi
- Joystick module (dual-axis potentiometer)
- MQTT broker (Mosquitto or compatible)
- IoTCraft desktop client
- USB-C cable

## Related Article

https://developer.espressif.com/workshops/tinygo/assignment-9/

## References

- natiu-mqtt: https://github.com/soypat/natiu-mqtt
- TinyGo espradio: https://github.com/tinygo-org/espradio
- IoTCraft: /Volumes/ssdt5/Users/georgik/projects/iotcraft/
- Desktop client: /Volumes/ssdt5/Users/georgik/projects/iotcraft/desktop-client/
