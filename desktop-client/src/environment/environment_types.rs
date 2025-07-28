use bevy::prelude::*;
use serde::{Deserialize, Serialize};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockType {
    Grass,
    Dirt,
    Stone,
    QuartzBlock,
    GlassPane,
    CyanTerracotta,
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

    /// Save the voxel world to a JSON file
    pub fn save_to_file(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let map_data = VoxelMapData {
            blocks: self
                .blocks
                .iter()
                .map(|(pos, block_type)| VoxelBlockData {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                    block_type: *block_type,
                })
                .collect(),
        };

        let json = serde_json::to_string_pretty(&map_data)?;
        std::fs::write(filename, json)?;
        Ok(())
    }

    /// Load the voxel world from a JSON file
    pub fn load_from_file(&mut self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(filename)?;
        let map_data: VoxelMapData = serde_json::from_str(&content)?;

        self.blocks.clear();
        for block_data in map_data.blocks {
            self.blocks.insert(
                IVec3::new(block_data.x, block_data.y, block_data.z),
                block_data.block_type,
            );
        }
        Ok(())
    }
}

/// Serializable representation of a voxel block
#[derive(Serialize, Deserialize)]
struct VoxelBlockData {
    x: i32,
    y: i32,
    z: i32,
    block_type: BlockType,
}

/// Serializable representation of the entire voxel map
#[derive(Serialize, Deserialize)]
struct VoxelMapData {
    blocks: Vec<VoxelBlockData>,
}
