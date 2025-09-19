# IoTCraft MCP Protocol

[![Crates.io](https://img.shields.io/crates/v/iotcraft-mcp-protocol.svg)](https://crates.io/crates/iotcraft-mcp-protocol)
[![Documentation](https://docs.rs/iotcraft-mcp-protocol/badge.svg)](https://docs.rs/iotcraft-mcp-protocol)

Shared MCP (Model Context Protocol) protocol definitions for IoTCraft desktop client and mcplay orchestrator.

## Features

- 🔧 **Shared Tool Definitions**: All MCP tools defined in one place
- 🛡️ **Type Safety**: Shared types prevent API mismatches  
- ✅ **Input Validation**: Built-in parameter validation
- 📦 **Organized by Category**: Tools grouped logically
- 🌐 **Cross-Platform**: Works on desktop and web (WASM)
- 📖 **Well Documented**: Comprehensive API documentation

## Quick Start

Add this crate to your `Cargo.toml`:

```toml
[dependencies]
iotcraft-mcp-protocol = "0.1.0"
```

Basic usage:

```rust
use iotcraft_mcp_protocol::{get_all_tools, ToolCategory};

// Get all available MCP tools
let tools = get_all_tools();
println!("Available tools: {}", tools.len());

// Get tools by category
let system_tools = ToolCategory::System.tools();
for tool in system_tools {
    println!("Tool: {} - {}", tool.name, tool.description);
}
```

## Tool Categories

### System/Health Commands
- `ping` - Test connectivity with the server
- `get_client_info` - Get basic information about the desktop client
- `get_game_state` - Get current game state from the desktop client  
- `health_check` - Perform a health check on the desktop client
- `get_system_info` - Get system information from the desktop client

### World Building Commands
- `place_block` - Place a single block at specified coordinates
- `remove_block` - Remove a block at specified coordinates  
- `create_wall` - Create a wall/rectangular structure between two 3D coordinates

### Device Management Commands
- `list_devices` - List all IoT devices in the world with positions and types
- `spawn_device` - Create a new IoT device at specified coordinates
- `control_device` - Send a control command to an IoT device
- `move_device` - Move a device to new coordinates

### World Management Commands
- `publish_world` - Publish current world to be discoverable by other clients
- `unpublish_world` - Stop sharing the current world and return to single-player mode
- `join_world` - Join a shared world by world ID
- `leave_world` - Leave the current shared world and return to single-player mode
- `list_online_worlds` - List all discoverable shared worlds
- `get_multiplayer_status` - Get current multiplayer mode and world information

### Game Control Commands
- `player_move` - Move the player to a specific position
- `wait_for_condition` - Wait for a specific condition to be met (useful for test scenarios)
- `create_world` - Create a new world and set game state to InGame
- `load_world` - Load an existing world by name and set game state to InGame
- `set_game_state` - Set the current game state for UI transitions

## Type Definitions

### Core Types
- `McpTool` - Tool definition with name, description, and JSON schema
- `McpToolResult` - Tool execution result with content and error status
- `McpContent` - Content types (Text, JSON, Image)
- `McpError` - Error information with code and message

### IoTCraft-Specific Types
- `Position3D` - 3D coordinates for positioning
- `BlockType` - Supported block types (Grass, Dirt, Stone, etc.)
- `DeviceType` - Supported device types (Lamp, Door)
- `GameState` - Game states for UI transitions

## Validation

The crate includes comprehensive input validation:

```rust
use iotcraft_mcp_protocol::validation::{ToolValidator, validate_device_id};

// Validate tool parameters
let params = serde_json::json!({
    "block_type": "stone",
    "x": 10,
    "y": 5,
    "z": 15
});

ToolValidator::validate_tool_params("place_block", &params)?;

// Validate individual values
validate_device_id("lamp_01")?; // OK
validate_device_id("invalid id")?; // Error
```

## Protocol Constants

Access protocol constants and error codes:

```rust
use iotcraft_mcp_protocol::protocol::{error_codes, methods, DEFAULT_MCP_PORT};

// Standard error codes
println!("Method not found: {}", error_codes::METHOD_NOT_FOUND);

// Standard methods
println!("Tools list method: {}", methods::TOOLS_LIST);

// Configuration
println!("Default MCP port: {}", DEFAULT_MCP_PORT);
```

## Features

### `serde` (default)
Enables JSON serialization/deserialization support. Disable for no-std environments:

```toml
[dependencies]
iotcraft-mcp-protocol = { version = "0.1.0", default-features = false }
```

## Architecture

This crate is designed to be shared between:
- **IoTCraft Desktop Client**: Implements the MCP server
- **mcplay**: Orchestrator that consumes the MCP API
- **Future clients**: Any application needing IoTCraft MCP integration

## Contributing

1. Add new tools to the appropriate category in `src/tools.rs`
2. Update validation logic in `src/validation.rs`
3. Add tests for new functionality
4. Update documentation

## Version Compatibility

- Protocol Version: `2024-11-05`
- IoTCraft Desktop Client: `1.0.0+`
- mcplay: `1.0.0+`

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
