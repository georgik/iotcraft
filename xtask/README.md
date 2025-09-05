# IoTCraft Workspace Xtask

This is the workspace-level xtask for the IoTCraft project, providing automation tools for the entire workspace.

## Features

### Code Formatting

The xtask provides comprehensive formatting capabilities for all workspace members:

#### Format all workspace members
```bash
# Using the convenient alias (recommended)
cargo fmt-all

# Or directly via xtask
cargo run --package xtask -- format
```

#### Check formatting without modifying files
```bash
# Using the convenient alias (recommended) 
cargo fmt-check

# Or directly via xtask
cargo run --package xtask -- format --check
```

## How it works

The xtask:

1. **Reads workspace members** from the root `Cargo.toml`
2. **Iterates over each member** that exists and has a `Cargo.toml`
3. **Runs `cargo fmt --all -- --color always`** in each member directory
4. **Reports progress** and provides a summary

## Workspace Members

Currently processes these workspace members:
- `desktop-client` - Main 3D visualizer application
- `desktop-client/ctask` - Desktop client's specific ctask (formerly xtask)
- `iotcraft-mcp-protocol` - Shared MCP protocol definitions
- `mqtt-server` - MQTT broker
- `mqtt-client` - MQTT command-line client
- `mcplay` - Orchestration and testing system
- `xtask` - This workspace-level xtask

## Exit Codes

- **0**: Success - all members processed successfully
- **1**: Formatting issues found (when using `--check`) or formatting failed

## Integration with CI/CD

The `--check` option is perfect for CI/CD pipelines:

```bash
# In your CI pipeline
cargo fmt-check
```

This will:
- ✅ Exit with code 0 if all code is properly formatted
- ❌ Exit with code 1 and show detailed diff if formatting issues are found

## Rust-First Philosophy

This tool follows the project's Rust-first approach:
- ✅ Pure Rust implementation using native Cargo tooling
- ✅ Cross-platform compatibility (Windows, macOS, Linux)
- ✅ Type-safe with compile-time error checking
- ✅ Integrated with Cargo's dependency and workspace system
- ❌ No shell scripts or external scripting dependencies

## Development

To extend this xtask with additional commands:

1. Add new commands to the `Commands` enum in `src/main.rs`
2. Implement the command logic following the existing patterns
3. Update this README with usage examples

The xtask is designed to be simple, reliable, and maintainable, following Rust best practices throughout.
