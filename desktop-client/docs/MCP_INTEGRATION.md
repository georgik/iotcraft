# MCP (Model Context Protocol) Integration for IoTCraft

IoTCraft supports the Model Context Protocol (MCP), enabling AI agents like Claude, Cursor, ChatGPT, and others to directly interact with your virtual world and IoT devices.

## What is MCP?

MCP is an open standard that allows AI agents to securely interact with external tools and services. Similar to how Wokwi integrates with AI tools for embedded simulation, IoTCraft provides MCP support for voxel world manipulation and IoT device management.

## Quick Start

```bash
# Start IoTCraft with MCP server enabled
cargo run -- --mcp
```

This enables AI agents to:
- **Build structures**: Place blocks, create walls, build complex structures
- **Control IoT devices**: Spawn, move, and control virtual lamps and doors
- **Navigate the world**: Move camera, teleport, set viewing angles
- **Manage worlds**: Save, load, and get world status
- **Access sensor data**: Read IoT device telemetry

## Architecture

The MCP integration is built using Bevy's async-friendly patterns and supports multiple connection methods:

### Method 1: Direct Integration (Recommended)
- **Non-blocking execution**: All MCP operations run asynchronously without blocking the game's rendering pipeline
- **Plugin-based architecture**: The `McpPlugin` integrates seamlessly with Bevy's existing systems
- **Command integration**: MCP tool calls are converted to existing console commands for consistency
- **Extensible design**: Easy to add new tools and capabilities

### Method 2: Warp Terminal Bridge (Legacy)
For Warp terminal integration, the system follows this pattern:

```
Warp Terminal (stdin/stdout JSON-RPC)
        ↓
IoTCraft MCP Bridge (../iotcraft-mcp-bridge/)
        ↓ (TCP JSON-RPC on port 3001)
IoTCraft Desktop Client (with --mcp flag)
        ↓ (MQTT commands)
IoT Devices (ESP32-C6 lamps, doors, etc.)
```

## Configuration for AI Agents

### Claude Desktop / VS Code

Add this to your MCP configuration:

```json
{
  "servers": {
    "IoTCraft": {
      "type": "stdio",
      "command": "/path/to/iotcraft-desktop-client",
      "args": ["mcp"]
    }
  }
}
```

### Cursor

Similar configuration in your AI assistant settings:

```json
{
  "mcp": {
    "servers": {
      "IoTCraft": {
        "command": "/path/to/iotcraft-desktop-client",
        "args": ["mcp"],
        "type": "stdio"
      }
    }
  }
}
```

### Warp Terminal (Legacy Method)

Copy the `iotcraft_mcp_config.json` to your Warp MCP configuration directory:

```json
{
  "mcpServers": {
    "iotcraft": {
      "command": "cargo",
      "args": ["run"],
      "cwd": "/path/to/iotcraft-mcp-bridge",
      "env": {
        "MCP_PORT": "3001",
        "RUST_LOG": "info"
      }
    }
  }
}
```

## Available Tools

### World Manipulation

- **`place_block`**: Place individual blocks in the world
  ```json
  {
    "name": "place_block",
    "arguments": {
      "block_type": "stone",
      "x": 10,
      "y": 5,
      "z": 15
    }
  }
  ```

- **`remove_block`**: Remove blocks from the world
  ```json
  {
    "name": "remove_block", 
    "arguments": {
      "x": 10,
      "y": 5,
      "z": 15
    }
  }
  ```

- **`create_wall`**: Build walls of blocks between two points
  ```json
  {
    "name": "create_wall",
    "arguments": {
      "block_type": "quartz_block",
      "x1": 0, "y1": 0, "z1": 0,
      "x2": 10, "y2": 5, "z2": 0
    }
  }
  ```

### IoT Device Management

- **`spawn_device`**: Create new IoT devices in the world
  ```json
  {
    "name": "spawn_device",
    "arguments": {
      "device_id": "lamp_001",
      "device_type": "lamp",
      "x": 5.0,
      "y": 2.0,
      "z": 8.0
    }
  }
  ```

- **`list_devices`**: Get information about all active devices
  ```json
  {
    "name": "list_devices",
    "arguments": {}
  }
  ```

- **`control_device`**: Send commands to IoT devices
  ```json
  {
    "name": "control_device",
    "arguments": {
      "device_id": "lamp_001",
      "command": "ON"
    }
  }
  ```

