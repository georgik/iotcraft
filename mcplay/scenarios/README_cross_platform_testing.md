# Cross-Platform Testing with mcplay

## Alice Desktop + Bob Visual WASM Scenario

This document describes the new cross-platform testing capability in mcplay that allows testing desktop-to-WASM multiplayer interactions.

### Scenario: `alice_desktop_bob_wasm_visual.ron`

**Purpose**: Comprehensive cross-platform testing where Alice runs on desktop client and Bob joins via visual WASM client in Chrome browser.

### New mcplay Features Implemented

#### 0. WASM Client Type Support
```ron
client: (
  id: "bob",
  client_type: "wasm", 
  config: Some({
    "browser": "chrome",
    "url": "http://localhost:8000",
    "readiness_probe": {
      "type": "process_running",
      "initial_delay_seconds": 8,
      "timeout_seconds": 15
    }
  })
)
```
- **WASM Client Management**: Bob appears as managed process in mcplay TUI
- **Browser Integration**: Automatic browser launching with specified browser
- **Process Monitoring**: Simple readiness probe (process doesn't exit with error)
- **Status Tracking**: WASM client shows in left pane with status updates

#### 1. System Command Integration
```ron
action: (
  type: "system_command",
  command: ["cargo", "ctask", "web-build", "--release"],
  working_dir: "../desktop-client",
  background: false,
  timeout_seconds: 300,
)
```
- **Background execution**: Start processes that run in background
- **Timeout handling**: Prevent commands from hanging indefinitely  
- **Working directory**: Execute commands in specific directories
- **Output capture**: Capture and log command output

#### 2. Browser Launching
```ron
action: (
  type: "open_browser", 
  url: "http://localhost:8000",
  browser: "chrome",
  wait_seconds: 8,
)
```
- **macOS Integration**: Uses native `open` command
- **Browser Selection**: Chrome, Safari, Firefox, or system default
- **Wait Control**: Configurable wait time for browser loading

#### 3. Rich Messaging
```ron
action: (
  type: "show_message",
  message: "Multi-line message with instructions...",
  message_type: "info",
)
```
- **Multi-line Support**: Detailed testing instructions
- **Message Types**: Info, warning, error, success with appropriate emojis
- **Formatted Output**: Clean indentation for readability

### Scenario Workflow

#### Phase 1: Alice World Setup
1. **Create Medieval World**: Alice creates world with medieval template
2. **Verify Creation**: Check world status and block count
3. **Build Welcome Beacon**: Create landmark for Bob to find
4. **Publish World**: Make world discoverable for multiplayer

#### Phase 2: WASM Client Preparation  
1. **Build WASM Client**: Compile optimized WASM version (5 min timeout)
2. **Start Web Server**: Serve WASM client on port 8000
3. **Verify Server**: Confirm web server is ready

#### Phase 3: Browser Launch
1. **Open Chrome**: Launch Bob's visual WASM client
2. **Wait for Loading**: Allow browser to fully initialize

#### Phase 4: Manual Testing Coordination
1. **Show Instructions**: Comprehensive testing checklist
2. **Extended Session**: Up to 2 hours of manual testing
3. **Keep-Alive Mode**: All processes remain active

### Usage

```bash
# Run the cross-platform testing scenario
cd mcplay
cargo run scenarios/alice_desktop_bob_wasm_visual.ron

# Validate scenario without running
cargo run scenarios/alice_desktop_bob_wasm_visual.ron --validate
```

### Testing Checklist

When the scenario runs, you'll get comprehensive instructions for:

**✅ Cross-Platform Verification**:
- Block synchronization between desktop and WASM
- Device interactions across platforms  
- World state consistency
- MQTT communication testing

**🎨 Visual Testing**:
- Rendering consistency between desktop and browser
- Camera controls in browser environment
- Medieval template structure visibility

**🔧 Interactive Testing**:
- Alice (desktop) and Bob (browser) simultaneous gameplay
- Real-time multiplayer synchronization
- IoT device spawning and control

### Key Coordinates

- **Castle**: Around (0, 0, -12) - Medieval template main structure
- **Welcome Beacon**: (0, 5, 2) - White quartz tower built by Alice
- **Village Houses**: Various locations around the castle

### Architecture Benefits

1. **Complete Orchestration**: mcplay manages all processes (MQTT, desktop client, web server, browser)
2. **WASM Client Management**: Bob appears as managed client in left pane with status monitoring
3. **Visual Testing**: Real browser environment for authentic WASM testing
4. **Automated Setup**: No manual browser opening or URL typing
5. **Process Status Monitoring**: Both Alice (desktop) and Bob (WASM) show in client list
6. **Rich Logging**: All processes logged to separate files with timestamps
7. **Graceful Cleanup**: Ctrl+C cleanly terminates all spawned processes

### Platform Support

- **Primary**: macOS (using `open` command for browser launching)
- **Extensible**: Can be extended for Linux/Windows with platform-specific commands

### Future Extensions

- **Multiple Browsers**: Test across different browser engines
- **Automated Testing**: Add automated cross-platform validation
- **Performance Metrics**: Measure synchronization latency
- **Mobile Testing**: Extend to mobile WASM testing

This represents a significant advancement in cross-platform multiplayer testing for IoTCraft! 🚀
