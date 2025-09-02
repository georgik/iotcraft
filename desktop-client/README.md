# IoTCraft Desktop Client

A Rust-based desktop and web client for IoTCraft, built with Bevy game engine.

## Building and Running

### Desktop Version (Default)

The desktop version is now the default target. Simply run:

```bash
cargo run
# or
cargo r
```

For release builds:
```bash
cargo run --release
```

### Web Version

We now use a Rust-based build system (xtask) instead of shell scripts for better maintainability and portability.

#### Build web version
```bash
cargo xtask web-build --release
```

#### Serve web version locally
```bash
cargo xtask web-serve --port 8000
```

#### Build and serve (development)
```bash
cargo xtask web-dev --port 8000
```

#### Available xtask commands:
- `cargo xtask web-build [--release] [--output dist]` - Build the web version
- `cargo xtask web-serve [--port 8000] [--dir dist]` - Serve the built web version  
- `cargo xtask web-dev [--port 8000]` - Build and serve in one command
- `cargo xtask multi-client` - Run multiple desktop and web clients for testing
- `cargo xtask test unit` - Run unit tests only
- `cargo xtask test integration` - Run integration tests only
- `cargo xtask test mqtt` - Run MQTT-specific tests only
- `cargo xtask test wasm-unit` - Run WASM unit tests
- `cargo xtask test wasm-integration` - Run WASM integration tests
- `cargo xtask test all` - Run all tests (desktop + WASM)

## Testing

### Running Tests

The project uses `xtask` for comprehensive testing with automatic MQTT server management:

```bash
# Run all tests (recommended)
cargo xtask test all

# Run specific test suites
cargo xtask test unit        # Unit tests only
cargo xtask test integration # Integration tests with MQTT server
cargo xtask test mqtt        # MQTT-specific tests
```

The test system automatically:
- Starts MQTT server instances on different ports for isolation
- Manages server lifecycle (startup/cleanup)
- Runs tests in parallel safely
- Provides detailed logging for debugging

### Multi-Client Development Environment

The xtask system now supports running multiple desktop and web clients simultaneously for comprehensive multiplayer testing:

#### Option 1: Automated Multi-Client (Recommended)
```bash
# Run 2 desktop + 2 web clients with full infrastructure
cargo xtask multi-client --count 2 --web-clients 2 --full-env

# Run desktop clients only
cargo xtask multi-client --count 3 --full-env

# Run web clients only
cargo xtask multi-client --web-clients 3 --with-mqtt-server

# Custom configuration
cargo xtask multi-client \
  --count 2 \
  --web-clients 1 \
  --mqtt-port 1884 \
  --web-port 8080 \
  --browser-cmd firefox
```

#### Option 2: Manual Setup
```bash
# Terminal 1: Start infrastructure only
cargo xtask multi-client --count 0 --web-clients 0 --full-env

# Terminal 2: Manual desktop client
cargo run -- --player-id alice

# Terminal 3: Another desktop client
cargo run -- --player-id bob

# Browser: Manual web client
# Navigate to: http://localhost:8000?player=charlie
```

#### Client Configuration
- Desktop clients: Unique `--player-id` to avoid conflicts
- Web clients: Player ID passed via URL parameter (`?player=playerN`)
- Automatic infrastructure management (MQTT server, web server, observer)
- All clients automatically discover each other through MQTT
- Shared worlds are synchronized in real-time
- Use the in-game console to create/join multiplayer worlds

#### Features
- **Mixed Client Types**: Desktop and web clients in same session
- **Auto-detected Browsers**: Chrome, Chromium, Firefox, Safari support
- **Comprehensive Logging**: Separate log files for each client and server
- **Infrastructure Management**: MQTT server, web server, and observer
- **Cross-Platform Testing**: Test desktop/web synchronization

See [MULTI_CLIENT.md](MULTI_CLIENT.md) for detailed documentation and examples.

## Scenario Testing with mcplay

IoTCraft includes `mcplay`, a powerful scenario orchestration tool for testing complex multi-client interactions with an intuitive Text User Interface (TUI).

### Quick Start with mcplay

```bash
# Launch TUI to browse and run scenarios interactively
cargo run --bin mcplay

# Or run a specific scenario directly
cargo run --bin mcplay scenarios/orchestrator-test.ron
```

### TUI Features

The mcplay TUI provides an interactive experience for managing scenarios:

- **📋 Scenario Browser**: Navigate through all available scenarios with arrow keys
- **✅ Visual Validation**: See which scenarios are valid with status indicators
- **📖 Scenario Details**: Press `d` to view detailed scenario information
- **🔍 Quick Validation**: Press `v` to validate scenarios without running them
- **🔄 Live Refresh**: Press `r` to refresh the scenario list
- **⚡ One-Click Execution**: Press `Enter` to run scenarios directly
- **🎯 Smart Filtering**: Only shows valid scenarios for execution

### Command Line Options

```bash
# Interactive TUI (default when no arguments provided)
cargo run --bin mcplay

# List all available scenarios
cargo run --bin mcplay -- --list-scenarios

# Validate a scenario file
cargo run --bin mcplay -- --validate scenarios/my-scenario.ron

# Run with verbose output
cargo run --bin mcplay -- --verbose scenarios/test-scenario.ron

# Override MQTT port
cargo run --bin mcplay -- --mqtt-port 1884 scenarios/test-scenario.ron
```

### Scenario Format Support

mcplay supports multiple scenario formats with auto-detection:
- **RON** (`.ron`) - Recommended for complex scenarios with comments
- **JSON** (`.json`) - Universal compatibility
- **YAML** (`.yaml`, `.yml`) - Human-readable configuration

### Infrastructure Management

