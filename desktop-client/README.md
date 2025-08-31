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
- Multiplayer support
- Console system with scripting
- Internationalization (i18n)
- Asset management
- Minimap and UI systems
