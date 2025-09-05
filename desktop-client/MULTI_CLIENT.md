# Multi-Client Testing with ctask

The ctask now includes a powerful `multi-client` command to run multiple IoTCraft client instances simultaneously for multiplayer testing.

## Features

- **Concurrent Clients**: Run multiple desktop and web client instances with automatic player ID assignment
- **Mixed Client Types**: Simultaneously run desktop clients and web browser instances
- **Integrated Infrastructure**: Automatically starts MQTT server, MQTT observer, and web server as needed
- **Browser Auto-detection**: Automatically detects and uses available browsers for web clients
- **Structured Logging**: Each client's output is logged to separate files with timestamps
- **MQTT Server Override**: Test against different MQTT brokers
- **Real-time Monitoring**: Console output from all clients with prefixes
- **Graceful Shutdown**: Ctrl+C terminates all clients cleanly
- **Session Management**: Logs organized by run timestamp for easy tracking

## Usage

### Basic Usage

```bash
# Run 2 desktop clients (default)
cargo ctask multi-client

# Run 3 desktop clients
cargo ctask multi-client --count 3

# Run 2 web clients in browsers
cargo ctask multi-client --web-clients 2

# Run mixed clients: 2 desktop + 3 web
cargo ctask multi-client --count 2 --web-clients 3
```

### Advanced Usage

```bash
# Complete test environment with full infrastructure
cargo ctask multi-client \
  --count 2 \
  --web-clients 2 \
  --full-env

# Custom ports and browser
cargo ctask multi-client \
  --count 1 \
  --web-clients 3 \
  --mqtt-port 1884 \
  --web-port 8080 \
  --browser-cmd firefox

# With MQTT server and observer
cargo ctask multi-client \
  --count 2 \
  --web-clients 1 \
  --with-mqtt-server \
  --with-observer

# External MQTT server with mixed clients
cargo ctask multi-client \
  --count 2 \
  --web-clients 2 \
  --mqtt-server broker.hivemq.com \
  --log-dir test-logs

# Pass additional arguments to desktop clients only
cargo ctask multi-client \
  --count 2 \
  --web-clients 1 \
  -- --script test_script.txt
```

### Infrastructure-Only Setup

```bash
# Start just the infrastructure (useful for manual client testing)
cargo ctask multi-client \
  --count 0 \
  --web-clients 0 \
  --with-mqtt-server \
  --with-observer

# Start web server for manual browser testing
cargo ctask multi-client \
  --count 0 \
  --web-clients 1
# Then manually navigate to http://localhost:8000
```

## Command Options

### Client Configuration
- `--count <COUNT>` or `-c <COUNT>`: Number of desktop client instances (default: 2)
- `--web-clients <COUNT>` or `-w <COUNT>`: Number of web client instances (default: 0)
- `--mqtt-server <SERVER>` or `-m <SERVER>`: Override MQTT server address
- `--log-dir <DIR>` or `-l <DIR>`: Base directory for logs (default: logs)

### Infrastructure Options
- `--full-env`: Start complete test environment (MQTT server + observer)
- `--with-mqtt-server`: Start MQTT server from ../mqtt-server
- `--with-observer`: Add MQTT observer using mosquitto_sub
- `--mqtt-port <PORT>`: MQTT server port (default: 1883)
- `--web-port <PORT>`: Web server port for WASM clients (default: 8000)

### Browser Configuration
- `--browser-cmd <BROWSER>`: Browser command for web clients (e.g., 'chrome', 'firefox')
  - Auto-detects if not specified: Chrome → Chromium → Firefox → Safari
  - By default, opens URLs in existing browser instance (better UX, leverages cache)
- `--clean-browser`: Opens web clients in isolated browser instances (clean state, no cache)
  - Useful for testing without cached data or browser extensions interference

### Client Arguments
- `-- <ARGS>...`: Additional arguments passed to each desktop client (not web clients)

## Log Management

### Log Structure
```
logs/
└── 1640995200/  # Unix timestamp of the run
    ├── mqtt-server.log      # MQTT server logs (if --with-mqtt-server)
    ├── mqtt-observer.log    # MQTT observer logs (if --with-observer)
    ├── web-server.log       # Web server logs (if --web-clients > 0)
    ├── client-1.log         # Desktop client logs
    ├── client-2.log
    ├── web-client-1.log     # Web client browser logs
    └── web-client-2.log
```

