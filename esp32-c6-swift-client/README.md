# ESP32-C6 Swift IoTCraft Client

A production-ready IoT device client written in **Embedded Swift** for ESP32-C6, providing full integration with the IoTCraft ecosystem. This client demonstrates modern Swift development on embedded systems with ESP-IDF.

## 🎯 Overview

This Swift client implements an IoTCraft-compatible smart lamp device that:
- Connects to WiFi and MQTT automatically
- Announces itself to the IoTCraft system
- Responds to light control commands via MQTT
- Provides visual feedback using an onboard NeoPixel LED
- Handles position updates from the IoTCraft desktop client

## ✨ Features

### 🌐 **IoTCraft Integration**
- **Device Announcements**: Automatic registration on `devices/announce` topic
- **MQTT Topics**: Full compatibility with IoTCraft topic structure
- **JSON Protocols**: Standard IoTCraft message formats with location data
- **Dynamic Device ID**: MAC-based unique identification

### 💡 **Smart Device Functionality**
- **Light Control**: ON/OFF commands via `home/{device_id}/light`
- **Position Updates**: Handle dragging/positioning from desktop client
- **Device Types**: Configured as IoTCraft `lamp` device type
- **Real-time Response**: Immediate MQTT command processing

### 🎨 **Visual Feedback (NeoPixel)**
- **Status Colors**: 
  - 🔴 Red: Offline/Startup
  - 🟡 Yellow: Connecting to WiFi/MQTT
  - 🟢 Green: Online and ready
  - ⚪ White: Light ON state
  - ⚫ Black: Light OFF state (completely dark)
- **Brightness**: Dimmed to comfortable levels (10/255, matching Rust client)
- **Feedback**: Visual confirmation of all network events

### ⚡ **ESP32-C6 Optimizations**
- **Custom Partition Table**: 1.5MB app partition for Swift binary
- **SPI NeoPixel Driver**: Hardware-accelerated LED control
- **Memory Management**: Optimized for embedded Swift constraints
- **WiFi Integration**: Native ESP-IDF WiFi stack

## 🔧 Hardware Requirements

### **Supported Boards**
- **ESP32-C6-DevKitC-1** (recommended)
- **ESP32-C6-DevKitM-1** 
- Any ESP32-C6 board with onboard NeoPixel

### **Hardware Configuration**
- **NeoPixel LED**: GPIO 8 (configurable)
- **WiFi**: 2.4GHz 802.11b/g/n
- **Flash**: Minimum 2MB (uses custom partition table)
- **RAM**: 512KB (Swift optimized)

## 🚀 Quick Start

### **Prerequisites**
- **ESP-IDF 6.0+** with Swift support enabled
- **Swift 6.2+** with Embedded Swift support
- **CMake 3.29+** for Swift integration
- **Python 3.8+** for ESP-IDF tools

### **1. Clone and Setup**
```bash
# Navigate to IoTCraft project
cd /path/to/iotcraft

# The esp32-c6-swift-client directory should be present
cd esp32-c6-swift-client
```

### **2. Configure WiFi**
Edit the WiFi credentials in `main/Main.swift`:
```swift
let ssid = "your-wifi-name"
let password = "your-wifi-password"
```

### **3. Build and Flash**
```bash
# Set ESP-IDF target
idf.py set-target esp32c6

# Build the project
idf.py build

# Flash to device
idf.py -p /dev/cu.usbmodem* flash monitor
```

### **4. Expected Output**
```
=== ESP32-C6 IoTCraft Swift Client v3.0 ===
✅ NVS initialized
✅ LED strip initialized on GPIO8 with 1 LEDs  
✅ WiFi connected successfully
✅ MQTT connected successfully
📡 Device ID: esp32c6-aabbccddeeff
🚀 Ready to receive MQTT commands on IoTCraft topics!
```

## 📡 MQTT Integration

### **Topics Overview**
The Swift client uses standard IoTCraft MQTT topics:

| Topic Pattern | Direction | Purpose | Example |
|---------------|-----------|---------|---------|
| `devices/announce` | Publish | Device registration | Device announces itself |
| `home/{device_id}/light` | Subscribe | Light control | `ON`/`OFF` commands |
| `home/{device_id}/position/set` | Subscribe | Position updates | `{"x":1.5,"y":0.5,"z":2.0}` |

### **Device Announcement Format**
```json
{
  "device_id": "esp32c6-aabbccddeeff",
  "device_type": "lamp", 
  "state": "online",
  "location": {
    "x": 1.0,
    "y": 0.5, 
    "z": 2.0
  }
}
```

### **MQTT Broker Configuration**
- **Host**: `192.168.4.1` (iotcraft gateway)
- **Port**: `1883` 
- **Protocol**: MQTT 3.1.1
- **QoS**: AtLeastOnce for device announcements
- **Clean Session**: Enabled

## 🏗️ Architecture

