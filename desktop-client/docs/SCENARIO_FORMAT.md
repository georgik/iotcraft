# IoTCraft Scenario File Format

This document describes the JSON format for defining automated multi-client scenarios in IoTCraft.

## Overview

Scenarios allow you to define complex interactions between multiple clients using MCP tools, with proper synchronization and timing controls. This is useful for:

- Automated testing of multiplayer features
- Demonstrating IoTCraft capabilities
- Reproducible development scenarios
- Performance testing under specific conditions

## Scenario File Format

```json
{
  "name": "scenario_name",
  "description": "Brief description of what this scenario tests",
  "version": "1.0",
  "clients": [
    {
      "id": "client1",
      "player_id": "alice",
      "mcp_port": 3001,
      "type": "desktop"
    },
    {
      "id": "client2", 
      "player_id": "bob",
      "mcp_port": 3002,
      "type": "desktop"
    }
  ],
  "infrastructure": {
    "mqtt_server": {
      "required": true,
      "port": 1883
    },
    "mqtt_observer": {
      "required": false
    }
  },
  "steps": [
    {
      "name": "step_name",
      "description": "What this step does",
      "client": "client1",
      "action": {
        "type": "mcp_call",
        "tool": "publish_world",
        "arguments": {
          "world_name": "test_world",
          "max_players": 4,
          "is_public": true
        }
      },
      "wait_before": 0,
      "wait_after": 1000,
      "timeout": 10000,
      "success_condition": {
        "type": "mcp_response",
        "expected": "success"
      }
    }
  ],
  "synchronization": [
    {
      "name": "sync_point_name",
      "wait_for": ["step1", "step2"],
      "timeout": 30000
    }
  ]
}
```

## Field Descriptions

### Root Level
- `name`: Scenario identifier
- `description`: Human-readable description
- `version`: Scenario format version
- `clients`: Array of client configurations
- `infrastructure`: Required infrastructure services
- `steps`: Sequential actions to execute
- `synchronization`: Synchronization points between steps

### Client Configuration
- `id`: Unique client identifier for referencing in steps
- `player_id`: In-game player identifier
- `mcp_port`: MCP server port for this client
- `type`: Client type ("desktop", "web", "headless")

### Infrastructure
- `mqtt_server`: MQTT broker configuration
- `mqtt_observer`: Optional MQTT message monitoring

### Step Configuration
- `name`: Step identifier
- `description`: Human-readable step description
- `client`: Which client executes this step
- `action`: The action to perform
- `wait_before`: Milliseconds to wait before executing
- `wait_after`: Milliseconds to wait after executing
- `timeout`: Maximum execution time in milliseconds
- `success_condition`: How to determine if step succeeded

### Action Types

#### MCP Call
```json
{
  "type": "mcp_call",
  "tool": "tool_name",
  "arguments": {
    "param1": "value1",
    "param2": 123
  }
}
```

#### Wait Condition
```json
{
  "type": "wait_condition",
  "condition": "world_published",
  "expected_value": "test_world",
  "timeout": 5000
}
```

#### Console Command
```json
{
  "type": "console_command",
  "command": "list_online_worlds"
}
```

#### Delay
```json
{
  "type": "delay",
  "duration": 2000
}
```

## Success Conditions

### MCP Response
```json
{
  "type": "mcp_response",
  "expected": "success"
}
```

### World State
```json
{
  "type": "world_state",
  "check": "world_published",
  "expected": "test_world"
}
```

### Client Count
```json
{
  "type": "client_count",
  "world_id": "test_world",
  "expected": 2
}
```

## Example Scenarios

See `scenarios/` directory for example scenario files:
- `two_player_world_sharing.json` - Basic world creation and joining
- `ai_coordination.json` - Multiple AI agents building together
- `iot_smart_home.json` - IoT device management across clients
- `stress_test.json` - Performance testing with many clients

## Usage with mcplay

```bash
# Run a specific scenario
cargo run --bin mcplay -- scenarios/two_player_world_sharing.json

# Run with custom infrastructure
cargo run --bin mcplay -- scenarios/example.json --mqtt-port 1884

# Validate scenario file without running
cargo run --bin mcplay -- --validate scenarios/example.json

# List all available scenarios
cargo run --bin mcplay -- --list-scenarios

# Verbose mode for debugging
cargo run --bin mcplay -- scenarios/simple_test.json --verbose
```

## Integration with xtask

```bash
# Run scenario through xtask (automatically manages infrastructure)
cargo xtask multi-client --scenario scenarios/two_player_world_sharing.json

# Run scenario with custom settings
cargo xtask multi-client --scenario scenarios/example.json --mqtt-port 1884
```
