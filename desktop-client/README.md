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

## Web Version (WASM)

The IoTCraft desktop-client compiles to WebAssembly for browser deployment, enabling cross-platform testing with both native desktop and web clients. We use a Rust-based build system (ctask) instead of shell scripts for better maintainability and portability.

#### Build web version
```bash
cargo ctask web-build --release
```

#### Serve web version locally
```bash
cargo ctask web-serve --port 8000
```

#### Build and serve (development)
```bash
cargo ctask web-dev --port 8000
```

#### Cross-Platform Integration with mcplay

The WASM client integrates seamlessly with mcplay orchestration for sophisticated cross-platform testing:

```bash
# mcplay automatically builds, serves, and launches WASM client in browser
cd ../mcplay && cargo run scenarios/alice_desktop_bob_wasm_visual.ron

# Extended manual testing with browser client
cd ../mcplay && cargo run scenarios/alice_desktop_bob_wasm_visual.ron --keep-alive
```

**Key Features:**
- **🌐 Automatic Browser Management**: mcplay launches Chrome/Firefox and tracks as managed process
- **🔄 Real-time Synchronization**: Desktop ↔ Web client multiplayer interactions
- **📊 Visual Test Interface**: Rich manual testing instructions displayed in mcplay TUI
- **🎮 Full Feature Parity**: Same game mechanics, MQTT integration, and world building capabilities

#### Available ctask commands:
- `cargo ctask web-build [--release] [--output dist]` - Build the web version
- `cargo ctask web-serve [--port 8000] [--dir dist]` - Serve the built web version  
- `cargo ctask web-dev [--port 8000]` - Build and serve in one command
- `cargo ctask multi-client` - Run multiple desktop and web clients for testing
- `cargo ctask test unit` - Run unit tests only
- `cargo ctask test integration` - Run integration tests only
- `cargo ctask test mqtt` - Run MQTT-specific tests only
- `cargo ctask test wasm-unit` - Run WASM unit tests
- `cargo ctask test wasm-integration` - Run WASM integration tests
- `cargo ctask test all` - Run all tests (desktop + WASM)

## Testing

### Running Tests

The project uses `ctask` for comprehensive testing with automatic MQTT server management:

```bash
# Run all tests (recommended)
cargo ctask test all

# Run specific test suites
cargo ctask test unit        # Unit tests only
cargo ctask test integration # Integration tests with MQTT server
cargo ctask test mqtt        # MQTT-specific tests
```

The test system automatically:
- Starts MQTT server instances on different ports for isolation
- Manages server lifecycle (startup/cleanup)
- Runs tests in parallel safely
- Provides detailed logging for debugging

### Multi-Client Development Environment

The ctask system now supports running multiple desktop and web clients simultaneously for comprehensive multiplayer testing:

#### Option 1: Automated Multi-Client (Recommended)
```bash
# Run 2 desktop + 2 web clients with full infrastructure
cargo ctask multi-client --count 2 --web-clients 2 --full-env

# Run desktop clients only
cargo ctask multi-client --count 3 --full-env

# Run web clients only
cargo ctask multi-client --web-clients 3 --with-mqtt-server

# Custom configuration
cargo ctask multi-client \
  --count 2 \
  --web-clients 1 \
  --mqtt-port 1884 \
  --web-port 8080 \
  --browser-cmd firefox
```

#### Option 2: Manual Setup
```bash
# Terminal 1: Start infrastructure only
cargo ctask multi-client --count 0 --web-clients 0 --full-env

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

## Cross-Platform Testing with mcplay

IoTCraft includes `mcplay`, a comprehensive multi-client orchestration platform for testing complex interactions between desktop and web clients with an intuitive Text User Interface (TUI). mcplay enables sophisticated cross-platform testing scenarios with automated infrastructure management.

### Quick Start with mcplay

```bash
# Launch TUI to browse and run scenarios interactively
cd ../mcplay && cargo run

