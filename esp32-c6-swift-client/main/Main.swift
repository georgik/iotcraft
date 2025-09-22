//===----------------------------------------------------------------------===//
// Main.swift for ESP32-C6 Swift IoTCraft Client
// Production-ready IoT client using ESP-IDF WiFi and MQTT components
//===----------------------------------------------------------------------===//

// Global LED strip instance
var globalLedStrip: LedStrip? = nil

// LED control callback function called from C
@_cdecl("swift_led_control")
func ledControlCallback(isOn: Bool) {
    guard let ledStrip = globalLedStrip else {
        print("⚠️ LED strip not initialized")
        return
    }

    print("🎨 Setting LED: \(isOn ? "ON" : "OFF")")
    ledStrip.showLight(isOn: isOn)
}

@_cdecl("app_main")
func main() {
    print("=== ESP32-C6 IoTCraft Swift Client v3.0 ===")
    print("Production-ready with ESP-IDF integration")

    // Initialize NVS (Non-Volatile Storage)
    print("Initializing NVS...")
    if nvs_init() != 0 {
        print("❌ NVS initialization failed")
        return
    }
    print("✅ NVS initialized")

    // Initialize NeoPixel LED strip
    print("Initializing NeoPixel LED strip...")
    globalLedStrip = LedStrip(gpioPin: 8, maxLeds: 1)
    globalLedStrip?.showStatus(isOnline: false)  // Start with offline status

    // Register LED control callback with C layer
    let ledCallback: @convention(c) (Bool) -> Void = { isOn in
        ledControlCallback(isOn: isOn)
    }
    register_led_control_callback(ledCallback)
    print("✅ LED control callback registered")

    // Initialize WiFi subsystem
    print("Initializing WiFi subsystem...")
    if wifi_init() != 0 {
        print("❌ WiFi initialization failed")
        return
    }
    print("✅ WiFi initialized")

    // Connect to WiFi network
    print("Connecting to WiFi network...")
    let ssid = "iotcraft"
    let password = "iotcraft123"

    let wifiResult = ssid.withCString { ssidPtr in
        password.withCString { passwordPtr in
            wifi_connect(ssidPtr, passwordPtr)
        }
    }

    if wifiResult == 0 {
        print("✅ WiFi connected successfully")
        print("SSID: iotcraft")
        globalLedStrip?.showStatus(isOnline: false)  // Still connecting to MQTT
    } else {
        print("❌ WiFi connection failed")
        globalLedStrip?.showStatus(isOnline: false)
        return
    }

    // Initialize and connect to MQTT broker
    print("Initializing MQTT client...")
    if mqtt_client_init() != 0 {
        print("❌ MQTT client initialization failed")
        return
    }
    print("✅ MQTT client initialized")

    print("Connecting to MQTT broker...")
    print("Broker: 192.168.4.1:1883 (iotcraft gateway)")

    if mqtt_client_start() != 0 {
        print("❌ MQTT connection failed")
        return
    }

    print("✅ MQTT connected successfully")
    globalLedStrip?.flash(color: .green)  // Flash green for MQTT success

    // Generate device ID from MAC address
    let deviceIDBuffer = UnsafeMutablePointer<CChar>.allocate(capacity: 32)
    defer { deviceIDBuffer.deallocate() }

    generate_device_id(deviceIDBuffer, 32)
    print("Device ID: \(String(cString: deviceIDBuffer))")

    // Give MQTT client time to establish connection
    print("Waiting for MQTT handshake...")
    globalLedStrip?.setAll(color: .connecting)  // Yellow for connecting
    globalLedStrip?.refresh()
    vTaskDelay(2000 / (1000 / UInt32(configTICK_RATE_HZ)))

    // Start main application loop
    print("=== Starting IoTCraft Main Loop ===")
    print("🚀 Ready to receive MQTT commands on IoTCraft topics!")
    print("📡 Device ID: \(String(cString: deviceIDBuffer))")
    print("📋 Subscribed to topics:")
    print("   - home/\(String(cString: deviceIDBuffer))/light")
    print("   - home/\(String(cString: deviceIDBuffer))/position/set")
    print("💡 MQTT event handling managed by ESP-IDF C layer")

    // Show device is fully online
    globalLedStrip?.showStatus(isOnline: true)

    var loopCounter: UInt32 = 0
    var lastAnnouncementTime: UInt32 = 0

    while true {
        let currentTime = loopCounter

        // Send periodic device announcement every ~60 seconds (600 * 100ms = 60s)
        if currentTime - lastAnnouncementTime >= 600 {
            let announceResult = mqtt_publish_device_announcement()

            if announceResult == 0 {
                print("📢 Device announcement sent (IoTCraft format)")
            } else {
                print("❌ Failed to send device announcement")
            }

            lastAnnouncementTime = currentTime
        }

        // Periodic status log every ~10 seconds (100 * 100ms = 10s)
        if loopCounter % 100 == 0 {
            print("⚡ Loop: \(loopCounter) - ESP32-C6 IoTCraft device operational")
        }

        // Brief delay - 100ms
        vTaskDelay(100 / (1000 / UInt32(configTICK_RATE_HZ)))
        loopCounter = loopCounter &+ 1

        // Reset counter to prevent overflow
        if loopCounter >= 30000 {
            loopCounter = 0
            lastAnnouncementTime = 0
        }
    }
}
