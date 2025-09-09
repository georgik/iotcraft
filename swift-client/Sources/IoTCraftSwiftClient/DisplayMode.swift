import Foundation

/// Display mode for the Swift client
enum DisplayMode: String, CaseIterable, CustomStringConvertible, Sendable {
    case gui = "gui"
    case terminal = "terminal"
    
    var description: String {
        switch self {
        case .gui:
            return "SwiftUI GUI Interface"
        case .terminal:
            return "Terminal Text Mode"
        }
    }
}
