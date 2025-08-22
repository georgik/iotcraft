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

## Prerequisites

### Desktop
- Rust toolchain with Swift toolchain configured (on macOS)
- System dependencies for Bevy (graphics drivers, etc.)

### Web
- `wasm-pack` (automatically installed by xtask if missing)
- Python 3 (for local development server)

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
