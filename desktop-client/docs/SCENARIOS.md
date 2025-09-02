# IoTCraft Scenario System

The IoTCraft scenario system provides a flexible, multi-format orchestration framework for testing complex multi-client interactions and world building scenarios.

## Features

- **Multi-format support**: RON, YAML, and JSON scenario files
- **Rich type system**: Strongly-typed scenario definitions with full validation
- **Infrastructure orchestration**: Automatic MQTT server and observer management
- **Complex timing control**: Parallel and sequential execution with retries and timeouts
- **Comprehensive validation**: Pre-conditions, post-conditions, and expectations
- **Extensible actions**: Built-in actions with support for custom extensions

## Formats Comparison

### RON (Rust Object Notation) - Recommended
```ron
// Complex scenario with rich type system
Scenario(
    name: "Complex World Building",
    description: Some("Comprehensive multi-client orchestration"),
    version: Some("1.2.0"),
    
    infrastructure: Infrastructure(
        mqtt_server: MqttServerConfig(
            enabled: true,
            port: Some(1883),
        ),
        mqtt_observer: Some(MqttObserverConfig(
            enabled: true,
            topics: Some(["world/#", "players/#"]),
        )),
    ),
    
    clients: [
        Client(
            id: "builder",
            client_type: "player",
            config: Some(ClientConfig(
                spawn_position: Some(Position(x: 0.0, y: 64.0, z: 0.0)),
                initial_inventory: Some([
                    InventoryItem(item_type: "stone", quantity: 64),
                ]),
            )),
        ),
    ],
    
    steps: [
        Step(
            name: "Initialize World",
            action: ClientAction(
                client_id: "builder",
                action_type: Connect,
            ),
            expectations: Some([
                ClientState(
                    client_id: "builder",
                    expected_state: "connected",
                    within_ms: Some(10000),
                ),
            ]),
        ),
    ],
)
```

**Advantages:**
- Native Rust syntax with excellent serde support
- Comments and documentation inline
- Strong typing with enums and structs
- Compact and readable
- No JSON escaping issues

### YAML - Human-Readable
```yaml
name: "Simple Collaboration"
description: "Two players working together"
version: "1.0.0"

infrastructure:
  mqtt_server:
    enabled: true
    port: 1883
  mqtt_observer:
    enabled: true
    topics:
      - "world/#"
      - "players/#"

clients:
  - id: "player1"
    client_type: "player"
    name: "Primary Builder"
    config:
      spawn_position:
        x: 0.0
        y: 64.0
        z: 0.0

steps:
  - name: "Connect Players"
    action:
      type: "ClientAction"
      params:
        client_id: "player1"
        action_type: "Connect"
```

**Advantages:**
- Widely known format
- Excellent for configuration
- Good tooling support
- Human-readable

### JSON - Universal Compatibility
```json
{
  "name": "Two Player World Sharing",
  "description": "Basic multi-client scenario",
  "version": "1.0.0",
  "infrastructure": {
    "mqtt_server": {
      "enabled": true,
      "port": 1883
    }
  },
  "clients": [
    {
      "id": "player1",
      "client_type": "player"
    }
  ]
}
```

**Advantages:**
- Universal format
- Wide tooling support
- Web-friendly

## Scenario Structure

### Core Components

1. **Infrastructure**: MQTT server, observer, and additional services
2. **Clients**: Player definitions with spawn positions and inventory
3. **Steps**: Orchestrated actions with timing and validation
4. **Configuration**: Global settings and environment variables

### Action Types

#### Basic Actions
- `Wait`: Pause execution for specified duration
- `MqttPublish`: Send MQTT message
- `MqttExpect`: Wait for MQTT message

#### Client Actions
- `Connect`: Client joins the world
- `Disconnect`: Client leaves the world
- `MoveTo`: Move to position
- `PlaceBlock`: Place a block
- `BreakBlock`: Break a block
- `UseItem`: Use inventory item
- `Chat`: Send chat message

#### Orchestration Actions
- `Parallel`: Execute multiple actions simultaneously
- `Sequence`: Execute actions in order
- `Custom`: Extensible custom actions

### Timing and Retries

```ron
timing: Some(Timing(
    delay_ms: Some(2000),     // Wait 2 seconds before starting
    timeout_ms: Some(30000),  // 30 second timeout
    retry: Some(RetryConfig(
        max_attempts: 3,
        delay_ms: 5000,
        backoff: Some(Exponential(base: 2.0)),
    )),
))
```

### Conditions and Expectations

#### Pre-conditions
```ron
conditions: Some([
    ClientConnected(client_id: "player1"),
    MqttTopicValue(
        topic: "world/ready",
        expected_value: "true",
        timeout_ms: Some(5000),
    ),
])
```

#### Post-conditions (Expectations)
```ron
expectations: Some([
    ClientState(
        client_id: "player1",
        expected_state: "building",
        within_ms: Some(10000),
    ),
    MqttMessage(
        topic: "world/block_placed",
        payload_pattern: Some("stone"),
        within_ms: Some(5000),
    ),
])
```

## Usage

### Interactive TUI (Recommended)

The mcplay binary now includes a Text User Interface (TUI) for interactive scenario management:

```bash
# Launch interactive TUI
cargo run --bin mcplay
```

**TUI Features:**
- 📋 **Visual Scenario Browser**: Navigate scenarios with arrow keys
- ✅ **Status Indicators**: See valid/invalid scenarios at a glance  
- 📖 **Detailed View**: Press `d` to view scenario information
- 🔍 **Quick Validation**: Press `v` to validate without running
- 🔄 **Live Refresh**: Press `r` to reload scenario list
- ⚡ **One-Click Execution**: Press `Enter` to run scenarios
- 🎯 **Smart Filtering**: Only valid scenarios can be executed

