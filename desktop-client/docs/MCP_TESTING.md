# MCP Testing Guide for IoTCraft

This guide explains how to test the Model Context Protocol (MCP) integration in the IoTCraft desktop client using our comprehensive Rust-based testing infrastructure.

## Overview

The IoTCraft MCP integration allows WARP and other MCP clients to interact with the desktop client through JSON-RPC over TCP. Our testing approach ensures that:

1. JSON requests are correctly parsed and validated
2. Tool parameters are properly converted to console commands
3. Commands are executed correctly in the main engine
4. Error handling works as expected
5. The protocol implementation is robust and reliable

## Testing Infrastructure

### 1. Unit Tests (`src/mcp/tests.rs`)

**Purpose**: Test individual MCP tool functions and command conversion logic

**Coverage**:
- Parameter parsing and validation
- JSON-to-command translation
- Error handling for missing/invalid parameters
- Edge cases (negative coordinates, floating-point truncation, etc.)

**Run with**:
```bash
cargo test mcp
```

**Key Test Categories**:
- `mcp_tests`: Tests individual MCP tool execution functions
- `command_conversion_tests`: Tests the conversion from JSON parameters to console commands

### 2. Integration Tests (`tests/mcp_integration_tests.rs`)

**Purpose**: Test the full MCP server communication flow

**Coverage**:
- TCP connection establishment
- JSON-RPC protocol compliance
- End-to-end tool execution
- Error response handling
- Multiple concurrent requests

**Run with**:
```bash
cargo test --test mcp_integration_tests
```

**Requirements**: These tests require the MCP server to be running (they skip if unavailable)

### 3. CLI Test Client (`src/bin/mcp_test_client.rs`)

**Purpose**: Comprehensive testing tool for manual and automated MCP testing

**Features**:
- Individual tool testing
- Interactive testing mode
- Comprehensive test suite with reporting
- JSON and text output formats
- Cross-platform compatibility (pure Rust)

**Usage Examples**:
```bash
# Build the test client
cargo build --bin mcp_test_client

# Run comprehensive test suite
cargo run --bin mcp_test_client -- run-tests

# Test specific tools
cargo run --bin mcp_test_client -- test-wall stone 0 0 0 3 2 1
cargo run --bin mcp_test_client -- test-place grass 5 1 5

# Interactive mode
cargo run --bin mcp_test_client -- interactive

# Generate JSON report
cargo run --bin mcp_test_client -- run-tests --format json --output report.json
```

### 4. Test Fixtures (`tests/fixtures/mcp_test_data.json`)

**Purpose**: Comprehensive test data covering all scenarios

**Contents**:
- Valid request/response examples for all tools
- Error case scenarios (missing parameters, invalid types)
- Edge cases (extreme coordinates, empty values)
- Stress test cases (large walls, negative coordinates)

## Common Issues and Solutions

### 1. Wall Command Issues

**Problem**: The `create_wall` command may have issues with JSON parameter conversion.

**What we test**:
- Parameter order: `x1, y1, z1, x2, y2, z2`
- Coordinate validation (integers required)
- Volume calculation accuracy
- Negative coordinate handling

**Expected conversion**:
```json
{
  "block_type": "stone",
  "x1": 0, "y1": 0, "z1": 0,
  "x2": 3, "y2": 2, "z2": 1
}
```
↓
```
wall stone 0 0 0 3 2 1
```

### 2. Parameter Type Issues

**Problem**: JSON numbers vs. expected Rust types

**Solutions**:
- Blocks use integer coordinates (`as_i64()`)
- Devices use floating-point coordinates (`as_f64()`)
- Proper error handling for type mismatches

### 3. Missing Parameter Handling

**Problem**: MCP tools should gracefully handle missing required parameters

**What we test**:
- Required parameters return appropriate errors
- Optional parameters use sensible defaults
- Error messages are descriptive

## Running the Full Test Suite

### 1. Unit Tests Only
```bash
cargo test mcp
```

### 2. Integration Tests (requires MCP server)
```bash
# Terminal 1: Start desktop client with MCP support
cargo run -- --mcp

# Terminal 2: Run integration tests
cargo test --test mcp_integration_tests
```

### 3. Comprehensive Testing with CLI Tool
```bash
# Terminal 1: Start desktop client with MCP support
cargo run -- --mcp

# Terminal 2: Run comprehensive test suite
cargo run --bin mcp_test_client -- run-tests
```

