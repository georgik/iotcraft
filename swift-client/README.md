# IoTCraft Swift Client 🦅

A cutting-edge Swift 6.3-dev demonstration client that provides a bird's eye view of the IoTCraft world. This client showcases modern Swift concurrency features while providing a simplified, top-down visualization of IoT devices and world blocks.

## 🎯 Project Goals

This Swift client was created to evaluate Swift's capabilities for IoT visualization and compare it with the existing Rust desktop client. It demonstrates:

- ✅ **Swift 6.3-dev Features**: Modern concurrency, strict concurrency checking, @MainActor isolation
- ✅ **Real-time Simulation**: Dynamic device states, position updates, and world changes
- ✅ **ASCII Map Visualization**: Terminal-based bird's eye view of the IoTCraft world
- ✅ **Structured Concurrency**: TaskGroup-based parallel task management
- ✅ **Type Safety**: Sendable types with compile-time concurrency safety

## 🏗️ Architecture

### Core Components

- **`IoTCraftClient`**: Main client with @MainActor isolation for UI safety
- **`Models.swift`**: Sendable data types (IoTDevice, WorldState, BlockType)
- **`DisplayMode`**: GUI vs Terminal rendering modes
- **Structured Concurrency**: Parallel device simulation, world updates, and display rendering

### Swift 6.3-dev Features Demonstrated

```swift
// @MainActor isolation for UI safety
@MainActor
final class IoTCraftClient: ObservableObject {
    @Published private(set) var devices: [String: IoTDevice] = [:]
}

// Structured concurrency with TaskGroup
simulationTask = Task {
    try await withThrowingTaskGroup(of: Void.self) { group in
        group.addTask { try await self.simulateDevices() }
        group.addTask { try await self.simulateWorld() }  
        group.addTask { try await self.runDisplay() }
        try await group.waitForAll()
    }
}

// Sendable types for concurrency safety
struct IoTDevice: Codable, Identifiable, Sendable {
    let id: String
    let type: DeviceType
    var state: DeviceState
    var position: Position
}
```

## 🚀 Quick Start

### Prerequisites

- **Swift 6.3-dev** (installed via swiftly)
- **macOS 13+** 

### Installation

```bash
# Switch to Swift 6.3-dev (if not already)
swiftly use main-snapshot-2025-09-07

# Navigate to the Swift client directory
cd iotcraft/swift-client

# Build the project
swift build

# Run with help to see options
swift run iotcraft-swift-client --help
```

### Running the Client

```bash
# Terminal mode (default) - shows ASCII bird's eye view
swift run iotcraft-swift-client --terminal --verbose

# GUI mode (placeholder)
swift run iotcraft-swift-client

# Connect to specific MQTT broker
swift run iotcraft-swift-client --host mqtt.example.com --port 1883

# Verbose logging
swift run iotcraft-swift-client --terminal --verbose
```

### Swift Testing Utility (Rust-first Inspired)

Following IoTCraft's Rust-first approach for tooling, we've created a native Swift testing utility:

```bash
# Quick demonstration of Swift 6.3-dev features
swift run swift-test-runner quick-demo --verbose

# Run comprehensive test suite (5 second duration)
swift run swift-test-runner run-tests --duration 5

# Performance benchmarks (500 iterations)
swift run swift-test-runner benchmark-test --iterations 500

# Validate Swift 6.3-dev concurrency features
swift run swift-test-runner validate-swift-features --verbose

# Show all available test commands
swift run swift-test-runner --help
```

## 🖥️ Terminal Output

The terminal mode provides a real-time ASCII visualization:

```
============================================================
🦅 IoTCraft Swift Client - Bird's Eye View
============================================================
🔗 Status: Connected (Simulated)
📊 Stats: 3 devices, 7 blocks

📍 Device Positions:
  🟢 🚪 🔋 at (5.0, 3.0)
  🟢 🌡️ 🔋 at (-2.0, 4.0)
  🟢 💡 💡 at (0.0, 0.0)

🗺️  World Map (Top-Down View):
   0123456789
 0 ⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜
 1 ⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜
 2 ⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜
 3 ⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜
 4 ⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜
 5 ⬜⬜⬜⬜⬜💡🟩⬜⬜⬜
 6 ⬜⬜⬜⬜⬜🟫🟫⬜⬜⬜
 7 ⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜
 8 ⬜⬜⬜⬜⬜⬜⬜⬜⬜⬜
 9 ⬜⬜⬜🌡️⬜⬜⬜⬜⬜⬜
============================================================
Swift 6.3-dev Features in use:
  ✓ @MainActor isolation
  ✓ Structured concurrency with TaskGroup
  ✓ Async sequences and modern patterns
  ✓ Sendable types with strict concurrency
============================================================
```

