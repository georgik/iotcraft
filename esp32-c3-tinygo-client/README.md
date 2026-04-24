# WiFi MQTT Lamp

## Overview

This assignment demonstrates WiFi connectivity and MQTT communication on ESP32-C3. The device acts as a controllable lamp that:
- Connects to WiFi network
- Establishes MQTT client connection
- Subscribes to control topics
- Controls NeoPixel LED based on received commands
- Sends device announcements

## Board Support

- **ESP32-C3**: esp32c3-generic target (primary, fully supported)
- **ESP32-S3**: esp32s3-generic target (known ws2812 driver issue)
- **ESP32**: Not supported (espradio requires ESP32-C3/S3)

## Hardware Configuration

### Components

- ESP32-C3 board
- NeoPixel LED (WS2812)
- WiFi connection

### Wiring

| Component | ESP32-C3 GPIO |
|-----------|---------------|
| NeoPixel Data | GPIO8 |
| NeoPixel VCC  | 3.3V |
| NeoPixel GND  | GND |

**Note:** GPIO8 matches the Rust implementation. Include a resistor (330-470Ω) in series with the data pin for reliable communication.

### Network Configuration

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

This ensures the MQTT and espradio packages are available.

## Building and Flashing

### ESP32-C3 (Recommended)

```bash
tinygo flash -target esp32c3-generic main.go
```

### ESP32-S3 (Known Issue)

ESP32-S3 has a known issue with the ws2812 driver (machine.CPUFrequency undefined).
Use ESP32-C3 for reliable operation.

## Troubleshooting

### ESP32-S3 Build Fails

Error: `undefined: machine.CPUFrequency`

This is a known issue with the ws2812 driver on ESP32-S3.
Solution: Use ESP32-C3 target instead.

## How It Works

### Architecture

```
ESP32-C3
  ├─ WiFi Connection (espradio)
  │   └─ Connect to tinygo network
  ├─ MQTT Client (natiu-mqtt)
  │   ├─ TCP connection to broker
  │   ├─ Subscribe to control topic
  │   └─ Publish device announcements
  └─ LED Control (ws2812)
      ├─ ON command → Yellow LED
      └─ OFF command → LED off
```

### MQTT Topics

**Control Topic:**
- Pattern: `home/{device_id}/light`
- Example: `home/esp32c3-lamp-A1B2C3/light`
- Commands: `ON`, `OFF`

**Announcement Topic:**
- Topic: `devices/announce`
- Payload: JSON with device info
- Sent on startup

### Device ID

Each device generates a unique ID on startup:
- Format: `esp32c3-lamp-XXXXXX`
- Suffix: 6 random uppercase letters
- Example: `esp32c3-lamp-A1B2C3`

## Testing

### 1. Setup MQTT Broker

If you don't have an MQTT broker running:

```bash
# Install Mosquitto
brew install mosquitto

# Start broker
mosquitto -p 1883

# Or use Docker
docker run -it -p 1883:1883 eclipse-mosquitto
```

### 2. Monitor Device

```bash
tinygo monitor -target esp32c3-generic
```

Expected output:
```
WiFi MQTT Lamp - ESP32-C3
========================

Connecting to WiFi...
WiFi connected!
IP Address: 192.168.1.50
Client ID: esp32c3-lamp-A1B2C3
TCP connected to 192.168.1.100:1883
MQTT connected
Subscribed to topic: home/esp32c3-lamp-A1B2C3/light
Device announcement sent

Listening for commands...
Send: mosquitto_pub -h 192.168.1.100 -t home/esp32c3-lamp-A1B2C3/light -m "ON"
```

### 3. Send Commands

**Turn LED ON:**
```bash
mosquitto_pub -h 192.168.1.100 -t "home/esp32c3-lamp-A1B2C3/light" -m "ON"
```

**Turn LED OFF:**
```bash
mosquitto_pub -h 192.168.1.100 -t "home/esp32c3-lamp-A1B2C3/light" -m "OFF"
```

### 4. Monitor MQTT Traffic

**Monitor all lamp commands:**
```bash
mosquitto_sub -h 192.168.1.100 -t "home/+/light"
```

**Monitor device announcements:**
```bash
mosquitto_sub -h 192.168.1.100 -t "devices/announce"
```

## Customization

### Change WiFi Credentials

Edit the variables at the top of `main.go`:

```go
var (
    ssid     string = "your-ssid"
    password string = "your-password"
    broker   string = "192.168.1.100:1883"
)
```

### Change MQTT Broker

```go
var broker string = "your-broker-ip:1883"
```