mcplay includes improved MQTT server startup logic:
- **Smart Port Checking**: Validates port availability before starting services
- **Enhanced Error Messages**: Clear feedback when ports are already in use
- **Progress Feedback**: Real-time status updates during infrastructure startup
- **Robust Timeout Handling**: Intelligent retries with exponential backoff

### Unified Scenario Format

mcplay now uses a unified scenario format that supports both simple mcplay scenarios and complex xtask orchestration scenarios:

- **Backward Compatibility**: Existing mcplay scenarios continue to work unchanged
- **Extended Capabilities**: Support for xtask-style actions like MQTT operations, parallel execution, and custom actions
- **Flexible Client Configuration**: Supports both simple client definitions and extended configuration with spawn positions, inventories, and permissions
- **Rich Action Types**: Includes MCP calls, MQTT publish/expect, client actions, parallel/sequence execution, and custom actions
- **Enhanced Validation**: Comprehensive validation with detailed error messages and warnings
- **Future-Proof Design**: Extensible architecture for adding new action types and configuration options

### Available Scenarios

mcplay automatically discovers scenarios in the `scenarios/` directory:
- `orchestrator-test.ron` - Basic orchestrator-only testing
- `simple_test.json` - Single-client block placement
- `two_player_world_sharing.json` - Multi-client collaboration
- And many more...

See **[docs/SCENARIOS.md](docs/SCENARIOS.md)** for comprehensive scenario documentation and **[CHANGELOG_MCPLAY.md](CHANGELOG_MCPLAY.md)** for recent mcplay enhancements.

## Model Context Protocol (MCP) Integration

IoTCraft supports the Model Context Protocol, enabling AI agents like Claude, Cursor, and ChatGPT to interact directly with your virtual world and IoT devices.

### Quick Start with MCP

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

### Available MCP Tools

**World Manipulation:**
- `place_block` - Place individual blocks
- `remove_block` - Remove blocks
- `create_wall` - Build walls between coordinates

**IoT Device Management:**
- `spawn_device` - Create new IoT devices (lamps, doors)
- `list_devices` - List all active devices
- `control_device` - Send commands (ON/OFF, open/close)
- `move_device` - Relocate devices

**Navigation & Observation:**
- `teleport_camera` - Move camera to coordinates
- `set_camera_angle` - Adjust viewing angle
- `get_world_status` - Get current world information
- `get_sensor_data` - Access sensor readings

**World Persistence:**
- `save_world` - Save current world state
- `load_world` - Load saved worlds

### Example AI Interactions

```
"Build me a simple house with stone walls at coordinates (0,0,0) to (10,10,10)"
"Set up a smart lighting system with 3 lamps and turn them on"
"Show me what's in the world and move the camera for a better view"
```

### Documentation

- **[docs/MCP_INTEGRATION.md](docs/MCP_INTEGRATION.md)** - Comprehensive setup and technical documentation
- **[docs/MCP_TESTING.md](docs/MCP_TESTING.md)** - Complete testing guide for MCP functionality

## Advanced Testing Scenarios

### MCP Testing Infrastructure

The project includes comprehensive MCP testing capabilities:

```bash
# Unit tests for MCP tool functions
cargo test mcp

# Integration tests (requires MCP server running)
cargo test --test mcp_integration_tests

# Comprehensive MCP test client
cargo run --bin mcp_test_client -- run-tests

# Interactive MCP testing
cargo run --bin mcp_test_client -- interactive
```

### Multi-Client MCP Testing

Combine multi-client testing with MCP for advanced scenarios:

```bash
# Run multiple clients with MCP enabled
cargo xtask multi-client --count 2 --full-env -- --mcp

# Test AI coordination across multiple clients
# Each client can be controlled by different AI agents
```

### Testing Documentation

- **[docs/MCP_TESTING.md](docs/MCP_TESTING.md)** - MCP-specific testing infrastructure
- **[MULTI_CLIENT.md](MULTI_CLIENT.md)** - Multi-client testing scenarios
- Built-in test fixtures with comprehensive edge case coverage
- Automated test reporting and validation

## Prerequisites

### Desktop
- Rust toolchain with Swift toolchain configured (on macOS)
- System dependencies for Bevy (graphics drivers, etc.)

### Web
- `wasm-pack` (automatically installed by xtask if missing)
- Python 3 (for local development server)

### Development & Testing
- MQTT server (iotcraft-mqtt-server) - automatically managed by xtask
- Supported browsers for web clients: Chrome, Chromium, Firefox, Safari
- No manual setup required - xtask handles all infrastructure

## CI/CD

The project includes GitHub Actions workflows for:
- **Desktop builds and tests** - Runs on pushes to desktop-client files
- **Web builds** - Builds web version automatically on pushes  
- **GitHub Pages deployment** - Manual deployment via workflow dispatch or automatic on main branch

To manually deploy to GitHub Pages:
1. Go to Actions tab in GitHub
2. Select "Desktop Client Web Build" workflow
3. Click "Run workflow" 
4. Check "Deploy to GitHub Pages" option

## Architecture

- **Desktop Client**: Native Rust application using Bevy
- **Web Client**: WASM compilation of the same codebase
- **Build System**: Rust-based `xtask` for web builds (replaces shell scripts)
- **Shared Code**: Most game logic works on both desktop and web
- **Platform-specific**: MQTT connectivity differs between native (rumqttc) and web (WebSocket)

## Features

- 3D voxel world with physics
- MQTT-based IoT device connectivity
- Multiplayer support with real-time synchronization
- **Model Context Protocol (MCP) integration** for AI agent interaction
- Console system with scripting
- Internationalization (i18n)
- Asset management
- Minimap and UI systems
- Comprehensive testing infrastructure
