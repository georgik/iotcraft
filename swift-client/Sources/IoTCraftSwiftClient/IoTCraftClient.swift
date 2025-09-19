import Foundation
import Logging
import MQTTNIO

/// Main IoTCraft client showcasing Swift 6.3-dev features
@MainActor
final class IoTCraftClient: ObservableObject {
    private let mqttHost: String
    private let mqttPort: Int
    private let displayMode: DisplayMode
    private let logger: Logger
    
    // World state with Swift 6.3-dev concurrency safety
    @Published private(set) var worldState = WorldState()
    @Published private(set) var devices: [String: IoTDevice] = [:]
    @Published private(set) var connectionStatus: ConnectionStatus = .disconnected
    
    // MQTT client for real IoT integration
    private var mqttClient: IoTCraftMQTTClient?
    private var simulationTask: Task<Void, Error>?
    
    init(mqttHost: String, mqttPort: Int, displayMode: DisplayMode, logger: Logger) {
        self.mqttHost = mqttHost
        self.mqttPort = mqttPort
        self.displayMode = displayMode
        self.logger = logger
    }
    
    deinit {
        simulationTask?.cancel()
    }
    
    /// Start the IoT client with Swift 6.3-dev structured concurrency
    func start() async throws {
        logger.info("🚀 IoTCraft Swift Client starting...")
        logger.info("📡 Target: \(mqttHost):\(mqttPort)")
        logger.info("🖥️  Mode: \(displayMode)")
        
        // Initialize MQTT client
        mqttClient = IoTCraftMQTTClient(host: mqttHost, port: mqttPort)
        
        // Set up MQTT event handlers
        setupMQTTHandlers()
        
        // Attempt MQTT connection
        connectionStatus = .connecting
        
        do {
            try await mqttClient?.connect()
            connectionStatus = .connected
            logger.info("✅ Connected to real MQTT broker")
        } catch {
            logger.warning("⚠️  MQTT connection failed: \(error)")
            logger.info("🔄 Falling back to simulation mode")
            connectionStatus = .simulationMode
        }
        
        // Start simulation and display tasks concurrently
        simulationTask = Task {
            try await withThrowingTaskGroup(of: Void.self) { group in
                // Only run simulation if not connected to real MQTT
                if self.connectionStatus == .simulationMode {
                    // Device simulation task
                    group.addTask {
                        try await self.simulateDevices()
                    }
                    
                    // World simulation task  
                    group.addTask {
                        try await self.simulateWorld()
                    }
                }
                
                // Display task
                group.addTask {
                    try await self.runDisplay()
                }
                
                try await group.waitForAll()
            }
        }
        
        try await simulationTask?.value
    }
    
    /// Set up MQTT event handlers
    private func setupMQTTHandlers() {
        mqttClient?.onDeviceAnnouncement = { [weak self] device in
            Task { @MainActor in
                await self?.handleDeviceAnnouncement(device)
            }
        }
        
        mqttClient?.onDeviceStateChange = { [weak self] deviceId, state in
            Task { @MainActor in
                await self?.handleDeviceStateChange(deviceId, state)
            }
        }
        
        mqttClient?.onWorldUpdate = { [weak self] worldId, data in
            Task { @MainActor in
                await self?.handleWorldUpdate(worldId, data)
            }
        }
    }
    
    /// Handle real device announcements from MQTT
    private func handleDeviceAnnouncement(_ device: IoTDevice) async {
        devices[device.id] = device
        logger.info("📡 Real device announced: \(device.id) (\(device.deviceType.emoji))")
    }
    
    /// Handle device state changes from MQTT
    private func handleDeviceStateChange(_ deviceId: String, _ state: [String: Any]) async {
        guard var device = devices[deviceId] else {
            logger.warning("⚠️  State change for unknown device: \(deviceId)")
            return
        }
        
        // Update device state based on received data
        if let lightState = state["light"] as? String {
            device.lightState = LightState(rawValue: lightState) ?? .off
        }
        
        if let doorState = state["door"] as? String {
            device.doorState = DoorState(rawValue: doorState) ?? .closed
        }
        
        if let position = state["position"] as? Position {
            device.position = position
        }
        
        devices[deviceId] = device
        logger.debug("🔄 Device \(deviceId) state updated")
    }
    
