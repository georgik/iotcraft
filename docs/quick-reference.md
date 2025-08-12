# IoTCraft Quick Reference

## Getting Started

1. **Start the desktop client**: `cargo run` in `desktop-client/`
2. **Open console**: Press **F12** or **T**
3. **Start building**: Use `place` commands to add blocks

## Essential Commands

### Basic Building
```bash
place grass 0 1 0    # Place a grass block
place dirt 0 0 0     # Place foundation
place stone 0 2 0    # Place wall blocks
wall stone 0 0 0 5 3 0  # Create a wall from (0,0,0) to (5,3,0)
remove 0 1 0         # Remove a block
```

### Console and System
```bash
list                 # List connected devices
```

### World Management
```bash
save_map my_world.json     # Save current world
load_map my_world.json     # Load saved world
load build_script.txt   # Run a building script
```

### Device Control
```bash
spawn test_lamp 3.0 1.0 2.0      # Create test device
move esp32c6_123 5.0 0.5 3.0     # Move device
blink start                      # Start blinking all devices
mqtt status                      # Check MQTT connection
```

## Camera Controls
- **WASD**: Move around
- **Mouse + Left Click**: Look around  
- **E/Q**: Fly up/down
- **Shift**: Move faster
- **Scroll**: Adjust speed

## Minimap
- **M**: Toggle minimap visibility
- **Interactive**: Click on minimap to teleport to that location
- **Real-time**: Shows blocks, devices, and player position
- **Top-down view**: Overhead perspective of the world

## Block Types
- **grass**: Green surface blocks
- **dirt**: Brown foundation blocks  
- **stone**: Gray structural blocks
- **quartz_block**: White decorative blocks
- **glass_pane**: Transparent glass blocks
- **cyan_terracotta**: Blue-green ceramic blocks

## File Formats

### Script Files (`.txt`)
```bash
# Comments start with #
place grass 0 0 0
place dirt 0 -1 0
save_map structure.json
```

### Map Files (`.json`)
```json
{
  "blocks": [
    {
      "x": 0, "y": 0, "z": 0,
      "block_type": "Grass"
    }
  ]
}
```

## Quick Building Tips

1. **Start with foundation**: Use `dirt` blocks or `wall dirt` for large areas
2. **Build walls**: Use `stone` blocks or `wall stone` for faster construction
3. **Use wall command**: Create large structures quickly with `wall` command
4. **Add details**: Use `grass` for decoration
5. **Save frequently**: Use `save_map` often
6. **Use scripts**: Automate repetitive builds

## Common Workflows

### Building a Structure
1. Plan coordinates and size
2. Build foundation with `place dirt` or `wall dirt` for large areas
3. Add walls with `place stone` or `wall stone` for efficiency
4. Add roof and details
5. Save with `save_map`

### Loading and Sharing
1. Save your build: `save_map my_build.json`
2. Share the JSON file
3. Others load with: `load_map my_build.json`

### Script Development
1. Test commands manually in console
2. Create `.txt` file with commands
3. Run with `load script_name.txt`
4. Iterate and improve

## Troubleshooting

- **Console not opening**: Try F12 or T key
- **Blocks not appearing**: Check coordinates are integers
- **File not found**: Ensure file path is correct
- **Performance issues**: Remove unnecessary blocks

## Next Steps

- Read [Console Commands Reference](console-commands.md)
- Learn about [Voxel System](voxel-system.md)  
- Try the example script: `load docs/examples/simple_house.txt`
