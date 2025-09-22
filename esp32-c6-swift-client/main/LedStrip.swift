//===----------------------------------------------------------------------===//
// LedStrip.swift for ESP32-C6 Swift IoTCraft Client
// NeoPixel (WS2812) LED control for IoTCraft visual feedback
//===----------------------------------------------------------------------===//

/// Swift wrapper for ESP-IDF LED strip driver
/// Provides easy NeoPixel control for IoTCraft device status indication
struct LedStrip {
    private let handle: led_strip_handle_t

    /// Initialize LED strip on specified GPIO pin
    /// - Parameters:
    ///   - gpioPin: GPIO pin number for NeoPixel data line
    ///   - maxLeds: Maximum number of LEDs in the strip
    init(gpioPin: Int, maxLeds: Int) {
        var handle = led_strip_handle_t(bitPattern: 0)
        var stripConfig = led_strip_config_t(
            strip_gpio_num: Int32(gpioPin),
            max_leds: UInt32(maxLeds),
            led_pixel_format: LED_PIXEL_FORMAT_GRB,
            led_model: LED_MODEL_WS2812,
            flags: .init(invert_out: 0)
        )
        var spiConfig = led_strip_spi_config_t(
            clk_src: SPI_CLK_SRC_DEFAULT,
            spi_bus: SPI2_HOST,
            flags: .init(with_dma: 1)
        )
        guard led_strip_new_spi_device(&stripConfig, &spiConfig, &handle) == ESP_OK,
            let handle = handle
        else {
            print("❌ Failed to configure LED strip on GPIO\(gpioPin)")
            fatalError("cannot configure LED strip SPI device")
        }
        self.handle = handle
        print("✅ LED strip initialized on GPIO\(gpioPin) with \(maxLeds) LEDs")
    }

    /// Color structure for RGB LED control
    struct Color {
        static var off = Color(r: 0, g: 0, b: 0)
        static var white = Color(r: 10, g: 10, b: 10)  // Dimmed white
        static var red = Color(r: 10, g: 0, b: 0)  // Dimmed red
        static var green = Color(r: 0, g: 10, b: 0)  // Dimmed green
        static var blue = Color(r: 0, g: 0, b: 10)  // Dimmed blue
        static var yellow = Color(r: 10, g: 10, b: 0)  // Dimmed yellow
        static var cyan = Color(r: 0, g: 10, b: 10)  // Dimmed cyan
        static var magenta = Color(r: 10, g: 0, b: 10)  // Dimmed magenta

        // IoTCraft specific colors - reduced brightness to match Rust client
        static var online = Color(r: 0, g: 10, b: 0)  // Green for online (dimmed)
        static var offline = Color(r: 10, g: 0, b: 0)  // Red for offline (dimmed)
        static var connecting = Color(r: 10, g: 10, b: 0)  // Yellow for connecting (dimmed)
        static var lightOn = Color(r: 10, g: 10, b: 10)  // White for light on (dimmed)
        static var dimmed = Color(r: 2, g: 2, b: 2)  // Very dim white for standby

        var r, g, b: UInt8
    }

    /// Set color of a specific LED pixel
    /// - Parameters:
    ///   - index: LED index (0-based)
    ///   - color: Color to set
    func setPixel(index: Int, color: Color) {
        led_strip_set_pixel(
            handle, UInt32(index), UInt32(color.r), UInt32(color.g), UInt32(color.b))
    }

    /// Set all LEDs to the same color
    /// - Parameter color: Color to set for all LEDs
    func setAll(color: Color) {
        // Assuming single LED for most IoTCraft devices
        setPixel(index: 0, color: color)
    }

    /// Refresh the LED strip to show new colors
    func refresh() {
        led_strip_refresh(handle)
    }

    /// Clear all LEDs (turn off)
    func clear() {
        led_strip_clear(handle)
        refresh()
    }

    /// Convenience function to show device status
    /// - Parameter isOnline: Whether device is online
    func showStatus(isOnline: Bool) {
        if isOnline {
            setAll(color: .online)
        } else {
            setAll(color: .offline)
        }
        refresh()
    }

    /// Show light state for IoTCraft lamp device
    /// - Parameter isOn: Whether the light should be on
    func showLight(isOn: Bool) {
        if isOn {
            setAll(color: .lightOn)
        } else {
            setAll(color: .off)  // Completely off, no glow
        }
        refresh()
    }

    /// Flash LED briefly for feedback
    /// - Parameter color: Color to flash (default is dimmed white)
    func flash(color: Color = Color(r: 10, g: 10, b: 10)) {
        setAll(color: color)
        refresh()
        vTaskDelay(200 / (1000 / UInt32(configTICK_RATE_HZ)))  // 200ms
        setAll(color: .dimmed)
        refresh()
    }
}
