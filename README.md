
# IoTCraft

IoTCraft is a multi-component Rust project showcasing MQTT-controlled IoT devices and a desktop 3D visualizer.

## Components

- **desktop-client**  
  A Rust **std** application built with Bevy. It renders a 3D scene containing:
  - A command-driven lamp cube (ON/OFF via MQTT).
  - A textured grass ground and sky.
  - A rotating logo cube.
  - A thermometer indicator scaling with temperature readings.
  - WASD + mouse camera controls.

- **mqtt-server**  
  A Rust **std** executable using `rumqttd` as an embedded MQTT broker.  
  Reads `rumqttd.toml` for configuration and handles MQTT v4/v5 and WebSocket connections.

- **mqtt-client**  
  A Rust **std** command-line tool using `rumqttc` for synchronous publish/subscribe.  
  Examples:
  ```bash
  # Subscribe to lamp topic
  mosquitto_sub -h localhost -p 1883 -t home/cube/light -i mqtt-client
  
  # Publish ON/OFF
  mosquitto_pub -h localhost -p 1883 -t home/cube/light -m "ON" -i mqtt-client
  ```

- **esp32-c6-client**  
  An embedded **no_std** application for the ESP32-C6 using Embassy and ESP HAL.  
  - Connects to Wi-Fi, drives MQTT subscription for lamp control (topic `home/cube/light`),  
    toggles an LED.
  - Reads the on-board temperature sensor via I2C and publishes readings (`home/sensor/temperature`).

- **esp32-c3-devkit-rust-1**  
  An embedded **no_std** application for the ESP32-C3-DevKit-RS board using Embassy.  
  Similar to the C6 client but adapted to the C3’s GPIO layout and peripherals.

## Getting Started

### Desktop Client

```bash
# Ensure MQTT broker is running:
cd mqtt-server
cargo run

# Run the Bevy 3D visualizer:
cd desktop-client
cargo run
```

### MQTT Server

```bash
cd mqtt-server
cargo run
```

### MQTT Client

```bash
cd mqtt-client
cargo run
```

### ESP32 Clients

Set Wi-Fi SSID/PASSWORD and broker address in the embedded source, then flash:

```bash
export SSID="your_wifi_ssid"
export PASSWORD="your_wifi_password"
export SERVER_IP="your_mqtt_broker_ip"
# ESP32-C6
cd esp32-c6-client
cargo run --release

# ESP32-C3-DevKit
cd esp32-c3-devkit-rust-1
cargo run --release
```

#### Simulation

Use Wokwi simulator ([wokwi-cli](https://docs.wokwi.com/wokwi-ci/cli-usage), 
[JetBrains plugin](https://plugins.jetbrains.com/plugin/23826-wokwi-simulator) or 
[VS Code plugin](https://docs.wokwi.com/vscode/getting-started)) to simulate ESP32-C3 device.

```bash
export SSID="Wokwi-GUEST"
export PASSWORD=""
export SERVER_IP="your_mqtt_broker_ip"

cd esp32-c3-devkit-rust-1
cargo build --release
wokwi-cli
```

## Development

**Recommended IDE:** RustRover

### MQTT Client using Mosquitto

Client:

```shell
brew install mosquitto
```

Watch:
```shell
mosquitto_sub -h localhost -p 1883 -t home/cube/light -i iotcraft-client
```

Change:
```shell
mosquitto_pub -h localhost -p 1883 -t home/cube/light -m "ON" -i iotcraft-client
mosquitto_pub -h localhost -p 1883 -t home/cube/light -m "OFF" -i iotcraft-client
```