**TUI Controls:**
- `↑↓` - Navigate scenarios
- `Enter` - Run selected scenario
- `d` - Show scenario details
- `v` - Validate scenario
- `r` - Refresh scenario list
- `q`/`Esc` - Quit or go back

### Command Line Interface

```bash
# Run specific scenario (format auto-detected)
cargo run --bin mcplay scenarios/complex-world-building.ron
cargo run --bin mcplay scenarios/simple-collaboration.yaml
cargo run --bin mcplay scenarios/legacy-scenario.json

# Validate scenario file
cargo run --bin mcplay -- --validate scenarios/my-scenario.ron

# List all scenarios
cargo run --bin mcplay -- --list-scenarios

# Run with custom MQTT port
cargo run --bin mcplay -- --mqtt-port 1884 scenarios/test.ron

# Verbose output (enhanced with progress feedback)
cargo run --bin mcplay -- --verbose scenarios/debug.ron
```

### Enhanced Infrastructure Management

mcplay now includes improved MQTT server startup logic:

**Smart Port Management:**
- Pre-checks port availability before starting services
- Fast failure detection for occupied ports
- Clear error messages with resolution suggestions

**Progress Feedback (Verbose Mode):**
```bash
🔧 Starting infrastructure...
  Checking if MQTT port 1883 is available...
  Starting MQTT server on port 1883
  Waiting for MQTT server to become ready on port 1883...
    ⏳ Still waiting for port localhost:1883 (3s elapsed)...
    ⏳ Still waiting for port localhost:1883 (6s elapsed)...
    ✅ Port localhost:1883 is now available (attempt 15)
  ✅ MQTT server ready on port 1883
```

**Error Handling:**
- `❌ MQTT port 1883 is already in use. Please stop any existing MQTT brokers or choose a different port.`
- `❌ MQTT server failed to start on port 1883 within 30 second timeout`

### Creating Scenarios

1. **Choose format**: RON for complex scenarios, YAML for simple ones, JSON for compatibility
2. **Define infrastructure**: Configure MQTT server and observer
3. **Add clients**: Define players with spawn positions and inventory
4. **Create steps**: Orchestrate actions with proper timing
5. **Add validation**: Define expectations for each step

### Validation

The system provides comprehensive validation:
- **Schema validation**: Ensures all required fields are present
- **Type validation**: Checks data types and constraints
- **Reference validation**: Verifies client IDs and topic references
- **Logic validation**: Checks step dependencies and timing

### Example Workflow

```ron
// 1. Connect all clients in parallel
Step(
    name: "Initialize",
    action: Parallel(actions: [/* client connections */]),
),

// 2. Wait for world to be ready
Step(
    name: "Wait for World",
    action: MqttExpect(topic: "world/ready", timeout_ms: Some(10000)),
),

// 3. Execute coordinated building
Step(
    name: "Collaborative Build",
    action: Sequence(actions: [/* building actions */]),
    conditions: Some([ClientConnected(client_id: "builder")]),
    expectations: Some([MqttMessage(topic: "world/complete")]),
),
```

## Advanced Features

### Environment Variables
```ron
config: Some(ScenarioConfig(
    environment: Some({
        "WORLD_SEED": "12345",
        "DEBUG_MODE": "true",
        "COLLABORATION_MODE": "enabled",
    }),
))
```

### Logging Configuration
```ron
logging: Some(LoggingConfig(
    level: Some("debug"),
    log_mqtt: Some(true),
    log_client_actions: Some(true),
    filters: Some(["world/*", "players/*"]),
))
```

### Custom Services
```ron
services: Some({
    "world_generator": ServiceConfig(
        service_type: "external_process",
        enabled: true,
        config: Some({"binary": "/usr/local/bin/world-gen"}),
    ),
})
```

## Best Practices

1. **Start Simple**: Begin with basic JSON/YAML and migrate to RON for complex scenarios
2. **Use Comments**: RON supports comments - document your scenarios
3. **Modular Design**: Break complex scenarios into smaller, reusable steps
4. **Proper Timing**: Add appropriate delays and timeouts
5. **Comprehensive Validation**: Define expectations for all critical steps
6. **Error Handling**: Use retry logic for unreliable operations
7. **Resource Management**: Clean up clients and services in final steps

## Migration Guide

### From JSON to RON
1. Convert object syntax: `{}` → `()`
2. Remove quotes from field names: `"name"` → `name`
3. Use enum variants: `{"type": "Connect"}` → `Connect`
4. Add type constructors: `{}` → `Scenario(...)`

### From YAML to RON
1. Change array syntax: `- item` → `[item]`
2. Convert object syntax: `: value` → `: value,`
3. Use native types: `true/false` remain the same
4. Add constructors and remove `type` discriminators

## Troubleshooting

### Common Issues
- **Parse errors**: Check syntax for chosen format
- **Validation failures**: Ensure all required fields are present
- **Timeout errors**: Adjust timing configurations
- **MQTT connection issues**: Verify server configuration

### Debugging
- Use `--verbose` flag for detailed output
- Check log files in the generated `logs/` directory
- Validate scenarios before running with `--validate`
- Use MQTT observer logs to monitor message flow

## Future Enhancements

- **Visual scenario editor**: GUI for creating scenarios
- **Template system**: Reusable scenario templates
- **Performance metrics**: Built-in profiling and benchmarking
- **Extended validation**: Custom validation rules
- **Real-time monitoring**: Live scenario execution dashboard
