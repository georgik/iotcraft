# mcplay - IoTCraft Multi-Client Orchestration Platform

mcplay is a comprehensive orchestration and testing platform for IoTCraft that enables sophisticated multi-client scenarios including cross-platform desktop + WASM browser testing.

## Features

### 🎭 **Advanced Orchestration**
- **Multi-Client Management**: Coordinate desktop and WASM browser clients simultaneously
- **Infrastructure Automation**: Automatic MQTT server, web server, and browser management
- **Cross-Platform Testing**: Desktop ↔ WASM browser multiplayer synchronization testing
- **Health Monitoring**: Comprehensive readiness and liveness probes for all services
- **Process Lifecycle**: Complete process management with graceful cleanup

### 🎨 **Visual Management**
- **Real-Time TUI**: Kubernetes-style status indicators with emoji-coded service health
- **Multi-Pane Logging**: Separate log streams for orchestrator, MQTT, and each client
- **Interactive Search**: Modal search dialog with live filtering and text highlighting
- **Interactive MCP Interface**: Send MCP commands directly from TUI
- **System Monitoring**: Real-time CPU, memory, and process information
- **Status Tracking**: Visual process states (Starting → Ready → Running → Stopped)
- **Enhanced Navigation**: Improved layout with broader progress indicators

### 🌐 **WASM Client Support** 
- **Browser Integration**: Automatic Chrome/Safari/Firefox launching
- **WASM Client Type**: Define Bob as managed WASM client alongside Alice (desktop)
- **Process Monitoring**: Browser process health monitoring and status updates
- **Cross-Platform Sync**: Test real-time synchronization between desktop and browser

### 🔧 **System Integration**
- **System Commands**: Execute shell commands with background/foreground modes
- **Browser Launching**: Native macOS `open` command integration with browser selection
- **Rich Messaging**: Multi-line formatted messages with emoji indicators
- **Keep-Alive Mode**: Extended manual testing sessions with `--keep-alive` flag

## Architecture Benefits

This project was extracted from the main `desktop-client` as a sibling project to:

1. **Eliminate dependency conflicts**: Prevents ring/rustls rebuilds when switching between binaries
2. **Improve compilation times**: Each project has its own target directory and dependency compilation
3. **Better separation of concerns**: Testing/orchestration tools separate from the core client
4. **Independent versioning**: McPlay can evolve independently from the desktop client

## TUI Interface Guide

### 📱 **Interactive Scenario Browser**
When you run `cargo run` without arguments, mcplay opens an interactive TUI for browsing and running scenarios.

#### **Navigation Controls:**
- `↑`/`↓` - Navigate scenario list
- `Enter` - Run selected scenario
- `d` - View detailed scenario information
- `v` - Validate scenario without running
- `r` - Refresh scenario list
- `q`/`Esc`/`Ctrl+C` - Quit application

#### **🔍 Advanced Search Features:**
- `/` - Open modal search dialog
- **Live filtering** - Results update as you type
- **Text highlighting** - Search matches highlighted in yellow
- **Smart previews** - Shows context from scenario descriptions
- **Results counter** - "Found: X of Y scenarios" display
- `Enter` - Apply filter and close search
- `Esc` - Cancel search and restore full list
- `Backspace` - Delete characters from search query

#### **📊 Enhanced Progress Display:**
- **Improved layout** - 35% left panel, 65% right panel for better proportions
- **Optimized progress bars** - Fit comfortably in left panel
- **Real-time updates** - Live system monitoring and step progress
- **Service status indicators** - Kubernetes-style emoji status (⏳🟡🟢🔴🔵🟠)

#### **🎮 During Scenario Execution:**
- `Tab`/`Shift+Tab` - Switch between log panes
- `↑`/`↓` - Scroll through logs
- `Enter` - Send MCP commands (when client selected)
- `q`/`Esc`/`Ctrl+C` - Exit scenario

## Quick Start

### Cross-Platform Testing (Recommended)
```bash
# Interactive scenario selector with enhanced TUI
# Features: Modal search (/), text highlighting, improved layout, Ctrl+C support
cargo run

# Run Alice (desktop) + Bob (WASM browser) cross-platform test
cargo run scenarios/alice_desktop_bob_wasm_visual.ron

# Extended manual testing (keeps all processes running)
cargo run scenarios/alice_desktop_bob_wasm_visual.ron --keep-alive
```

