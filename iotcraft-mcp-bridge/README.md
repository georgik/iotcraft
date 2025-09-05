# IoTCraft MCP Bridge

A bridge application that connects Warp terminal's MCP client to the IoTCraft desktop client's MCP server.

## Purpose

This bridge follows the Model Context Protocol (MCP) architecture pattern:
- Warp terminal communicates via stdin/stdout JSON-RPC
- This bridge translates stdin/stdout JSON-RPC to TCP JSON-RPC 
- IoTCraft desktop client runs an MCP server on TCP (port 3001)

## Usage

### Prerequisites

1. Make sure the IoTCraft desktop client is running with the `--mcp` flag:
   ```bash
   cd ../desktop-client
   cargo run -- --mcp
   ```

### Running the Bridge

```bash
cargo run
```

Or build and run directly:
```bash
cargo build
./target/debug/iotcraft-mcp-bridge
```

### Environment Variables

- `MCP_PORT`: TCP port where desktop client MCP server is running (default: 3001)
- `RUST_LOG`: Log level (debug, info, warn, error)

## Architecture

```
Warp Terminal
     ↓ (stdin/stdout JSON-RPC)
IoTCraft MCP Bridge (this project)
     ↓ (TCP JSON-RPC on port 3001)  
IoTCraft Desktop Client
     ↓ (MQTT commands)
IoT Devices
```

## Integration with Warp

Configure Warp to use this bridge by adding to your MCP configuration:

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

## Error Handling

The bridge provides error handling for:
- Connection failures to desktop client
- Invalid JSON-RPC from stdin
- TCP connection issues
- Proper cleanup on exit

## Logging

Set `RUST_LOG=debug` for detailed request/response logging:
```bash
RUST_LOG=debug cargo run
```