- **`move_device`**: Relocate devices in the virtual world
  ```json
  {
    "name": "move_device",
    "arguments": {
      "device_id": "lamp_001",
      "x": 10.0,
      "y": 3.0,
      "z": 12.0
    }
  }
  ```

### Navigation and Observation

- **`teleport_camera`**: Move the camera/player to specific coordinates
  ```json
  {
    "name": "teleport_camera",
    "arguments": {
      "x": 50.0,
      "y": 20.0,
      "z": 30.0
    }
  }
  ```

- **`set_camera_angle`**: Adjust the viewing angle
  ```json
  {
    "name": "set_camera_angle",
    "arguments": {
      "yaw": 45.0,
      "pitch": -15.0
    }
  }
  ```

- **`get_world_status`**: Retrieve current world information
  ```json
  {
    "name": "get_world_status",
    "arguments": {}
  }
  ```

- **`get_sensor_data`**: Access IoT sensor readings
  ```json
  {
    "name": "get_sensor_data",
    "arguments": {}
  }
  ```

### World Persistence

- **`save_world`**: Save the current world state
  ```json
  {
    "name": "save_world",
    "arguments": {
      "filename": "my_creation.world"
    }
  }
  ```

- **`load_world`**: Load a previously saved world
  ```json
  {
    "name": "load_world",
    "arguments": {
      "filename": "my_creation.world"
    }
  }
  ```

## Supported Block Types

- `grass`
- `dirt`
- `stone`
- `quartz_block`
- `glass_pane`
- `cyan_terracotta`

## Supported Device Types

- `lamp`: Controllable lights with ON/OFF states
- `door`: Openable/closable doors

## Example AI Interactions

### Building a Simple House

*"Build me a simple house with stone walls and a grass roof at coordinates (0,0,0) to (10,10,10)"*

The AI can use multiple `place_block` or `create_wall` commands to construct the structure.

### IoT Smart Home Setup

*"Set up a smart lighting system with 3 lamps around the house and turn them on"*

The AI will:
1. Use `spawn_device` to create lamp devices
2. Use `control_device` to turn them on
3. Use `list_devices` to confirm the setup

### World Exploration

*"Show me what's currently in the world and move the camera to get a better view"*

The AI can:
1. Use `get_world_status` to see blocks and devices
2. Use `teleport_camera` to position the view
3. Use `set_camera_angle` to adjust the perspective

### Example Warp Commands (Legacy Method)

Once the Warp integration is set up, you can use natural language commands:

```
"Place a grass block at position 5, 10, 5"
"Turn on the lamp with ID lamp_01"  
"Show me the current temperature reading"
"Create a wall of stone blocks from 0,0,0 to 10,10,0"
"Teleport camera to position 0, 20, 0"
```

## MQTT Integration

The MCP tools can interact with real IoT devices through MQTT:

- Device control commands are sent via MQTT to control physical devices
- Sensor data is read from MQTT topics and made available through MCP tools
- Device announcements are monitored to keep track of available devices

## Testing the MCP Integration

Comprehensive testing infrastructure is available for MCP functionality:

```bash
# Unit tests for MCP tools
cargo test mcp

# Integration tests (requires MCP server running)
cargo test --test mcp_integration_tests

# Interactive testing with CLI client
cargo run --bin mcp_test_client -- interactive

# Comprehensive test suite
cargo run --bin mcp_test_client -- run-tests
```

### Multi-Client MCP Testing

Combine with multi-client testing for advanced scenarios:

```bash
# Run multiple clients with MCP enabled
cargo xtask multi-client --count 2 --full-env -- --mcp

# Each client can be controlled by different AI agents
# Test coordination and synchronization between AI-controlled clients
```

See **[MCP_TESTING.md](MCP_TESTING.md)** for detailed testing documentation.

## Technical Implementation

### Async Architecture

```rust
// MCP requests are processed in Bevy systems without blocking
fn process_mcp_requests(
    mut request_channel: ResMut<McpRequestChannel>,
    mut pending_commands: ResMut<PendingCommands>,
) {
    // Convert MCP tool calls to existing command system
    // Commands are queued and executed by existing systems
}
```

### Command Integration

MCP tool calls are converted to the existing console command format:
- `place_block(stone, 5, 3, 7)` → `"place stone 5 3 7"`
- `teleport_camera(10, 20, 30)` → `"tp 10 20 30"`

This ensures consistency with the existing scripting system and console interface.

### Error Handling