### CLI Options
```bash
# List all available scenarios
cargo run -- --list-scenarios

# Validate scenario file without execution
cargo run -- --validate <scenario.ron>

# Run with verbose logging
cargo run -- --verbose <scenario.ron>

# Override MQTT port
cargo run -- --mqtt-port 1884 <scenario.ron>

# Extended manual testing mode
cargo run -- --keep-alive <scenario.ron>

# View help
cargo run -- --help
```

### Scenario Types
```bash
# Cross-platform testing (PRIMARY USE CASE)
cargo run scenarios/alice_desktop_bob_wasm_visual.ron

# Four-player desktop multiplayer
cargo run scenarios/four_player_multiplayer_test.ron

# Medieval world creation testing
cargo run scenarios/alice_medieval_world_test.ron

# MCP integration testing
cargo run scenarios/test_mcp_ping.ron
```

## Building

```bash
# Build with TUI support (default)
cargo build

# Build without TUI
cargo build --no-default-features

# Run tests
cargo test
```

## Dependencies

- **tokio**: Async runtime
- **clap**: Command line parsing
- **serde/serde_json**: Serialization
- **ron**: RON format support
- **ratatui** (optional): TUI interface
- **crossterm** (optional): Terminal control
- **chrono** (optional): Time handling for TUI

## Featured Scenarios

### 🌐 **Cross-Platform Testing**
- **`alice_desktop_bob_wasm_visual.ron`** - 🎆 PRIMARY: Alice (desktop) + Bob (WASM browser) testing
  - Automated WASM build and web server setup
  - Browser launching with Chrome integration
  - Medieval world creation with comprehensive testing guide
  - Extended manual testing mode with `--keep-alive`

### 🚀 **Multi-Client Scenarios**
- **`four_player_multiplayer_test.ron`** - Four desktop clients with multiplayer sync testing
- **`alice_medieval_world_test.ron`** - World template system validation with 4,455 blocks
- **`two_player_world_sharing.ron`** - Alice creates, Bob joins with block placement sync

### 🤖 **MCP Integration Testing**
- **`test_mcp_ping.ron`** - Basic MCP connectivity verification
- **`test_create_world.ron`** - MCP world creation and game state transitions
- **`test_get_game_state.ron`** - MCP game state retrieval testing

### 🔧 **Infrastructure Testing**
- **`full_orchestration.ron`** - Complete infrastructure with health monitoring
- **`status_indicators_test.ron`** - Service status transition demonstration
- **`simple_test.ron`** - Basic MQTT server and client setup

**Total Available:** 50+ scenarios in `scenarios/` directory
Use `cargo run -- --list-scenarios` to see all available scenarios.

## Scenario Format

mcplay uses **RON (Rust Object Notation)** format for scenario definitions with comprehensive action types and client configurations.

### Client Types
```ron
clients: [
    // Desktop client
    (
        id: "alice",
        client_type: "desktop",
        mcp_port: 3001,
        // ... desktop config
    ),
    // WASM browser client  
    (
        id: "bob",
        client_type: "wasm",
        mcp_port: 0, // No MCP for WASM
        config: Some({
            "browser": "chrome",
            "url": "http://localhost:8000",
            "readiness_probe": {
                "type": "process_running",
                "initial_delay_seconds": 8
            }
        })
    )
]
```

### Action Types
**MCP Commands:**
```ron
action: (
    type: "mcp_call",
    tool: "create_world",
    arguments: { "world_name": "TestWorld", "template": "medieval" }
)
```

**System Integration:**
```ron
// Execute shell commands
action: (
    type: "system_command",
    command: ["cargo", "ctask", "web-build", "--release"],
    working_dir: "../desktop-client",
    background: false,
    timeout_seconds: 300
)

// Launch browser
action: (
    type: "open_browser",
    url: "http://localhost:8000",
    browser: "chrome",
    wait_seconds: 5
)

// Show formatted messages
action: (
    type: "show_message",
    message: "Multi-line instructions\nwith detailed steps",
    message_type: "info"
)
```

**Timing Control:**
```ron
action: (
    type: "wait_condition",
    condition: "manual_exit",
    timeout: 7200000  // 2 hours
)
```

### Health Monitoring
**Readiness Probes:**
- `tcp_port` - Check if TCP port is accepting connections
- `process_running` - Verify process started without error

**Liveness Probes:**
- `mcp_ping` - Send MCP ping requests
- `process_check` - Monitor process health

### Infrastructure
```ron
infrastructure: (
    mqtt_server: ( required: true, port: 1883 ),
    mqtt_observer: Some(( required: true, topics: Some(["#"]) ))
)
```

**For detailed format specification:** See [WARP.md](WARP.md) for complete RON scenario examples.