    /// Handle world updates from MQTT
    private func handleWorldUpdate(_ worldId: String, _ data: Any) async {
        logger.info("🌍 World \(worldId) update received")
        // TODO: Parse and apply world updates
    }
    
    /// Send device control command via MQTT
    func controlDevice(_ deviceId: String, command: String, value: String) async {
        do {
            try await mqttClient?.publishDeviceCommand(deviceId, command: command, value: value)
            logger.info("📤 Sent command to \(deviceId): \(command) = \(value)")
        } catch {
            logger.error("❌ Failed to send command to \(deviceId): \(error)")
        }
    }
    
    /// Simulate IoT devices with modern Swift patterns
    private func simulateDevices() async throws {
        logger.info("📱 Starting device simulation...")
        
        // Create some mock devices using Swift 6.3-dev features
        let mockDevices = [
            IoTDevice.lamp(id: "esp32_lamp_001"),
            IoTDevice.door(id: "esp32_door_001", at: Position(x: 5, y: 1, z: 3)),
            IoTDevice(id: "esp32_sensor_001", type: .sensor, state: .online, position: Position(x: -2, y: 0, z: 4))
        ]
        
        // Add devices to state
        for device in mockDevices {
            devices[device.id] = device
            logger.info("📱 Added device: \(device.id) (\(device.type.emoji))")
        }
        
        // Simulate device state changes
        while !Task.isCancelled {
            try await Task.sleep(for: .seconds(3))
            await simulateDeviceStateChanges()
        }
    }
    
    /// Simulate world blocks
    private func simulateWorld() async throws {
        logger.info("🌍 Starting world simulation...")
        
        // Create a simple world layout
        let blocks = [
            BlockData(x: 0, y: 0, z: 0, block_type: "grass"),
            BlockData(x: 1, y: 0, z: 0, block_type: "grass"),
            BlockData(x: 2, y: 0, z: 0, block_type: "stone"),
            BlockData(x: 0, y: 0, z: 1, block_type: "dirt"),
            BlockData(x: 1, y: 0, z: 1, block_type: "dirt"),
            BlockData(x: 2, y: 0, z: 1, block_type: "stone"),
        ]
        
        worldState.updateBlocks(blocks)
        logger.info("🌍 Added \(blocks.count) blocks to world")
        
        // Simulate world changes
        while !Task.isCancelled {
            try await Task.sleep(for: .seconds(10))
            await simulateWorldChanges()
        }
    }
    
    /// Simulate device state changes using Swift 6.3-dev patterns
    private func simulateDeviceStateChanges() async {
        guard !devices.isEmpty else { return }
        
        // Pick a random device and change its state
        let deviceIds = Array(devices.keys)
        let randomId = deviceIds.randomElement()!
        
        if var device = devices[randomId] {
            // Toggle light state for lamps
            if device.type == .lamp {
                device.lightState = device.lightState == .on ? .off : .on
                devices[randomId] = device
                logger.debug("💡 \(randomId) light: \(device.lightState)")
            }
            
            // Simulate position changes
            if Double.random(in: 0...1) > 0.8 {
                let newPosition = Position(
                    x: device.position.x + Float.random(in: -1...1),
                    y: device.position.y,
                    z: device.position.z + Float.random(in: -1...1)
                )
                device.position = newPosition
                devices[randomId] = device
                logger.debug("📍 \(randomId) moved to: \(newPosition)")
            }
        }
    }
    
    /// Simulate world changes
    private func simulateWorldChanges() async {
        // Add a random block
        let x = Int.random(in: -5...5)
        let z = Int.random(in: -5...5)
        let blockTypes = BlockType.allCases
        let randomType = blockTypes.randomElement()!
        
        let position = Position(x: x, y: 0, z: z)
        worldState.addBlock(at: position, type: randomType)
        
        logger.debug("🧱 Added \(randomType) block at \(position)")
    }
    
