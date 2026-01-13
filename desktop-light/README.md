# IotCraft Desktop Simulator

Desktop version of the ESP32-P4 IotCraft client for rapid development and debugging.

## Building

### Prerequisites

```bash
# macOS
brew install raylib

# Ubuntu/Debian
sudo apt-get install libraylib-dev

# Or build from source
git clone https://github.com/raysan5/raylib.git
cd raylib
mkdir build && cd build
cmake ..
make && sudo make install
```

### Compile

```bash
cd desktop-light
mkdir build && cd build
cmake ..
make -j$(sysctl -n hw.ncpu)
```

## Usage

### Interactive Mode

```bash
./iotcraft-desktop
```

Shows:
1. 5-second chessboard test (Phase 2)
2. 3D medieval world (Phase 3)
3. Auto-rotating camera

### Screenshot Mode (for AI/CI)

```bash
# Run for 5 seconds, save screenshot, exit
./iotcraft-desktop --screenshot output.png

# Custom duration
./iotcraft-desktop --screenshot frame100.png --duration 10

# High-resolution mode
./iotcraft-desktop --screenshot hires.png --size 640x480

# Headless (no window shown)
./iotcraft-desktop --screenshot test.png --headless --duration 3
```

### Verbose Mode

```bash
./iotcraft-desktop --verbose
```

Shows debug overlay:
- Frame counter
- Block count
- Camera position

## Command-Line Options

```
Options:
  --screenshot FILE    Save screenshot and exit
  --duration SECONDS   Run for N seconds (default: 5)
  --headless           Don't show window (for screenshots)
  --size WIDTH HEIGHT  Resolution (default: 320x240)
  --verbose, -v        Enable verbose logging
  --help, -h           Show help message
```

## Examples

### Debug with LLDB

```bash
lldb ./iotcraft-desktop
(lldb) b draw_textured_column
(lldb) run
(lldb) p tex_y
(lldb) p tex_y_step
(lldb) continue
```

### Automated Testing

```bash
# Test multiple resolutions
for size in 320x240 640x480 800x600; do
    ./iotcraft-desktop --screenshot "test_${size}.png" --size $size
done

# Compare screenshots
ls -lh test_*.png
```

### Performance Profiling

```bash
# With valgrind
valgrind --leak-check=full ./iotcraft-desktop --verbose

# With time
time ./iotcraft-desktop --duration 30
```

## Code Structure

```
desktop-light/
├── CMakeLists.txt       # Build configuration
├── main.c              # Entry point
├── cli_options.c/h     # Command-line parsing
├── mocks/              # ESP-IDF mocks
│   ├── esp_log_mock.c/h    # ESP_LOGI/E/W macros
│   └── freertos_mock.c/h   # vTaskDelay, xTaskCreate
└── ../esp32-p4-lcd-ev-devkit-client/main/  # Shared code
    ├── camera.c         # ← Same as ESP32!
    ├── renderer.c       # ← Same as ESP32!
    ├── world.c          # ← Same as ESP32!
    └── ...
```

## Phases


**Desktop Debugging:**

```bash
# Enable verbose mode to see debug overlay
./iotcraft-desktop --verbose

# Take screenshot at specific frame
./iotcraft-desktop --screenshot debug.png --duration 1

# Use lldb to inspect texture stepping
lldb ./iotcraft-desktop
(lldb) b renderer.c:239  # draw_textured_column
(lldb) run
(lldb) p tex_y
(lldb) p tex_y_step
(lldb) p column_height
(lldb) p ty
(lldb) continue
```

**Visual Debugging:**

Edit `renderer.c` to draw debug info:
```c
DrawText(TextFormat("ty: %d", ty), x + 10, y, 10, YELLOW);
```

## Exit Codes

- `0` - Success
- `1` - Error (initialization, rendering, or screenshot failed)

## Integration with CI/CD

```yaml
# GitHub Actions example
- name: Run desktop simulator
  run: |
    cd desktop-light/build
    ./iotcraft-desktop --screenshot output.png --duration 5 --headless

- name: Upload screenshot
  uses: actions/upload-artifact@v3
  with:
    name: screenshot
    path: desktop-light/build/output.png
```

## Notes

- Frame rate: 20 FPS (50ms per frame)
- Default resolution: 320x240 (matches ESP32 internal resolution)
- Screenshot format: PNG (lossless compression)
- Screenshot size: ~80 KB for 320x240 RGBA

## See Also

- ESP32 version: `../esp32-p4-lcd-ev-devkit-client/`
- Desktop client (Rust/Bevy): `../desktop-client/`
- Implementation plan: `../plan-desktop-light.txt`
