import Foundation
import MQTTNIO
import AnyCodable
import Logging
import NIOCore
import NIOPosix

/// MQTT client for IoTCraft integration
/// Connects to MQTT broker and manages device communication
@MainActor
final class IoTCraftMQTTClient: ObservableObject {
    private var mqttClient: MQTTClient?
    private let logger = Logger(label: "mqtt-client")
    private let eventLoopGroup = MultiThreadedEventLoopGroup(numberOfThreads: 1)
    
    // MQTT configuration
    private let brokerHost: String
    private let brokerPort: Int
    private let clientId: String
    
    // Connection state
    @Published private(set) var isConnected = false
    @Published private(set) var connectionStatus = "Disconnected"
    
    // Device and world state callbacks (async)
    var onDeviceAnnouncement: ((IoTDevice) async -> Void)?
    var onDeviceStateChange: ((String, [String: Any]) async -> Void)?
    var onWorldUpdate: ((String, Any) async -> Void)?
    
    init(host: String = "localhost", port: Int = 1883, clientId: String? = nil) {
        self.brokerHost = host
        self.brokerPort = port
        self.clientId = clientId ?? "swift-client-\(UUID().uuidString.prefix(8))"
        
        logger.info("Initialized MQTT client for \(host):\(port) with ID: \(self.clientId)")
    }
    
    deinit {
        // Note: Cannot properly clean up async resources in deinit
        // Resources will be cleaned up when the client is deallocated
    }
    
    /// Connect to MQTT broker
    func connect() async throws {
        guard mqttClient == nil else {
            logger.warning("Already connected or connecting to MQTT broker")
            return
        }
        
        connectionStatus = "Connecting..."
        logger.info("Connecting to MQTT broker at \(brokerHost):\(brokerPort)")
        
        do {
            // Create MQTT configuration
            let configuration = MQTTConfiguration(
                target: .host(brokerHost, port: brokerPort),
                protocolVersion: .version3_1_1,
                clientId: clientId,
                clean: true
            )
            
            mqttClient = MQTTClient(
                configuration: configuration,
                eventLoopGroupProvider: .shared(eventLoopGroup)
            )
            
            // Connect to broker
            try await mqttClient?.connect()
            
            // Update connection state
            isConnected = true
            connectionStatus = "Connected"
            logger.info("Successfully connected to MQTT broker")
            
            // Subscribe to IoTCraft topics
            await subscribeToTopics()
            
        } catch {
            connectionStatus = "Connection failed: \(error.localizedDescription)"
            logger.error("Failed to connect to MQTT broker: \(error)")
            mqttClient = nil
            throw error
        }
    }
    
    /// Disconnect from MQTT broker
    func disconnect() async {
        guard let mqttClient = mqttClient else { return }
        
        logger.info("Disconnecting from MQTT broker")
        connectionStatus = "Disconnecting..."
        
        do {
            try await mqttClient.disconnect()
        } catch {
            logger.error("Error during disconnect: \(error)")
        }
        
        self.mqttClient = nil
        isConnected = false
        connectionStatus = "Disconnected"
    }
    
    /// Subscribe to IoTCraft MQTT topics
    private func subscribeToTopics() async {
        guard let client = mqttClient, isConnected else {
            logger.warning("Cannot subscribe: not connected to MQTT broker")
            return
        }
        
        let topics = [
            "devices/announce",           // Device announcements
            "home/+/light",              // Device light controls
            "home/+/door",               // Device door controls  
            "home/+/position/set",       // Device position updates
            "home/sensor/+",             // Sensor readings
            "world/+/blocks",            // World block updates
            "world/+/state"              // World state changes
        ]
        
        for topic in topics {
            do {
                try await client.subscribe(to: topic, qos: .atLeastOnce)
                logger.info("Subscribed to topic: \(topic)")
            } catch {
                logger.error("Failed to subscribe to topic \(topic): \(error)")
            }
        }
        
        // Start listening for messages
        Task {
            await listenForMessages()
        }
    }
    
    /// Listen for incoming MQTT messages
    private func listenForMessages() async {
        guard let client = mqttClient else { return }
        
        for await message in client.messages {
            await handleIncomingMessage(message)
        }
    }
    
    /// Handle incoming MQTT messages
    private func handleIncomingMessage(_ message: MQTTMessage) async {
        let topic = message.topic
        let payload = message.payload.string ?? ""
        
        logger.debug("Received message on topic \(topic): \(payload)")
        
        // Parse message based on topic pattern
        if topic == "devices/announce" {
            await handleDeviceAnnouncement(payload)
        } else if topic.hasPrefix("home/") && topic.hasSuffix("/light") {
            await handleDeviceStateMessage(topic, payload, "light")
        } else if topic.hasPrefix("home/") && topic.hasSuffix("/door") {
            await handleDeviceStateMessage(topic, payload, "door")
        } else if topic.hasPrefix("home/") && topic.contains("/position/set") {
            await handleDevicePositionMessage(topic, payload)
        } else if topic.hasPrefix("home/sensor/") {
            await handleSensorMessage(topic, payload)
        } else if topic.hasPrefix("world/") {
            await handleWorldMessage(topic, payload)
        }
    }
    