    /// Run display mode
    private func runDisplay() async throws {
        switch displayMode {
        case .gui:
            try await runGUIDisplay()
        case .terminal:
            try await runTerminalDisplay()
        }
    }
    
    /// GUI display mode (placeholder for SwiftUI)
    private func runGUIDisplay() async throws {
        logger.info("🖼️  Starting GUI mode (placeholder)")
        
        while !Task.isCancelled {
            try await Task.sleep(for: .seconds(2))
            logger.debug("🖥️  GUI refresh: \(devices.count) devices, \(worldState.blockCount) blocks")
        }
    }
    
    /// Terminal display mode with bird's eye view
    private func runTerminalDisplay() async throws {
        logger.info("📟 Starting terminal bird's eye view...")
        
        while !Task.isCancelled {
            try await Task.sleep(for: .seconds(5))
            await renderBirdsEyeView()
        }
    }
    
    /// Render bird's eye view in terminal
    private func renderBirdsEyeView() async {
        print("\n" + "=".repeating(count: 60))
        print("🦅 IoTCraft Swift Client - Bird's Eye View")
        print("=".repeating(count: 60))
        print("🔗 Status: \(connectionStatus.rawValue)")
        print("📊 Stats: \(devices.count) devices, \(worldState.blockCount) blocks")
        print("")
        
        // Render a simple ASCII map
        print("📍 Device Positions:")
        for (_, device) in devices {
            let status = device.state == .online ? "🟢" : "🔴"
            let light = device.lightState == .on ? "💡" : "🔋"
            let pos = device.position
            print(String(format: "  %@ %@ %@ at (%.1f, %.1f)", 
                        status, device.type.emoji, light, pos.x, pos.z))
        }
        
        print("")
        print("🗺️  World Map (Top-Down View):")
        renderSimpleMap()
        
        print("=".repeating(count: 60))
        print("Swift 6.3-dev Features in use:")
        print("  ✓ @MainActor isolation")
        print("  ✓ Structured concurrency with TaskGroup")
        print("  ✓ Async sequences and modern patterns")
        print("  ✓ Sendable types with strict concurrency")
        print("=".repeating(count: 60))
    }
    
    /// Render a simple ASCII map
    private func renderSimpleMap() {
        let mapSize = 10
        let centerX = mapSize / 2
        let centerZ = mapSize / 2
        
        // Create empty map
        var map = Array(repeating: Array(repeating: "⬜", count: mapSize), count: mapSize)
        
        // Add blocks
        for (position, blockType) in worldState.blocks {
            let mapX = centerX + Int(position.x)
            let mapZ = centerZ + Int(position.z)
            
            if mapX >= 0 && mapX < mapSize && mapZ >= 0 && mapZ < mapSize {
                map[mapZ][mapX] = blockType.emoji
            }
        }
        
        // Add devices
        for (_, device) in devices {
            let mapX = centerX + Int(device.position.x)
            let mapZ = centerZ + Int(device.position.z)
            
            if mapX >= 0 && mapX < mapSize && mapZ >= 0 && mapZ < mapSize {
                map[mapZ][mapX] = device.type.emoji
            }
        }
        
        // Print map
        print("   " + (0..<mapSize).map { String($0) }.joined())
        for (z, row) in map.enumerated() {
            print(String(format: "%2d ", z) + row.joined())
        }
    }
}

// MARK: - Supporting Types

enum ConnectionStatus: String {
    case disconnected = "Disconnected"
    case connecting = "Connecting"
    case connected = "Connected (Real MQTT)"
    case simulationMode = "Simulation Mode"
}

enum IoTCraftError: Error, LocalizedError {
    case simulationFailed
    
    var errorDescription: String? {
        switch self {
        case .simulationFailed:
            return "Device simulation failed"
        }
    }
}

// String extension
private extension String {
    func repeating(count: Int) -> String {
        return String(repeating: self, count: count)
    }
}
