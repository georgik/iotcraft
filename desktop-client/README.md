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
- `cargo xtask test unit` - Run unit tests only
- `cargo xtask test integration` - Run integration tests only
- `cargo xtask test mqtt` - Run MQTT-specific tests only
- `cargo xtask test all` - Run all tests (unit, integration, and MQTT)
- `cargo xtask test unit` - Run unit tests only
- `cargo xtask test integration` - Run integration tests only
- `cargo xtask test mqtt` - Run MQTT-specific tests only
- `cargo xtask test all` - Run all tests (unit, integration, and MQTT)

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

To test multiplayer functionality with multiple clients:

#### Option 1: Complete Environment with xtask
```bash
# Terminal 1: Start the complete environment (MQTT server + first client)
cargo xtask env start

# Terminal 2: Run second client connecting to the same MQTT server
cargo run -- --player-id player2

# Terminal 3: Run third client with different player name
cargo run -- --player-id player3
```

#### Option 2: Manual Setup
```bash
# Terminal 1: Start MQTT server
cd ../mqtt-server
cargo run

# Terminal 2: First client (default player)
cargo run

# Terminal 3: Second client with custom player ID
cargo run -- --player-id alice

# Terminal 4: Third client with custom player ID
cargo run -- --player-id bob
```

#### Client Configuration
- Each client needs a unique `--player-id` to avoid conflicts
- Clients automatically discover each other through MQTT
- Shared worlds are synchronized in real-time
- Use the in-game console to create/join multiplayer worlds

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

To test multiplayer functionality with multiple clients:

#### Option 1: Complete Environment with xtask
```bash
# Terminal 1: Start the complete environment (MQTT server + first client)
cargo xtask env start

# Terminal 2: Run second client connecting to the same MQTT server
cargo run -- --player-id player2

# Terminal 3: Run third client with different player name
cargo run -- --player-id player3
```

#### Option 2: Manual Setup
```bash
# Terminal 1: Start MQTT server
cd ../mqtt-server
cargo run

# Terminal 2: First client (default player)
cargo run

# Terminal 3: Second client with custom player ID
cargo run -- --player-id alice

# Terminal 4: Third client with custom player ID
cargo run -- --player-id bob
```

#### Client Configuration
- Each client needs a unique `--player-id` to avoid conflicts
- Clients automatically discover each other through MQTT
- Shared worlds are synchronized in real-time
- Use the in-game console to create/join multiplayer worlds

## Prerequisites

### Desktop
- Rust toolchain with Swift toolchain configured (on macOS)
- System dependencies for Bevy (graphics drivers, etc.)

### Web
- `wasm-pack` (automatically installed by xtask if missing)
- Python 3 (for local development server)

### Development & Testing
- MQTT server (iotcraft-mqtt-server) - automatically managed by xtask
- Multiple terminal sessions for multi-client testing

### Development & Testing
- MQTT server (iotcraft-mqtt-server) - automatically managed by xtask
- Multiple terminal sessions for multi-client testing

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
- Multiplayer support
- Console system with scripting
- Internationalization (i18n)
- Asset management
- Minimap and UI systems