### Log Format
Each log file contains:
- Header with client/server info, start time, and command
- Timestamped output with stream type (STDOUT/STDERR)
- Process prefix for easy identification

#### Desktop Client Log Example:
```
=== IoTCraft Client 1 (Player: player-1) ===
Started at: 2024-01-01 12:00:00 UTC
Command: cargo run -- --player-id player-1 --mqtt-server localhost
==========================================

[12:00:01.234] [STDOUT] [Client-1] INFO: Starting IoTCraft client...
[12:00:01.456] [STDOUT] [Client-1] INFO: Connected to MQTT broker at localhost:1883
```

#### Web Client Log Example:
```
=== IoTCraft Web Client 1 (Player: player-3) ===
Started at: 2024-01-01 12:00:00 UTC
Browser: google-chrome
URL: http://localhost:8000?player=player-3
=============================================

[12:00:02.123] [STDOUT] [Web-Client-1] INFO: Browser started with PID 12345
[12:00:02.456] [STDERR] [Web-Client-1] INFO: Opening URL http://localhost:8000?player=player-3
```

#### Infrastructure Log Example:
```
=== Web Server ===
Started at: 2024-01-01 12:00:00 UTC
Port: 8000
Directory: dist
Command: cargo ctask web-serve --port 8000 --dir dist
==================

[12:00:01.789] [STDOUT] [Web-Server] INFO: Server starting on 0.0.0.0:8000...
[12:00:01.890] [STDOUT] [Web-Server] INFO: Serving files from dist/
```

### Viewing Logs
The tool provides `tail -f` commands for real-time log monitoring:

```bash
# View logs from the most recent run
# Desktop clients
tail -f logs/1640995200/client-1.log
tail -f logs/1640995200/client-2.log

# Web clients
tail -f logs/1640995200/web-client-1.log
tail -f logs/1640995200/web-client-2.log

# Infrastructure
tail -f logs/1640995200/mqtt-server.log
tail -f logs/1640995200/mqtt-observer.log
tail -f logs/1640995200/web-server.log

# View all logs simultaneously
tail -f logs/1640995200/*.log
```

## Testing Scenarios

### 1. Basic Multiplayer Sync (Desktop Only)
```bash
# Test basic desktop multiplayer synchronization
cargo ctask multi-client --count 2 --full-env
```

### 2. Cross-Platform Testing (Desktop + Web)
```bash
# Test desktop and web client synchronization
cargo ctask multi-client \
  --count 2 \
  --web-clients 2 \
  --full-env
```

### 3. Web-Only Testing
```bash
# Test multiple web clients
cargo ctask multi-client \
  --web-clients 3 \
  --with-mqtt-server
```

### 4. Load Testing (Mixed Clients)
```bash
# High-load testing with mixed client types
cargo ctask multi-client \
  --count 4 \
  --web-clients 4 \
  --full-env
```

### 5. Browser-Specific Testing
```bash
# Test with specific browser
cargo ctask multi-client \
  --web-clients 2 \
  --browser-cmd firefox \
  --full-env
```

### 6. Cross-Server Testing
```bash
# Test against external MQTT broker
cargo ctask multi-client \
  --count 2 \
  --web-clients 1 \
  --mqtt-server test.mosquitto.org
```

### 7. Infrastructure Development
```bash
# Start infrastructure for manual development
cargo ctask multi-client \
  --count 0 \
  --web-clients 0 \
  --full-env
# Then manually start clients: cargo run -- --player-id test1
# Or open browser to: http://localhost:8000?player=test1
```

### 8. Scripted Desktop Testing
```bash
# Run desktop clients with automated scripts
cargo ctask multi-client \
  --count 2 \
  --web-clients 1 \
  --full-env \
  -- --script scripts/multiplayer_test.txt
```

## Real-time Monitoring

During execution with mixed clients, you'll see:

