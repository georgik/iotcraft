# McPlay - IoTCraft Multi-client Scenario Player

McPlay is a dedicated testing and orchestration tool for IoTCraft that has been extracted from the main desktop client to improve build efficiency and architectural clarity.

## Features

- **Scenario-based testing**: Run multi-client IoTCraft scenarios like a screenplay
- **Interactive TUI**: Terminal user interface for scenario selection and live log monitoring
- **MCP Integration**: Support for Model Context Protocol communication with clients
- **Infrastructure orchestration**: Automatic MQTT server and observer management
- **Real-time monitoring**: Live system information and log aggregation

## Architecture Benefits

This project was extracted from the main `desktop-client` as a sibling project to:

1. **Eliminate dependency conflicts**: Prevents ring/rustls rebuilds when switching between binaries
2. **Improve compilation times**: Each project has its own target directory and dependency compilation
3. **Better separation of concerns**: Testing/orchestration tools separate from the core client
4. **Independent versioning**: McPlay can evolve independently from the desktop client

## Usage

```bash
# List available scenarios
cargo run -- --list-scenarios

# Run a specific scenario
cargo run -- path/to/scenario.json

# Run with TUI (default)
cargo run

# Validate a scenario without running
cargo run -- --validate path/to/scenario.json
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

## Available Scenarios

The `scenarios/` directory contains ready-to-use test scenarios:

- **simple_test.ron** - Basic MQTT server and desktop client setup for manual testing
- **environment_setup.ron** - Complete IoTCraft environment with structures and IoT devices
- **test_create_world.ron** - Tests MCP create_world command and game state transitions
- **test_set_game_state.ron** - Tests set_game_state MCP tool for UI state transitions
- **two_player_world_sharing.ron** - Multi-player scenario with world sharing and block placement

## Scenario Format

McPlay supports both JSON and RON scenario formats with comprehensive action types for client coordination and testing. Scenarios define infrastructure requirements, client configurations, and step-by-step test procedures.
