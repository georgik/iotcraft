# IoTCraft Gateway (ESP32-S3)

**A complete WiFi network infrastructure solution for the IoTCraft ecosystem**

## 🎯 Overview

The IoTCraft Gateway transforms an ESP32-S3-BOX-3 into a complete network infrastructure device that provides WiFi connectivity, MQTT messaging, and device management for IoTCraft clients. It creates a standalone network where IoTCraft clients can automatically discover and connect to services.

## ✨ Features

### Core Network Services
- **WiFi Access Point**: Creates "iotcraft" network with WPA2 security
- **DHCP/NAT Router**: Automatic IP assignment (192.168.4.2-254) with internet sharing
- **MQTT Broker**: Real-time messaging on port 1883 for world synchronization
- **mDNS Service**: Zero-configuration networking with `.local` hostnames
- **HTTP Configuration Server**: Web-based management interface

### User Interface
- **Status GUI**: Real-time display on ESP32-S3-BOX-3 screen showing:
  - Network credentials (SSID/Password)
  - Service status indicators  
  - System health (CPU, memory, uptime)
  - Connected client count
- **Web Interface**: Full WiFi configuration and network management
- **USB Console**: Development and debugging access (stays active with WiFi)

### Configuration Management
- **Dynamic WiFi Configuration**: Change SSID/password through web interface
- **Parent Network Support**: Connect gateway to existing WiFi for internet access
- **Persistent Storage**: Configuration saved to LittleFS filesystem
- **DHCP Reservations**: Assign specific IPs to known devices

## 🔧 Target Hardware

- **Primary**: ESP32-S3-BOX-3 (320x240 IPS display, capacitive touch)
- **Requirements**: 16MB Flash, 8MB PSRAM, ESP32-S3 dual-core
- **Alternative**: Other ESP32-S3 boards with SDL3-compatible display

## Build

```bash
# Configure for ESP32-S3-BOX-3
idf.py set-target esp32s3
idf.py menuconfig  # Select ESP32-S3-BOX-3 board profile if available

# Build and flash
idf.py build flash monitor
```

## Network Architecture

```
[Internet] ↔ [ESP32-S3 Gateway] ↔ [IoTCraft Devices]
             192.168.4.1           192.168.4.2-254
             ├─ DHCP Server
             ├─ NAT Router  
             ├─ MQTT Broker (port 1883)
             ├─ HTTP Server (port 80)
             └─ mDNS (*.local hostnames)
```

## Service Discovery

IoTCraft devices can automatically discover services:
- `iotcraft-broker.local:1883` - MQTT broker
- `iotcraft-gateway.local` - HTTP configuration interface
- Gateway IP: `192.168.4.1` (fallback)

## Development Status

- ✅ **DHCP/NAT Router**: Working (from esp32-dhcp-server base)
- 🔄 **MQTT Broker**: In progress (ESP-IDF Mosquitto port)
- 🔄 **mDNS Service**: In progress (espressif/mdns component)
- 🔄 **HTTP Server**: In progress
- 🔄 **Status GUI**: In progress (SDL3 + ESP-IDF)
- ❌ **Bridge Support**: Future feature

