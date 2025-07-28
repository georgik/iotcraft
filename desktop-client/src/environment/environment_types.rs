use bevy::prelude::*;
use std::collections::HashMap;

/// Standard size for all cubes in the world (voxels, devices, etc.)
pub const CUBE_SIZE: f32 = 1.0;

#[derive(Component)]
pub struct Ground;

#[derive(Component)]
pub struct Thermometer;

#[derive(Resource)]
pub struct ThermometerMaterial(pub Handle<StandardMaterial>);

#[derive(Component)]
pub struct LogoCube;

/// Component for voxel blocks in the world
#[derive(Component)]
pub struct VoxelBlock {
    pub block_type: BlockType,
    pub position: IVec3,
}

/// Types of blocks available in the voxel world
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Grass,
    Dirt,
    Stone,
}

/// Resource to manage the voxel world state
#[derive(Resource)]
pub struct VoxelWorld {
    pub blocks: HashMap<IVec3, BlockType>,
    pub chunk_size: i32,
}

impl Default for VoxelWorld {
    fn default() -> Self {
        Self {
            blocks: HashMap::new(),
            chunk_size: 16,
        }
    }
}

impl VoxelWorld {
    /// Add a block at the given position
    pub fn set_block(&mut self, position: IVec3, block_type: BlockType) {
        self.blocks.insert(position, block_type);
    }
    
    /// Remove a block at the given position
    pub fn remove_block(&mut self, position: &IVec3) -> Option<BlockType> {
        self.blocks.remove(position)
    }
    
    /// Get block type at position
    pub fn get_block(&self, position: &IVec3) -> Option<BlockType> {
        self.blocks.get(position).copied()
    }
    
    /// Generate a flat grass terrain
    pub fn generate_flat_terrain(&mut self, size: i32, height: i32) {
        for x in -size..=size {
            for z in -size..=size {
                self.set_block(IVec3::new(x, height, z), BlockType::Grass);
            }
        }
    }
}
