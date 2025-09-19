import ArgumentParser
import Foundation
import Logging

@main
struct IoTCraftSwiftClient: AsyncParsableCommand {
    static let configuration = CommandConfiguration(
        commandName: "iotcraft-swift-client",
        abstract: """
        IoTCraft Swift Client - Bird's Eye View IoT Visualizer
        
        A Swift 6.3-dev demonstration client that provides a top-down minimap view 
        of the IoTCraft world with simulated devices and world state.
        """,
        version: "1.0.0"
    )
    
    @Option(name: .shortAndLong, help: "MQTT broker hostname")
    var host: String = "localhost"
    
    @Option(name: .shortAndLong, help: "MQTT broker port") 
    var port: Int = 1883
    
    @Flag(name: .shortAndLong, help: "Enable verbose logging")
    var verbose: Bool = false
    
    @Flag(name: .long, help: "Run in terminal mode")
    var terminal: Bool = false
    
    func run() async throws {
        // Configure logging
        LoggingSystem.bootstrap { label in
            var logger = StreamLogHandler.standardOutput(label: label)
            logger.logLevel = verbose ? .debug : .info
            return logger
        }
        
        let logger = Logger(label: "iotcraft.swift-client")
        
        logger.info("🚀 Starting IoTCraft Swift Client v1.0.0")
        logger.info("📡 Target MQTT broker: \(host):\(port)")
        
        let displayMode: DisplayMode = terminal ? .terminal : .gui
        logger.info("🖥️  Display mode: \(displayMode)")
        
        // Create client on MainActor
        let client = await IoTCraftClient(
            mqttHost: host,
            mqttPort: port,
            displayMode: displayMode,
            logger: logger
        )
        
        do {
            try await client.start()
        } catch {
            logger.error("❌ Failed to start client: \(error)")
            throw ExitCode.failure
        }
    }
}