    /// Handle device announcement messages
    private func handleDeviceAnnouncement(_ payload: String) async {
        do {
            let data = payload.data(using: .utf8) ?? Data()
            let announcement = try JSONDecoder().decode(MQTTDeviceAnnouncement.self, from: data)
            
            let device = IoTDevice(
                id: announcement.device_id,
                deviceType: DeviceType(rawValue: announcement.device_type) ?? .lamp,
                state: DeviceState(rawValue: announcement.state) ?? .online,
                position: announcement.location ?? Position(x: 0.0, y: 0.0, z: 0.0)
            )
            
            logger.info("Device announced: \(device.id) (\(device.deviceType.rawValue))")
            await onDeviceAnnouncement?(device)
            
        } catch {
            logger.error("Failed to parse device announcement: \(error)")
        }
    }
    
    /// Handle device state change messages
    private func handleDeviceStateMessage(_ topic: String, _ payload: String, _ stateType: String) async {
        // Extract device ID from topic: home/{device_id}/light
        let components = topic.split(separator: "/")
        guard components.count >= 3 else { return }
        
        let deviceId = String(components[1])
        var state: [String: Any] = [:]
        
        if stateType == "light" {
            let lightState = payload.uppercased() == "ON" ? LightState.on : LightState.off
            state["light"] = lightState.rawValue
        } else if stateType == "door" {
            let doorState = payload.uppercased() == "OPEN" ? DoorState.open : DoorState.closed
            state["door"] = doorState.rawValue
        }
        
        logger.info("Device \(deviceId) state changed: \(stateType) = \(payload)")
        await onDeviceStateChange?(deviceId, state)
    }
    
    /// Handle device position update messages
    private func handleDevicePositionMessage(_ topic: String, _ payload: String) async {
        let components = topic.split(separator: "/")
        guard components.count >= 2 else { return }
        
        let deviceId = String(components[1])
        
        do {
            let data = payload.data(using: .utf8) ?? Data()
            let position = try JSONDecoder().decode(Position.self, from: data)
            
            logger.info("Device \(deviceId) position updated: \(position)")
            await onDeviceStateChange?(deviceId, ["position": position])
            
        } catch {
            logger.error("Failed to parse position update: \(error)")
        }
    }
    
    /// Handle sensor messages
    private func handleSensorMessage(_ topic: String, _ payload: String) async {
        let sensorType = String(topic.split(separator: "/").last ?? "unknown")
        
        if let value = Double(payload) {
            logger.info("Sensor reading: \(sensorType) = \(value)")
            await onDeviceStateChange?("sensor", [sensorType: value])
        }
    }
    
    /// Handle world update messages
    private func handleWorldMessage(_ topic: String, _ payload: String) async {
        let worldId = String(topic.split(separator: "/")[1])
        let messageType = String(topic.split(separator: "/").last ?? "unknown")
        
        logger.info("World \(worldId) update: \(messageType)")
        await onWorldUpdate?(worldId, payload)
    }
    
    /// Publish device control command
    func publishDeviceCommand(_ deviceId: String, command: String, value: String) async throws {
        guard let client = mqttClient, isConnected else {
            throw MQTTClientError.notConnected
        }
        
        let topic = "home/\(deviceId)/\(command)"
        
        try await client.publish(value, to: topic, qos: .atLeastOnce)
        logger.info("Published command: \(topic) = \(value)")
    }
    
    /// Publish device position update
    func publishDevicePosition(_ deviceId: String, position: Position) async throws {
        guard let client = mqttClient, isConnected else {
            throw MQTTClientError.notConnected
        }
        
        let topic = "home/\(deviceId)/position/set"
        let encoder = JSONEncoder()
        let data = try encoder.encode(position)
        let positionJSON = String(data: data, encoding: .utf8) ?? "{}"
        
        try await client.publish(positionJSON, to: topic, qos: .atLeastOnce)
        logger.info("Published position update: \(deviceId) -> \(position)")
    }
}

// MARK: - Supporting Types

/// MQTT Device announcement message structure
private struct MQTTDeviceAnnouncement: Codable {
    let device_id: String
    let device_type: String
    let state: String
    let location: Position?
}

/// MQTT client specific errors
enum MQTTClientError: Error, LocalizedError {
    case notConnected
    case connectionFailed(String)
    case publishFailed(String)
    
    var errorDescription: String? {
        switch self {
        case .notConnected:
            return "Not connected to MQTT broker"
        case .connectionFailed(let message):
            return "Connection failed: \(message)"
        case .publishFailed(let message):
            return "Publish failed: \(message)"
        }
    }
}
