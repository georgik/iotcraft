# Voxel System Documentation

IoTCraft features a Luanti-like voxel system that allows building and manipulating 3D worlds using individual cube blocks. The system supports multiple block types, world persistence, and seamless integration with IoT devices.

## Overview

The voxel system replaces the traditional flat plane with individual 1x1x1 meter cubes that can be placed, removed, and saved to create persistent 3D worlds. Each block has a specific type and properties, and the entire world can be saved to and loaded from JSON files.

## Block Types

### Current Block Types

#### Grass Blocks
- **Texture**: `textures/grass.png`
- **Use Case**: Surface terrain, decorative elements
- **Appearance**: Green textured cubes representing grass-covered earth

#### Dirt Blocks  
- **Texture**: `textures/dirt.png`
- **Use Case**: Foundation blocks, underground structures
- **Appearance**: Brown textured cubes representing soil

#### Stone Blocks
- **Texture**: `textures/stone.png` 
- **Use Case**: Structural elements, walls, towers
- **Appearance**: Gray textured cubes representing stone material

### Adding New Block Types

To add new block types:

1. **Add to BlockType enum** in `src/environment/environment_types.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockType {
    Grass,
    Dirt,
    Stone,
    Wood,  // New block type
}
```

2. **Add texture mapping** in placement functions:
```rust
let texture_path = match block_type {
    BlockType::Grass => "textures/grass.png",
    BlockType::Dirt => "textures/dirt.png", 
    BlockType::Stone => "textures/stone.png",
    BlockType::Wood => "textures/wood.png", // New texture
};
```

3. **Add to console command parsing**:
```rust
let block_type = match block_type_str {
    "grass" => BlockType::Grass,
    "dirt" => BlockType::Dirt,
    "stone" => BlockType::Stone,
    "wood" => BlockType::Wood, // New command
    // ...
};
```

## World Coordinate System

### Coordinate Space
- **X-axis**: East (positive) / West (negative)
- **Y-axis**: Up (positive) / Down (negative)  
- **Z-axis**: North (positive) / South (negative)
- **Block Size**: Each block occupies exactly 1x1x1 units
- **Grid Alignment**: All blocks are placed on integer coordinates

### Default World
The system starts with a 21x21 flat grass terrain:
- **Size**: 21 blocks × 21 blocks (coordinates -10 to +10 on X and Z axes)
- **Height**: Y=0 (ground level)
- **Block Type**: All grass blocks
- **Total Blocks**: 441 blocks initially

## Architecture

### Core Components

#### VoxelWorld Resource
```rust
pub struct VoxelWorld {
    pub blocks: HashMap<IVec3, BlockType>,
    pub chunk_size: i32,
}
```
- **blocks**: Maps 3D coordinates to block types
- **chunk_size**: Reserved for future chunk-based loading optimization

#### VoxelBlock Component
```rust
pub struct VoxelBlock {
    pub block_type: BlockType,
    pub position: IVec3,
}
```
- Attached to each block entity in the ECS
- Stores block type and position for quick lookups

### Key Methods

#### Block Management
```rust
// Add a block
voxel_world.set_block(IVec3::new(x, y, z), BlockType::Grass);

// Remove a block  
let removed_type = voxel_world.remove_block(&IVec3::new(x, y, z));

// Check block at position
let block_type = voxel_world.get_block(&IVec3::new(x, y, z));
```

#### Persistence
```rust
// Save world to file
voxel_world.save_to_file("my_world.json")?;

// Load world from file
voxel_world.load_from_file("my_world.json")?;
```

## File Format

### JSON Structure
Voxel worlds are saved in a human-readable JSON format:

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
    },
    {
      "x": 7,
      "y": 2,
      "z": 1,
      "block_type": "Stone"
    }
  ]
}
```

### File Properties
- **Human Readable**: Easy to edit manually if needed
- **Version Control Friendly**: Clean diffs when blocks are added/removed
- **Compact**: Only stores placed blocks, empty space is implicit
- **Extensible**: Easy to add new block properties in the future

## Performance Considerations

### Memory Usage
- **HashMap Storage**: Efficient for sparse worlds with many empty spaces
- **No Empty Blocks**: Only placed blocks consume memory
- **Entity Optimization**: Each block is a single ECS entity

### Rendering Optimization
- **Mesh Reuse**: All blocks of the same type share mesh data
- **Material Instancing**: Bevy automatically optimizes material usage
- **Culling**: Off-screen blocks are automatically culled by Bevy

### Scalability
- **Current Limits**: Tested with up to 1000+ blocks without performance issues
- **Future Improvements**: Chunk-based loading system planned for large worlds
- **Streaming**: Potential for loading/unloading distant chunks

## Integration with IoT Devices

### Device Placement
- Devices spawn at specific world coordinates
- Device positions align with the voxel grid when appropriate
- Block placement doesn't interfere with device positioning

### Coordinate Sharing
- Voxel coordinates and device coordinates use the same system
- MQTT position updates work seamlessly with voxel world coordinates
- Physical device movements can be constrained to voxel grid if desired

### Interactive Building
- Players can build structures around IoT devices
- Devices can be "embedded" within voxel structures
- Block placement can be triggered by device events (future feature)

## Building Workflows

### Manual Building
1. **Plan Structure**: Decide on size and block types needed
2. **Build Foundation**: Start with dirt/stone blocks as foundation
3. **Add Walls**: Use stone blocks for structural elements
4. **Surface Details**: Add grass blocks for decoration
5. **Save Progress**: Use `save_map` frequently to preserve work

### Script-Based Building
1. **Create Script File**: Write commands in a `.script` file
2. **Test Commands**: Run individual commands in console first
3. **Execute Script**: Use `load script_name.script`
4. **Iterate**: Modify script and re-run as needed

### Template System
1. **Create Templates**: Build and save common structures
2. **Share Templates**: JSON files can be shared between users
3. **Combine Templates**: Load and merge multiple template files
4. **Version Templates**: Use version control for template management

## Best Practices

### World Design
- **Start Simple**: Begin with basic structures before complex builds
- **Plan Coordinates**: Sketch out coordinate ranges before building
- **Use Layers**: Build in Y-layers for easier visualization
- **Leave Space**: Allow room for IoT device placement

### File Management
- **Descriptive Names**: Use clear, descriptive filenames
- **Regular Saves**: Save work frequently during building sessions  
- **Backup Important Builds**: Keep copies of important structures
- **Organize Files**: Group related builds in directories

### Performance Tips
- **Avoid Massive Solid Areas**: Use hollow structures when possible
- **Clean Up**: Remove unnecessary blocks regularly
- **Test Performance**: Monitor frame rate with large builds
- **Use Scripts**: Automate repetitive building tasks

## Future Enhancements

### Planned Features
- **Chunk System**: Large world support with streaming
- **More Block Types**: Wood, metal, glass, etc.
- **Block Properties**: Different materials, transparency, lighting
- **Advanced Tools**: Copy/paste, fill tools, selection tools
- **Procedural Generation**: Automated terrain and structure generation

### Integration Possibilities
- **Device-Triggered Building**: Blocks placed by IoT device events
- **Sensor-Based Materials**: Block appearance changes based on sensor data
- **Collaborative Building**: Multiple users building simultaneously
- **AR/VR Support**: Building in augmented/virtual reality

This voxel system provides a solid foundation for creating interactive 3D worlds that integrate seamlessly with IoT devices and support complex building workflows.