All MCP operations include proper error handling:
- Invalid parameters return descriptive error messages
- Failed operations don't crash the game
- Tool execution status is reported back to the AI agent

## Extending the System

To add new MCP tools:

1. **Define the tool** in `mcp_tools.rs`:
   ```rust
   McpTool {
       name: "my_new_tool".to_string(),
       description: "Description of what this tool does".to_string(),
       input_schema: json!({
           "type": "object",
           "properties": {
               "param1": {"type": "string", "description": "Parameter description"}
           },
           "required": ["param1"]
       }),
   }
   ```

2. **Implement execution** in the `execute_mcp_tool` function:
   ```rust
   "my_new_tool" => execute_my_new_tool(arguments, world),
   ```

3. **Add command conversion** if needed in `convert_tool_call_to_command`

4. **Add corresponding tests** in `src/mcp/tests.rs` and update test fixtures

## Development Components

The MCP integration consists of several components:

**Desktop Client (this directory):**
- `src/mcp/mcp_server.rs` - TCP JSON-RPC server in the desktop client
- `src/mcp/mcp_tools.rs` - Available MCP tools and their implementations
- `src/mcp/mcp_types.rs` - MCP data structures and types

**Bridge (../iotcraft-mcp-bridge/):**
- `src/main.rs` - Bridge executable for Warp integration

## Troubleshooting

### Common Issues

1. **MCP server not starting**: Check that all dependencies are installed
2. **Tool calls failing**: Verify parameter format matches the JSON schema
3. **Commands not executing**: Check the logs for error messages in the application

### Bridge Connection Issues (Warp Method)
- Make sure the desktop client is running with `--mcp` flag
- Check that port 3001 is not in use by another application
- Verify the MCP_PORT environment variable matches

### Warp Configuration Issues  
- Ensure the `cwd` path in the configuration points to the correct directory
- Check that `cargo` is available in your PATH
- Verify the configuration file is in the correct location for Warp

### MQTT Issues
- Ensure your MQTT broker is running and accessible
- Check MQTT configuration in the desktop client
- Verify IoT devices are connected and publishing to expected topics

### Debug Information

Enable debug logging to see MCP message flow:
```bash
RUST_LOG=debug ./iotcraft-desktop-client
```

### Performance

MCP operations are designed to be lightweight:
- Read-only operations (like `get_world_status`) execute immediately
- Write operations (like `place_block`) are queued and batched for efficiency
- No operations block the game's rendering pipeline

## Security Considerations

- MCP tools have the same permissions as console commands
- No file system access beyond designated world save directories
- IoT device control is limited to the virtual simulation
- All operations are logged for audit purposes

## Recent Improvements and Notes

### MQTT Server Readiness Detection

Recent improvements include enhanced MQTT server readiness detection:
- **Asynchronous port checking**: Replaced blocking TCP connections with async tokio-based detection
- **Improved reliability**: 1-second timeout with 500ms polling intervals for better server detection
- **Debug logging**: Detailed logs for troubleshooting connection issues
- **Non-blocking execution**: Port checks no longer block the main application thread

This ensures MCP integration works reliably with both local and remote MQTT brokers.

### Multi-Client AI Coordination

The MCP integration now supports advanced multi-client scenarios:
- Multiple AI agents can control different clients simultaneously
- Shared world state synchronization via MQTT
- Coordinated building and IoT device management across clients
- Real-time collaboration between human and AI participants

### Testing Infrastructure

Comprehensive testing ensures MCP reliability:
- Unit tests for all MCP tool functions
- Integration tests with real server communication
- CLI test client for manual and automated validation
- Test fixtures covering edge cases and error conditions

## Future Enhancements

- **TCP/WebSocket support**: Currently uses stdio, can be extended for network communication
- **Resource system**: Expose world files and configurations as MCP resources
- **Streaming updates**: Real-time world state changes sent to AI agents
- **Multi-agent support**: Multiple AI agents collaborating in the same world
- **Advanced sensor data**: More detailed IoT device telemetry and control

## Related Documentation

- **[MCP_TESTING.md](MCP_TESTING.md)** - Complete testing infrastructure documentation
- **[../MULTI_CLIENT.md](../MULTI_CLIENT.md)** - Multi-client testing for advanced scenarios
- **[../README.md](../README.md)** - Main project documentation with MCP overview

This MCP integration opens up exciting possibilities for AI-assisted world building, IoT device management, and creative collaboration in your virtual environment!