### **Swift + ESP-IDF Integration**
```
┌─────────────────────────────────────┐
│           Swift Layer               │
├─────────────────────────────────────┤
│  • Main.swift (app logic)           │
│  • LedStrip.swift (NeoPixel)        │  
│  • Callback handlers                │
└─────────────────┬───────────────────┘
                  │ Bridging Header
┌─────────────────▼───────────────────┐
│            C Layer                  │
├─────────────────────────────────────┤
│  • esp_swift_wrapper.c              │
│  • MQTT event handling              │
│  • WiFi management                  │  
└─────────────────┬───────────────────┘
                  │
┌─────────────────▼───────────────────┐
│           ESP-IDF                   │  
├─────────────────────────────────────┤
│  • WiFi driver                      │
│  • MQTT client                      │
│  • LED strip component              │
│  • FreeRTOS                         │
└─────────────────────────────────────┘
```

### **File Structure**
```
esp32-c6-swift-client/
├── main/
│   ├── Main.swift              # Swift application logic
│   ├── LedStrip.swift          # NeoPixel control wrapper  
│   ├── esp_swift_wrapper.c     # C-Swift bridge layer
│   ├── esp_swift_wrapper.h     # C function declarations
│   ├── BridgingHeader.h        # Swift-C interop
│   ├── CMakeLists.txt          # Swift compilation config
│   └── idf_component.yml       # LED strip dependency
├── partitions.csv              # Custom partition table
├── sdkconfig.defaults          # ESP-IDF configuration  
├── CMakeLists.txt              # Project configuration
└── README.md                   # This file
```

## 🧪 Testing with IoTCraft

### **1. Start IoTCraft Infrastructure**
```bash
# Terminal 1: MQTT Broker
cd ../mqtt-server && cargo run

# Terminal 2: Desktop Client  
cd ../desktop-client && cargo run
```

### **2. Flash and Monitor ESP32-C6**
```bash
# Terminal 3: ESP32-C6 Swift Client
cd esp32-c6-swift-client
idf.py flash monitor
```

### **3. Expected Behavior**
- ✅ **LED**: Red → Yellow → Green (startup sequence)
- ✅ **Desktop Client**: Device appears in 3D world automatically
- ✅ **Click Device**: LED turns white (ON) or black (OFF) 
- ✅ **Drag Device**: Position updates received via MQTT
- ✅ **Console Commands**: `list` shows registered Swift device

### **4. MQTT Message Flow**
```
ESP32-C6 Swift → devices/announce → Desktop Client
Desktop Client → home/{device_id}/light → ESP32-C6 Swift  
Desktop Client → home/{device_id}/position/set → ESP32-C6 Swift
```

## 🐛 Troubleshooting

### **Common Issues**

#### **"App partition too small" Error**
```bash
# Solution: Custom partition table is configured automatically
# If still seeing this, clean and rebuild:
idf.py fullclean
idf.py build
```

#### **LED Not Working**
```bash
# Check GPIO configuration
# Default: GPIO 8, verify your board's NeoPixel pin
# Update in Main.swift: LedStrip(gpioPin: YOUR_PIN, maxLeds: 1)
```

#### **MQTT Connection Issues**
```bash  
# Verify WiFi credentials in Main.swift
# Check MQTT broker IP: 192.168.4.1:1883
# Ensure IoTCraft MQTT server is running
```

#### **Monitor "read failed" Errors**
This is normal behavior during WiFi operations. The Swift application continues running properly.

## 🔗 Integration with IoTCraft Ecosystem

This Swift client is fully compatible with:
- **[Desktop Client](../desktop-client/README.md)**: 3D visualization and control
- **[Rust ESP32-C6 Client](../esp32-c6-client/README.md)**: Feature parity
- **[Desktop Device Client](../desktop-device-client/README.md)**: Virtual device simulation
- **[MQTT Server](../mqtt-server/README.md)**: Message broker

## 📊 Performance Metrics

### **Memory Usage**
- **Flash**: ~1.1MB (Swift binary + ESP-IDF)
- **RAM**: ~45KB (Swift runtime + network stack)
- **Boot Time**: ~3-4 seconds (WiFi + MQTT connection)

### **Network Performance**  
- **MQTT Latency**: <50ms (local network)
- **Command Response**: <100ms (MQTT → LED change)
- **Reconnection**: <2 seconds (WiFi loss recovery)

## 📝 Development Notes

### **Swift Embedded Features Used**
- ✅ No Foundation/standard library dependencies
- ✅ C interoperability via bridging headers
- ✅ Manual memory management  
- ✅ Hardware-specific optimizations
- ✅ Embedded-safe concurrency patterns

### **ESP-IDF Integration Patterns**
- ✅ Native MQTT client component
- ✅ WiFi station mode with automatic reconnection
- ✅ SPI-based LED strip driver
- ✅ FreeRTOS task integration
- ✅ NVS (Non-Volatile Storage) initialization