### Change LED Pin

```go
const (
    LED_PIN = machine.GPIO8 // Change to your pin
)
```

### Change LED Colors

```go
func setLED(on bool) {
    var c color.RGBA
    if on {
        c = color.RGBA{R: 255, G: 0, B: 0, A: 255} // Red
    } else {
        c = color.RGBA{R: 0, G: 0, B: 0, A: 0}    // Off
    }
    // ... rest of function
}
```

### Adjust Brightness

```go
const (
    LED_BRIGHTNESS = 128 // Change 0-255 (50%)
)
```

## Technical Details

### Libraries Used

- **natiu-mqtt**: Lightweight MQTT v3.1.1 client
  - Zero-allocation decoder
  - Transport-agnostic design
  - No goroutines required

- **espradio**: ESP32 WiFi driver
  - Handles WiFi connection
  - Network device registration
  - Compatible with ESP32-C3/S3

- **ws2812**: NeoPixel driver
  - Controls RGB LED
  - Single-wire communication
  - Configurable brightness

### Memory Usage

- MQTT buffer: 1500 bytes (MTU)
- Stack: Minimal (~2KB recommended)
- Heap: ~20KB for network buffers

### Network Protocol Flow

1. **WiFi Connection:**
   - espradio initializes radio
   - Connects to SSID with credentials
   - Obtains IP via DHCP

2. **MQTT Connection:**
   - TCP dial to broker:1883
   - MQTT CONNECT with client ID
   - Subscribe to control topic
   - Send device announcement

3. **Message Loop:**
   - Set read deadline (10s)
   - Call HandleNext() for incoming messages
   - OnPub callback processes commands
   - LED state updated via callback

### Error Handling

- WiFi connection: 3 retries with 5s delay
- TCP connection: 5 attempts with 2s delay
- MQTT errors: Logged, loop continues
- Read timeout: Logged, retry HandleNext()

## Troubleshooting

### WiFi Won't Connect

- Check SSID and password
- Verify network is in range
- Check serial output for error messages
- Try moving closer to access point

### MQTT Connection Fails

- Verify broker is running: `mosquitto -p 1883`
- Check broker IP address
- Ensure broker port is 1883
- Check firewall settings

### LED Won't Turn On

- Verify GPIO8 wiring
- Check NeoPixel VCC and GND connections
- Add resistor (330-470Ω) in series with data pin
- Test LED with example from assignment_2

### Commands Not Received

- Verify topic matches device ID
- Check broker is running
- Monitor topic: `mosquitto_sub -h 192.168.1.100 -t "home/+/light"`
- Check serial output for connection status

## Advanced Topics

### QoS Levels

Change from QoS0 to QoS1:

```go
TopicFilters: []mqtt.SubscribeRequest{
    {TopicFilter: []byte(topic), QoS: mqtt.QoS1},
}
```

### Multiple Devices

Deploy multiple lamps on same network:
- Each generates unique device ID
- Subscribe to wildcard: `home/+/light`
- Control individually: `home/esp32c3-lamp-XXXXXX/light`
- Control all: Use MQTT retained messages or LWT

### Persistence

To add state persistence across reboots:
- Use NVS (Non-Volatile Storage) on ESP32
- Store last LED state in flash
- Restore state on startup
- (Requires ESP-IDF NVS API bindings)

## Comparison to Rust Implementation

| Feature | TinyGo | Rust (esp32-c6-client) |
|---------|--------|------------------------|
| Lines of code | ~200 | 689 |
| MQTT library | natiu-mqtt | rust-mqtt |
| Concurrency | Single-threaded | Async/await |
| Storage | None | Flash persistence |
| Complexity | Simple | Advanced |
| Development time | 1-2 hours | 4-6 hours |

TinyGo advantages:
- Simpler, easier to understand
- Faster development cycle
- Smaller binary size
- Lower memory footprint

Rust advantages:
- Persistent state storage
- More robust error handling
- Advanced features (position tracking)
- Type-safe MQTT packet handling

## Requirements

- TinyGo 0.41+
- Go 1.26+
- ESP32-C3 board with WiFi
- NeoPixel LED (WS2812)
- MQTT broker (Mosquitto or compatible)
- USB-C cable

## Related Article

https://developer.espressif.com/workshops/tinygo/assignment-8/

## References

- natiu-mqtt: https://github.com/soypat/natiu-mqtt
- TinyGo espradio: https://github.com/tinygo-org/espradio
- MQTT protocol: https://mqtt.org/mqtt-specification/
- Mosquitto: https://mosquitto.org/