## 📊 Swift vs Rust Comparison

| Feature | Swift Client | Rust Desktop Client |
|---------|-------------|----------------------|
| **Concurrency** | Structured concurrency, @MainActor | Bevy ECS with async systems |
| **Type Safety** | Sendable types, strict concurrency | Send/Sync traits, Arc/Mutex |
| **Memory Safety** | ARC, no manual memory management | Ownership system, zero-cost abstractions |
| **Performance** | JIT compilation, runtime overhead | Compiled, zero-cost abstractions |
| **Visualization** | ASCII terminal maps | Full 3D rendering with Bevy |
| **MQTT Integration** | Simulated (placeholder) | Full MQTT-NIO integration |
| **Build Speed** | Fast incremental builds | Longer initial builds |
| **Cross-platform** | Apple ecosystems primarily | Windows, macOS, Linux, WASM |

## 🔧 Development

### Project Structure

```
swift-client/
├── Package.swift              # Swift Package Manager configuration
├── Sources/IoTCraftSwiftClient/
│   ├── main.swift            # CLI entry point with ArgumentParser
│   ├── IoTCraftClient.swift  # Main client implementation
│   ├── Models.swift          # Data models (IoTDevice, WorldState, etc.)
│   └── DisplayMode.swift     # Display mode enumeration
├── Tests/                    # Unit tests (to be implemented)
└── README.md                # This file
```

### Key Dependencies

- **swift-log**: Structured logging
- **swift-argument-parser**: Command-line interface
- **Foundation**: Core Swift functionality

### Swift 6.3-dev Compilation

The project uses the `-parse-as-library` flag to handle Swift 6.3-dev's stricter @main attribute requirements:

```swift
// Package.swift
.unsafeFlags([\"-parse-as-library\"]),
```

## 🧪 Testing

Currently implemented as a simulation-based client with mock data:

- **Device Simulation**: 3 IoT devices (lamp, door, sensor) with dynamic state changes
- **World Simulation**: Block placement and updates
- **Real-time Updates**: Position changes, light toggles, new block placement

### Future MQTT Integration

The client is architected to support real MQTT integration:

```swift
// TODO: Replace simulation with real MQTT client
// private var mqttClient: MQTTClient?
// private let eventLoopGroup = MultiThreadedEventLoopGroup(numberOfThreads: 1)
```

## 🎯 Swift Technology Evaluation

### ✅ Strengths Observed

1. **Modern Concurrency**: Swift 6.3-dev's async/await and structured concurrency provide excellent developer experience
2. **Type Safety**: Sendable types and @MainActor isolation prevent data races at compile-time
3. **Rapid Prototyping**: Quick iteration and development cycle
4. **Memory Safety**: ARC eliminates manual memory management concerns
5. **Platform Integration**: Natural fit for Apple ecosystem integration

### ⚠️ Limitations Identified

1. **Performance Overhead**: Runtime compilation and ARC have performance costs vs Rust
2. **Cross-platform Reach**: Primarily Apple ecosystems, limited elsewhere
3. **Ecosystem Maturity**: Fewer specialized libraries (e.g., MQTT, game engines) vs Rust
4. **Binary Size**: Larger runtime requirements compared to Rust
5. **Real-time Constraints**: Less predictable performance for embedded/real-time systems

### 🔍 Suitability Assessment

**For IoTCraft Client Development:**

- ✅ **Excellent for**: Rapid prototyping, Apple ecosystem clients, developer productivity
- ⚠️ **Consider for**: Cross-platform desktop applications, performance-critical visualization
- ❌ **Avoid for**: Embedded targets, WASM deployment, maximum performance requirements

## 🛠️ Future Enhancements

- [ ] **Real MQTT Integration**: Replace simulation with actual MQTT-NIO client
- [ ] **SwiftUI GUI**: Rich graphical interface for device control
- [ ] **Interactive Controls**: Click-to-control devices via MQTT
- [ ] **World Editing**: Add/remove blocks through Swift client
- [ ] **Integration Testing**: Connect with mcplay scenarios
- [ ] **Performance Benchmarking**: Compare with Rust client performance