# Or run a specific scenario directly
cd ../mcplay && cargo run scenarios/alice_desktop_bob_wasm_visual.ron

# Run with keep-alive for extended manual testing
cd ../mcplay && cargo run scenarios/alice_desktop_bob_wasm_visual.ron --keep-alive
```

### Cross-Platform Orchestration Features

The mcplay orchestration platform provides comprehensive multi-client testing capabilities:

#### **🎭 Multi-Client Management**
- **Desktop + WASM Clients**: Seamlessly orchestrate both native desktop and browser-based WASM clients
- **Automated Browser Launching**: Automatically opens browsers for WASM clients with proper process tracking
- **Cross-Platform Synchronization**: Test real-time multiplayer interactions between desktop and web
- **Client Health Monitoring**: Advanced readiness and liveness probes for all client types

#### **🏗️ Infrastructure Automation**
- **MQTT Server Management**: Automatic startup and configuration of MQTT broker infrastructure
- **Web Server Management**: Builds and serves WASM clients with automated port management
- **Observer Integration**: MQTT message monitoring and logging for debugging
- **Clean Shutdown**: Proper cleanup of all processes and resources

#### **🎨 Enhanced TUI Interface Features**
- **📋 Advanced Scenario Browser**: Navigate through all available scenarios with arrow keys
- **🔍 Modal Search Dialog**: Press `/` to open centered search with:
  - **Live filtering** and **text highlighting** in yellow
  - **Smart previews** showing context from descriptions  
  - **Results counter** displaying "Found: X of Y scenarios"
  - **Real-time updates** as you type
- **✅ Visual Status Indicators**: Real-time process status with Kubernetes-style indicators (⏳🟡🟢🔴🔵🟠)
- **📊 Enhanced Multi-Pane Layout**: Optimized 35%/65% split with improved progress bars
- **📖 Scenario Details**: Press `d` to view detailed scenario information
- **🔍 Quick Validation**: Press `v` to validate scenarios without running them
- **🔄 Live Refresh**: Press `r` to refresh the scenario list
- **⚡ One-Click Execution**: Press `Enter` to run scenarios directly
- **⌨️ Unix-Friendly Controls**: Support for `q`/`Esc`/`Ctrl+C` to quit

### Command Line Options

```bash
# Interactive TUI (default when no arguments provided)
cd ../mcplay && cargo run

# List all available scenarios
cd ../mcplay && cargo run -- --list-scenarios

# Validate a scenario file
cd ../mcplay && cargo run -- --validate scenarios/alice_desktop_bob_wasm_visual.ron

# Run with verbose output and keep-alive for extended testing
cd ../mcplay && cargo run -- --verbose --keep-alive scenarios/full_orchestration.ron

# Override MQTT port for custom configurations
cd ../mcplay && cargo run -- --mqtt-port 1884 scenarios/test-scenario.ron
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

mcplay now uses a unified scenario format that supports both simple mcplay scenarios and complex orchestration scenarios:

- **Backward Compatibility**: Existing mcplay scenarios continue to work unchanged
- **Extended Capabilities**: Support for orchestration-style actions like MQTT operations, parallel execution, and custom actions
- **Flexible Client Configuration**: Supports both simple client definitions and extended configuration with spawn positions, inventories, and permissions
- **Rich Action Types**: Includes MCP calls, MQTT publish/expect, client actions, parallel/sequence execution, and custom actions
- **Enhanced Validation**: Comprehensive validation with detailed error messages and warnings
- **Future-Proof Design**: Extensible architecture for adding new action types and configuration options

### Featured Cross-Platform Scenarios

mcplay automatically discovers scenarios in the `../mcplay/scenarios/` directory:

#### **Primary Cross-Platform Scenarios** 🌟
- `alice_desktop_bob_wasm_visual.ron` - **Primary cross-platform test**: Desktop client + WASM browser client with visual interface
- `full_orchestration.ron` - Comprehensive 7-step workflow testing all major features
- `four_player_multiplayer_test.ron` - Advanced multi-client synchronization testing

