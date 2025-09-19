# MCP API Synchronization Strategy

## Problem Statement

The IoTCraft desktop client exposes an MCP (Model Context Protocol) API that is consumed by the sibling mcplay project. Currently, these projects have API definitions that can get out of sync, leading to:

1. **Missing Commands**: mcplay expects a `ping` command that desktop client doesn't implement
2. **API Drift**: No shared protocol definition between projects
3. **Maintenance Overhead**: Manual synchronization of API changes

## Current MCP Commands

### Desktop Client (Implemented)
Based on `src/mcp/mcp_tools.rs`, the desktop client implements these MCP tools:

#### World Building
- `place_block` - Place a single block at specified coordinates
- `remove_block` - Remove a block at specified coordinates  
- `create_wall` - Create a wall/rectangular structure between two 3D coordinates

#### Device Management
- `list_devices` - List all IoT devices in the world with positions and types
- `spawn_device` - Create a new IoT device at specified coordinates
- `control_device` - Send a control command to an IoT device
- `move_device` - Move a device to new coordinates

#### World Management
- `publish_world` - Publish current world to be discoverable by other clients
- `unpublish_world` - Stop sharing the current world and return to single-player mode
- `join_world` - Join a shared world by world ID
- `leave_world` - Leave the current shared world and return to single-player mode
- `list_online_worlds` - List all discoverable shared worlds
- `get_multiplayer_status` - Get current multiplayer mode and world information

#### Game Control
- `player_move` - Move the player to a specific position
- `wait_for_condition` - Wait for a specific condition to be met (useful for test scenarios)
- `create_world` - Create a new world and set game state to InGame
- `load_world` - Load an existing world by name and set game state to InGame
- `set_game_state` - Set the current game state for UI transitions

### mcplay (Expected but Missing from Desktop Client)
Based on `mcplay/src/main.rs`, mcplay expects these commands:

#### Health/Connectivity
- `ping` - **MISSING** - Send a ping to test connectivity
- `get_client_info` - **MISSING** - Get basic information about the desktop client
- `get_game_state` - **MISSING** - Get current game state from the desktop client
- `list_commands` - **MISSING** - List all available MCP commands/tools
- `health_check` - **MISSING** - Perform a health check on the desktop client
- `get_system_info` - **MISSING** - Get system information from the desktop client

#### Protocol Management
- `initialize` - **MISSING** - Initialize MCP connection with protocol version
- `tools/list` - Standard MCP protocol method to list available tools
- `tools/call` - Standard MCP protocol method to call tools

## Recommended Solution: Shared Protocol Crate

### Option 1: Workspace with Shared Protocol (Recommended)

Create a shared workspace structure:

```
iotcraft/
├── desktop-client/           # Current desktop client
├── mcplay/                  # Current mcplay  
├── iotcraft-mcp-protocol/   # New shared protocol crate
└── Cargo.toml              # Root workspace manifest
```

#### Benefits:
- **Single Source of Truth**: Protocol defined once, used everywhere
- **Type Safety**: Shared types prevent API mismatches
- **Automated Sync**: Changes to protocol automatically propagate
- **Versioning**: Can version the protocol independently
- **Documentation**: API docs generated from shared types

#### Implementation:
```rust
// iotcraft-mcp-protocol/src/lib.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

// Standard protocol commands
pub const PING_TOOL: McpTool = McpTool {
    name: "ping",
    description: "Test connectivity with the server",
    input_schema: json!({
        "type": "object",
        "properties": {},
        "required": []
    }),
};

// All tool definitions...
pub fn get_all_tools() -> Vec<McpTool> {
    vec![
        PING_TOOL,
        PLACE_BLOCK_TOOL,
        // ... etc
    ]
}
```

### Option 2: Git Submodules
- Share protocol as a git submodule
- More complex dependency management
- Good for completely separate repositories

### Option 3: Published Crate
- Publish `iotcraft-mcp-protocol` to crates.io
- Good for public projects
- Requires version management

## Implementation Plan

### Phase 1: Create Shared Protocol Crate
1. Create `iotcraft-mcp-protocol` crate in workspace
2. Move MCP types from desktop-client to shared crate
3. Add missing commands that mcplay expects
4. Update desktop-client to use shared crate
5. Update mcplay to use shared crate

### Phase 2: Add Missing Commands to Desktop Client
Based on mcplay expectations, add these tools:

```rust
// High priority (needed for mcplay compatibility)
- ping                 // Simple connectivity test
- get_client_info      // Basic client information
- get_game_state       // Current game state
- list_commands        // Available commands
- health_check         // Health status

// Standard MCP protocol methods
- initialize           // MCP connection setup
- tools/list          // Standard MCP tools listing
- tools/call          // Standard MCP tool execution
```

### Phase 3: Enhanced Protocol
Consider adding:
- Command versioning
- Capability negotiation
- Error code standardization
- Event streaming support

## File Structure for Shared Crate

```
iotcraft-mcp-protocol/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Main exports
│   ├── tools/           # Tool definitions
│   │   ├── mod.rs
│   │   ├── world.rs     # World building tools
│   │   ├── device.rs    # Device management tools  
│   │   ├── game.rs      # Game control tools
│   │   └── system.rs    # System/health tools
│   ├── types.rs         # Shared types
│   ├── protocol.rs      # MCP protocol constants
│   └── validation.rs    # Input validation
├── examples/            # Usage examples
└── README.md           # Protocol documentation
```

## Migration Strategy

1. **Gradual Migration**: Keep both old and new APIs during transition
2. **Feature Flags**: Use features to enable/disable shared protocol
3. **Backward Compatibility**: Maintain existing mcplay scenarios
4. **Testing**: Add integration tests between projects

## Next Steps

1. Create the shared protocol crate structure
2. Implement missing `ping` command first (quick win)
3. Move existing tools to shared crate
4. Update both projects to use shared protocol
5. Add comprehensive testing

This approach will ensure API consistency, reduce maintenance overhead, and provide a foundation for future MCP enhancements.
