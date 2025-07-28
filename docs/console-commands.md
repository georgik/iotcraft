# Console Commands Reference

IoTCraft Desktop Client provides a powerful console interface for interacting with the voxel world, managing devices, and controlling the MQTT system. Press **F12** to open/close the console, or **T** to open it quickly.

## Voxel World Commands

### Block Manipulation

#### `place <block_type> <x> <y> <z>`
Places a block at the specified coordinates.

**Parameters:**
- `block_type`: Type of block to place (`grass`, `dirt`, `stone`)
- `x`, `y`, `z`: Integer coordinates where to place the block

**Examples:**
```
place grass 5 1 0
place dirt 0 0 0  
place stone 10 5 -3
```

#### `remove <x> <y> <z>`
Removes a block at the specified coordinates.

**Parameters:**
- `x`, `y`, `z`: Integer coordinates of the block to remove

**Examples:**
```
remove 5 1 0
remove 0 0 0
```

### Map Management

#### `save_map <filename>`
Saves the current voxel world to a JSON file.

**Parameters:**
- `filename`: Name of the file to save to (typically with `.json` extension)

**Examples:**
```
save_map my_world.json
save_map castle_build.json
save_map backup.json
```

**File Format:**
The saved file contains a JSON structure with all blocks:
```json
{
  "blocks": [
    {
      "x": 5,
      "y": 1,
      "z": 0,
      "block_type": "Grass"
    },
    {
      "x": 6,
      "y": 0,
      "z": 0,
      "block_type": "Dirt"
    }
  ]
}
```

#### `load_map <filename>`
Loads a voxel world from a JSON file, replacing the current world.

**Parameters:**
- `filename`: Name of the file to load from

**Examples:**
```
load_map my_world.json
load_map castle_build.json
```

**Note:** Loading a map will remove all existing voxel blocks and replace them with the blocks from the file.

## Device Management Commands

### Device Control

#### `spawn <device_id> <x> <y> <z>`
Manually spawns a device for testing purposes.

**Parameters:**
- `device_id`: Unique identifier for the device
- `x`, `y`, `z`: Float coordinates where to spawn the device

**Examples:**
```
spawn test_lamp 3.0 1.0 2.0
spawn debug_device 0.0 0.5 0.0
```

#### `move <device_id> <x> <y> <z>`
Moves an existing device to new coordinates.

**Parameters:**
- `device_id`: ID of the device to move
- `x`, `y`, `z`: Float coordinates for the new position

**Examples:**
```
move esp32c6_aabbcc112233 5.0 0.5 3.0
move test_lamp 0.0 1.0 0.0
```

#### `blink <action>`
Controls the blinking state of all registered devices.

**Parameters:**
- `action`: Either `start` or `stop`

**Examples:**
```
blink start
blink stop
```

## MQTT System Commands

#### `mqtt <action>`
Provides MQTT system information and diagnostics.

**Parameters:**
- `action`: Available actions:
  - `status`: Show MQTT connection status
  - `temp`: Display current temperature reading

**Examples:**
```
mqtt status
mqtt temp
```

## Script Management Commands

#### `load <filename>`
Loads and executes a script file containing multiple commands.

**Parameters:**
- `filename`: Path to the script file

**Examples:**
```
load build_castle.script
load setup_world.script
```

**Script File Format:**
Script files contain one command per line. Lines starting with `#` are comments.

```bash
# This is a comment
place grass 0 0 0
place grass 1 0 0
place dirt 0 -1 0
save_map simple_structure.json
```

## Camera Controls

While not console commands, these keyboard controls are essential for navigation:

- **WASD**: Move camera (forward/backward/left/right)
- **E/Q**: Move camera up/down
- **Mouse**: Look around (hold Left Mouse Button to grab cursor)
- **M**: Toggle cursor grab
- **Shift**: Move faster
- **Mouse Scroll**: Adjust movement speed

## Console Interface

- **F12**: Toggle console open/closed
- **T**: Open console quickly
- **Escape**: Close console
- **↑/↓ Arrow Keys**: Navigate command history
- **Tab**: Auto-complete commands (if available)

## Tips and Best Practices

### Building Structures
1. Start with a foundation using `place dirt` commands
2. Build walls using `place stone` or `place grass`
3. Save your work frequently with `save_map`
4. Use scripts for repetitive building tasks

### Managing Large Worlds
1. Use descriptive filenames for saved maps
2. Create backup saves before major changes
3. Organize builds into separate map files
4. Use the scripting system for complex structures

### Device Testing
1. Use `spawn` to create test devices for development
2. Use `blink start` to verify device connectivity
3. Use `move` to test device positioning
4. Monitor MQTT status with `mqtt status`

### Performance Considerations
- Very large worlds (thousands of blocks) may impact performance
- Consider breaking large builds into multiple map files
- The voxel system is optimized for moderate-sized structures
- Use `remove` commands to clean up unnecessary blocks

## Error Handling

The console provides clear error messages for common issues:

- **Invalid coordinates**: Check that x, y, z values are valid integers/floats
- **File not found**: Ensure the filename exists and path is correct
- **Unknown block type**: Use only `grass`, `dirt`, or `stone`
- **Device not found**: Verify device ID is correct for move commands
- **MQTT errors**: Check MQTT broker connection status

## Integration with Physical Devices

Console commands work seamlessly with physical ESP32 devices:

1. **Device Spawning**: Physical devices auto-spawn when they connect
2. **Position Updates**: `move` commands send MQTT messages to physical devices
3. **State Control**: `blink` commands control physical LEDs
4. **Real-time Sync**: Changes in the 3D world reflect on physical devices

This console system provides a powerful interface for both development and interactive use of the IoTCraft platform.