#### **Development & Testing Scenarios**
- `alice_medieval_world_test.ron` - MCP world creation with medieval templates
- `comprehensive_fast_test.ron` - Quick validation of core functionality  
- `status_indicators_test.ron` - Visual demonstration of mcplay's status system
- `simple_test.ron` - Basic client startup and indefinite running

#### **Browser & WASM Integration** 🌐
- **Automated Browser Launching**: Chrome/Firefox integration with process tracking
- **Visual Testing Instructions**: Rich manual test guidelines displayed in TUI
- **Cross-Platform Validation**: Desktop ↔ WASM client synchronization testing
- **Keep-Alive Mode**: `--keep-alive` flag for extended manual playtesting

See **[../mcplay/README.md](../mcplay/README.md)** for detailed mcplay documentation and **[../mcplay/CHANGELOG_MCPLAY.md](../mcplay/CHANGELOG_MCPLAY.md)** for recent orchestration enhancements.

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

Combine multi-client testing with MCP for advanced scenarios using mcplay orchestration:

```bash
# Cross-platform MCP testing with desktop + WASM clients
cd ../mcplay && cargo run scenarios/alice_desktop_bob_wasm_visual.ron --keep-alive

# Run comprehensive MCP validation across multiple clients
cd ../mcplay && cargo run scenarios/full_orchestration.ron

# Test AI coordination with multiple desktop clients (legacy ctask method)
cargo ctask multi-client --count 2 --full-env -- --mcp
```

#### **mcplay vs ctask for Multi-Client Testing**
- **🌟 mcplay (Recommended)**: Advanced orchestration with cross-platform support, visual monitoring, and scenario scripting
- **⚖️ ctask**: Simpler approach for rapid desktop-only testing without advanced orchestration

### Testing Documentation

- **[docs/MCP_TESTING.md](docs/MCP_TESTING.md)** - MCP-specific testing infrastructure
- **[../mcplay/README.md](../mcplay/README.md)** - Comprehensive mcplay orchestration documentation
- **[MULTI_CLIENT.md](MULTI_CLIENT.md)** - Multi-client testing scenarios (ctask-based)
- **[docs/CROSS_PLATFORM_TESTING.md](docs/CROSS_PLATFORM_TESTING.md)** - Cross-platform desktop + WASM testing workflows
- Built-in test fixtures with comprehensive edge case coverage
- Automated test reporting and validation
- Visual browser-based testing with mcplay orchestration

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

## Controls

IoTCraft supports both keyboard/mouse and gamepad input for full control of your 3D voxel world and IoT devices.

### 🎮 Gamepad Controls

**Movement:**
- **Left Stick**: Move forward/backward/strafe left/right
- **D-Pad**: Alternative movement (works in both analog and digital modes)
- **LT (Left Trigger)**: Run modifier (hold for faster movement)

**Camera:**
- **Right Stick**: Look around/camera rotation
- **Camera stops automatically when stick returns to center**

**Actions:**
- **A (South)**: Place block
- **B (East)**: Remove block
- **X (West)**: Interact with devices
- **Y (North)**: Jump

**System:**
- **LB/Left Bumper**: Previous inventory slot
- **RB/Right Bumper**: Next inventory slot
- **LT (Left Trigger 2)**: Run/Walk toggle
- **Start**: Open menu
- **Select**: Toggle console
- **Left Stick Button**: Toggle minimap

### ⌨️ Keyboard & Mouse Controls

**Movement:**
- **W/A/S/D**: Move forward/left/backward/right
- **Space**: Jump
- **Left Shift**: Run modifier (hold for faster movement)
- **F4**: Toggle between Flying and Walking modes

**Camera:**
- **Mouse**: Look around (requires cursor grab)
- **ESC**: Release cursor/return to menu