```
🚀 Starting IoTCraft test environment...
   Desktop client instances: 2
   Web client instances: 2
   Log directory: logs
   ✅ MQTT server: localhost:1883
   ✅ Web server: localhost:8000
   ✅ MQTT observer: mosquitto_sub
   📡 MQTT server: localhost

📁 Session logs will be stored in: logs/1640995200

🟢 Starting MQTT server...
   ✅ MQTT server is ready and listening on port 1883
🟢 Starting MQTT observer...
🟢 Starting web server...
   Building web version for clients...
   ✅ Web server is ready and serving on port 8000

🟢 Starting client 1 (Player ID: player-1)...
   Command: cargo run -- --player-id player-1 --mqtt-server localhost
   Log file: logs/1640995200/client-1.log

🟢 Starting client 2 (Player ID: player-2)...
   Command: cargo run -- --player-id player-2 --mqtt-server localhost
   Log file: logs/1640995200/client-2.log

🟢 Starting web client 1 (Player ID: player-3)...
   Browser: google-chrome
   URL: http://localhost:8000?player=player-3
   Log file: logs/1640995200/web-client-1.log

🟢 Starting web client 2 (Player ID: player-4)...
   Browser: google-chrome
   URL: http://localhost:8000?player=player-4
   Log file: logs/1640995200/web-client-2.log

✅ All components started successfully!
   🌐 MQTT Server: running on port 1883
   👁️  MQTT Observer: monitoring all topics
   🎮 Clients: 4 instances running (2 desktop + 2 web)

💡 Monitoring all processes...
   Press Ctrl+C to stop all components and exit
   Logs are being written to: logs/1640995200

[MQTT-Server] INFO: Server started on port 1883
[MQTT-Observer] INFO: Subscribed to all topics
[Web-Server] INFO: Serving on http://0.0.0.0:8000
[Client-1] INFO: Starting IoTCraft client...
[Client-2] INFO: Starting IoTCraft client...
[Web-Client-1] INFO: Browser started with PID 12345
[Web-Client-2] INFO: Browser started with PID 12346
[Client-1] INFO: Connected to MQTT broker at localhost:1883
[Client-2] INFO: Connected to MQTT broker at localhost:1883
```

## Benefits for Development

1. **Multiplayer Testing**: Easily test client-to-client synchronization
2. **MQTT Debugging**: View MQTT message flow in diagnostic screens (press F3 in-game)
3. **Performance Testing**: Identify bottlenecks with multiple concurrent clients  
4. **Cross-Platform Testing**: Test different MQTT brokers and configurations
5. **Automated Testing**: Integrate with scripts for continuous testing
6. **Issue Reproduction**: Recreate multiplayer bugs consistently
7. **Load Analysis**: Test server capacity and client limits

## Troubleshooting

### Common Issues

1. **Port Conflicts**: 
   - MQTT server port (default 1883) may conflict with existing brokers
   - Web server port (default 8000) may be in use
   - Use `--mqtt-port` and `--web-port` to change defaults

2. **Browser Issues**:
   - Browser not found: Install Chrome, Chromium, or Firefox
   - Browser crashes: Check browser logs and system resources
   - Multiple browser windows: Browsers may reuse existing windows

3. **Web Client Issues**:
   - WASM build failures: Run `cargo xtask web-build` first to check
   - Network connectivity: Web clients need HTTP access to localhost
   - CORS issues: Web server serves all origins, but check browser console

4. **Resource Limits**: 
   - Monitor CPU/memory with many clients
   - Browser instances can be resource-intensive
   - Consider reducing concurrent clients on slower systems

5. **MQTT Limits**: 
   - Some brokers have connection limits
   - External brokers may rate-limit connections
   - Check broker logs for rejected connections

6. **Log Space**: Monitor disk space for long-running tests with many clients

### Debugging Tips

1. **Desktop Clients**:
   - Use F3 debug screen to see MQTT subscription status
   - Check client logs for connection errors
   - Verify MQTT server is running and accessible

2. **Web Clients**:
   - Open browser developer tools (F12) for detailed WASM logs
   - Check network tab for failed requests
   - Verify web server is serving files correctly
   - Check URL parameters are passed correctly (?player=playerN)

3. **Infrastructure**:
   - Check MQTT server logs for client connection patterns
   - Monitor web server logs for HTTP requests
   - Use MQTT observer logs to see message flow between clients

4. **General**:
   - Use different log directories for different test scenarios
   - Check system resources (htop, Activity Monitor)
   - Start with fewer clients and scale up gradually
   - Test individual components before full integration

This multi-client testing capability significantly simplifies the development and debugging of multiplayer features in IoTCraft!
