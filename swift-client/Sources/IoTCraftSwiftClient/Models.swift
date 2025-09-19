import Foundation

// MARK: - IoT Device Models

/// Represents an IoT device in the IoTCraft world
struct IoTDevice: Codable, Identifiable, Sendable {
    let id: String
    let deviceType: DeviceType  // Alias for compatibility
    var type: DeviceType { deviceType } // Computed property for backward compatibility
    var state: DeviceState
    var position: Position
    var lightState: LightState = .off
    var doorState: DoorState = .closed
    var lastSeen: Date = Date()
    var batteryLevel: Double = 100.0
    var signalStrength: Double = -50.0
    
    init(id: String, deviceType: DeviceType, state: DeviceState, position: Position) {
        self.id = id
        self.deviceType = deviceType
        self.state = state
        self.position = position
    }
    
    // Legacy initializer for backward compatibility
    init(id: String, type: DeviceType, state: DeviceState, position: Position) {
        self.init(id: id, deviceType: type, state: state, position: position)
    }
}

/// Device types supported by IoTCraft
enum DeviceType: String, CaseIterable, Codable, Sendable {
    case lamp = "lamp"
    case door = "door"
    case sensor = "sensor"
    
    var emoji: String {
        switch self {
        case .lamp: return "💡"
        case .door: return "🚪"
        case .sensor: return "🌡️"
        }
    }
}

/// Device connection state
enum DeviceState: String, CaseIterable, Codable, Sendable {
    case online = "online"
    case offline = "offline"
}

/// Light state for controllable devices
enum LightState: String, CaseIterable, Codable, Sendable {
    case on = "ON"
    case off = "OFF"
}

/// Door state for door devices
enum DoorState: String, CaseIterable, Codable, Sendable {
    case open = "OPEN"
    case closed = "CLOSED"
}

/// 3D position in the IoTCraft world
struct Position: Codable, Sendable, CustomStringConvertible {
    let x: Float
    let y: Float
    let z: Float
    
    var description: String {
        return String(format: "(%.1f, %.1f, %.1f)", x, y, z)
    }
    
    /// Convert to 2D coordinates for top-down view
    var topDown: CGPoint {
        return CGPoint(x: CGFloat(x), y: CGFloat(z))
    }
}

// MARK: - MQTT Message Models

/// Device announcement message structure
struct DeviceAnnouncement: Codable, Sendable {
    let device_id: String
    let device_type: String
    let state: String
    let location: Position
}

// MARK: - World State Models

/// Represents the voxel world state
@MainActor
final class WorldState: ObservableObject, Sendable {
    @Published private(set) var blocks: [Position: BlockType] = [:]
    
    var blockCount: Int {
        return blocks.count
    }
    
    func updateBlocks(_ blockData: [BlockData]) {
        for block in blockData {
            let position = Position(x: Float(block.x), y: Float(block.y), z: Float(block.z))
            blocks[position] = BlockType(rawValue: block.block_type) ?? .grass
        }
    }
    
    func addBlock(at position: Position, type: BlockType) {
        blocks[position] = type
    }
    
    func removeBlock(at position: Position) {
        blocks.removeValue(forKey: position)
    }
    
    /// Get blocks visible in a 2D top-down view within bounds
    func blocksInBounds(minX: Float, maxX: Float, minZ: Float, maxZ: Float) -> [(Position, BlockType)] {
        return blocks.compactMap { (position, blockType) in
            if position.x >= minX && position.x <= maxX && 
               position.z >= minZ && position.z <= maxZ {
                return (position, blockType)
            }
            return nil
        }
    }
}

/// Block data from MQTT messages
struct BlockData: Codable, Sendable {
    let x: Int
    let y: Int
    let z: Int
    let block_type: String
}

/// Block types in the IoTCraft world
enum BlockType: String, CaseIterable, Codable, Sendable {
    case grass = "grass"
    case dirt = "dirt"
    case stone = "stone"
    case quartzBlock = "quartz_block"
    case glassPane = "glass_pane"
    case cyanTerracotta = "cyan_terracotta"
    
    var color: RGB {
        switch self {
        case .grass: return RGB(red: 0.2, green: 0.8, blue: 0.2)
        case .dirt: return RGB(red: 0.6, green: 0.4, blue: 0.2)
        case .stone: return RGB(red: 0.5, green: 0.5, blue: 0.5)
        case .quartzBlock: return RGB(red: 0.9, green: 0.9, blue: 0.9)
        case .glassPane: return RGB(red: 0.8, green: 0.8, blue: 1.0)
        case .cyanTerracotta: return RGB(red: 0.2, green: 0.8, blue: 0.8)
        }
    }
    
    var emoji: String {
        switch self {
        case .grass: return "🟩"
        case .dirt: return "🟫"
        case .stone: return "⬜"
        case .quartzBlock: return "⬜"
        case .glassPane: return "🟦"
        case .cyanTerracotta: return "🟪"
        }
    }
}

/// RGB color representation
struct RGB: Sendable {
    let red: Float
    let green: Float
    let blue: Float
}

// MARK: - Extensions for Hashing and Equality

extension Position: Hashable, Equatable {
    func hash(into hasher: inout Hasher) {
        hasher.combine(x)
        hasher.combine(y)
        hasher.combine(z)
    }
    
    static func == (lhs: Position, rhs: Position) -> Bool {
        return lhs.x == rhs.x && lhs.y == rhs.y && lhs.z == rhs.z
    }
}

// MARK: - Core Graphics Extensions

#if canImport(CoreGraphics)
import CoreGraphics

extension RGB {
    /// Convert to CGColor for rendering
    var cgColor: CGColor {
        return CGColor(red: CGFloat(red), green: CGFloat(green), blue: CGFloat(blue), alpha: 1.0)
    }
}
#endif

// MARK: - Convenience Initializers

extension Position {
    /// Create position from integers
    init(x: Int, y: Int, z: Int) {
        self.init(x: Float(x), y: Float(y), z: Float(z))
    }
    
    /// Origin position
    static let origin = Position(x: 0, y: 0, z: 0)
}

extension IoTDevice {
    /// Create a lamp device at origin
    static func lamp(id: String) -> IoTDevice {
        return IoTDevice(id: id, type: .lamp, state: .online, position: .origin)
    }
    
    /// Create a door device at position
    static func door(id: String, at position: Position) -> IoTDevice {
        return IoTDevice(id: id, type: .door, state: .online, position: position)
    }
}