**Building & Interaction:**
- **Left Click**: Place block/interact with devices
- **Right Click**: Remove block
- **1-9**: Select inventory hotbar slots
- **G**: Toggle debug overlay

**System:**
- **Tilde (~)**: Toggle console
- **M**: Toggle minimap
- **F3**: Debug information
- **Enter/Escape**: Navigate menus

### 🎯 Controller Features

**Analog Support:**
- **Continuous Movement**: Left stick and D-pad provide smooth, continuous motion while held
- **Variable Speed**: Analog stick pressure controls movement speed
- **Precise Control**: Deadzone filtering prevents accidental movement

**Digital Support:**
- **D-pad Modes**: Works seamlessly in both analog and digital D-pad modes
- **Button Hold**: Continuous action while buttons are held
- **Responsive Input**: Low-latency input processing

**Cross-Platform:**
- **Auto-Detection**: Controllers are automatically detected when connected
- **Hot-Swappable**: Connect/disconnect controllers during gameplay
- **Standard Layout**: Compatible with Xbox, PlayStation, and most PC controllers

### 🔧 Troubleshooting

**If gamepad isn't working:**
1. Ensure controller is properly connected via USB or Bluetooth
2. Check that the game detects the controller (look for connection logs)
3. Try pressing different buttons to verify input detection
4. Restart the application if controller was connected after launch

**Debug Information:**
Enable console logging to see detailed gamepad input information:
```
D-pad state: up=true, down=false, left=false, right=false
Left stick raw values: X=-1.000, Y=0.000
Camera rotation: look_x=0.250, look_y=-0.100
```

## Architecture

### Core Components
- **Desktop Client**: Native Rust application using Bevy engine
- **Web Client (WASM)**: WebAssembly compilation of the same codebase for browsers
- **Build System**: Rust-based `ctask` for web builds (replaces shell scripts)
- **Cross-Platform Orchestration**: mcplay manages both desktop and web clients simultaneously

### Code Sharing Strategy
- **🎯 Maximum Code Reuse**: 95%+ shared code between desktop and web platforms
- **Shared Systems**: World management, inventory, UI, localization, camera controls
- **Platform Abstraction**: Clean separation of networking (native MQTT vs WebSocket)
- **Conditional Compilation**: Strategic use of `#[cfg(target_arch = "wasm32")]` for platform-specific code

### Networking Architecture
- **Desktop**: Native MQTT via `rumqttc` on port 1883
- **Web**: WebSocket MQTT bridge on port 8083 (auto-configured)
- **Protocol Compatibility**: Same message formats ensure cross-platform device communication
- **Unified API**: Abstract MQTT interface allows identical high-level game logic

## Features

### Core Game Features
- 🧊 **3D Voxel World**: Physics-based block building and interaction system
- 📡 **MQTT IoT Integration**: Real-time connectivity with virtual and physical IoT devices
- 🎮 **Cross-Platform Gaming**: Native desktop and WebAssembly web clients with shared worlds
- 👥 **Real-time Multiplayer**: Desktop ↔ Web client synchronization with MQTT messaging

### Advanced Integration
- 🤖 **Model Context Protocol (MCP)**: AI agent interaction for automated world building and device control
- 🎭 **mcplay Orchestration**: Comprehensive multi-client testing and scenario management
- 🌐 **Browser Compatibility**: Full-featured WASM client with automatic browser launching
- 🔄 **Cross-Platform Sync**: Seamless interaction between desktop and web players

### Developer Tools
- 🖥️ **Console System**: Powerful scripting interface with command history
- 🌍 **Internationalization (i18n)**: Multi-language support with runtime switching
- 🗺️ **Asset Management**: Efficient texture, sound, and model loading
- 📊 **Minimap & UI**: Real-time world overview with device status indicators
- 🧪 **Testing Infrastructure**: Automated cross-platform testing with visual validation