### 4. Manual Testing
```bash
# Terminal 1: Start desktop client with MCP support
cargo run -- --mcp

# Terminal 2: Interactive testing
cargo run --bin mcp_test_client -- interactive
# Then use commands like:
# wall stone 0 0 0 2 1 1
# place grass 5 1 5
# tools
```

## Test Report Example

The CLI test client generates detailed reports:

```
🧪 MCP Test Report
==================
Server: 127.0.0.1:8080
Total tests: 8
✅ Passed: 7
❌ Failed: 1
Success rate: 87.5%

📋 Test Details
✅ server_connection (45 ms)
✅ initialize_protocol (67 ms)
✅ list_tools (52 ms)
✅ create_wall (156 ms)
✅ place_block (89 ms)
✅ spawn_device (94 ms)
❌ error_handling_missing_params (23 ms)
   Error: Did not handle missing parameters gracefully
✅ command_conversion (234 ms)
```

## Continuous Integration

All tests are designed to work in CI environments:

- Unit tests require no external dependencies
- Integration tests gracefully skip when server unavailable
- CLI test client provides machine-readable JSON output
- Cross-platform compatibility (Windows, macOS, Linux)

## Adding New Tests

### For New MCP Tools

1. Add unit tests in `src/mcp/tests.rs`:
   - Test parameter parsing
   - Test command conversion
   - Test error cases

2. Add integration test scenarios in `tests/mcp_integration_tests.rs`

3. Add test fixtures in `tests/fixtures/mcp_test_data.json`

### For New Features

1. Extend the CLI test client with new commands
2. Add corresponding test data fixtures
3. Update the comprehensive test suite

## Performance Considerations

- Unit tests run in milliseconds
- Integration tests include reasonable timeouts (10 seconds per request)
- CLI test client includes timing measurements
- Large wall creation tests include volume validation

## Troubleshooting

### Server Connection Issues
- Ensure desktop client is running with `--mcp` flag
- Check port 8080 availability
- Verify firewall settings

### Test Failures
- Check MCP server logs for detailed error messages
- Use individual tool tests to isolate issues
- Verify parameter formats match expected JSON schema

### Command Translation Issues
- Check the `convert_tool_call_to_command` function
- Verify parameter extraction logic
- Test with edge cases (negative numbers, defaults)

## Recent Improvements

### Enhanced MQTT Server Readiness

Recent improvements to the testing infrastructure include:

- **Asynchronous port detection**: Tests now use non-blocking async port checks for MQTT server readiness
- **Improved reliability**: 1-second timeout with 500ms polling intervals prevents test flakiness
- **Better error handling**: Detailed debug logs help troubleshoot connection issues
- **WASM compatibility**: Fixed async/await usage in browser-based WASM tests

### Multi-Client MCP Testing

Advanced testing scenarios now supported:

```bash
# Test multiple AI agents coordinating across clients
cargo xtask multi-client --count 2 --full-env -- --mcp

# Each client can run MCP servers on different ports
# Enables testing AI coordination and world synchronization
```

### Test Infrastructure Enhancements

- **Automatic server lifecycle**: Tests automatically start/stop required infrastructure
- **Port isolation**: Each test uses different ports to avoid conflicts
- **Comprehensive fixtures**: Edge cases and error conditions thoroughly covered
- **Cross-platform testing**: Works reliably on Windows, macOS, and Linux

## Integration with Multi-Client Testing

The MCP testing infrastructure integrates seamlessly with multi-client testing:

```bash
# Scenario: Two AI agents collaborate on building
# Terminal 1: Start infrastructure
cargo xtask multi-client --count 0 --web-clients 0 --full-env

# Terminal 2: Start first client with MCP
cargo run -- --player-id ai-agent-1 --mcp --mqtt-server localhost

# Terminal 3: Start second client with MCP on different port
cargo run -- --player-id ai-agent-2 --mcp --mcp-port 3002 --mqtt-server localhost

# Both clients can now be controlled by different AI agents
# Test coordination through shared MQTT world state
```

## Related Documentation

- **[docs/MCP_INTEGRATION.md](docs/MCP_INTEGRATION.md)** - Comprehensive setup and technical documentation
- **[MULTI_CLIENT.md](MULTI_CLIENT.md)** - Multi-client testing infrastructure
- **[README.md](README.md)** - Main project documentation with testing overview

This comprehensive testing infrastructure ensures the MCP integration is reliable, maintainable, and works correctly across different platforms and scenarios.
