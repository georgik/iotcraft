# Desktop Client CTask

This is the desktop-client specific ctask (formerly xtask), providing automation tools specifically for the desktop client component.

## Usage

```bash
# Using the convenient alias (recommended)
cargo ctask <command>

# Or directly
cargo run --package ctask -- <command>
```

## Available Commands

- `web-build` - Build the web version of the application
- `web-serve` - Serve the web version locally
- `web-dev` - Build and serve the web version
- `format-html` - Format HTML files
- `multi-client` - Run multiple client instances for testing
- `test` - Run tests with proper infrastructure

## Note

This was renamed from `xtask` to `ctask` to avoid naming conflicts with the workspace-level `xtask`. The workspace-level `xtask` is now the primary automation tool for the entire IoTCraft workspace, while this `ctask` focuses specifically on desktop-client operations.

For workspace-wide operations like formatting all members, use:
```bash
cargo xtask format       # Format all workspace members
cargo fmt-all           # Alias for the above
```
