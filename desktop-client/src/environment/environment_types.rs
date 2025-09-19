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
    pub position: IVec3,
}

/// Types of blocks available in the voxel world
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockType {
    Grass,
    Dirt,
    Stone,
    QuartzBlock,
    GlassPane,
    CyanTerracotta,
    Water,
}

/// Resource to manage the voxel world state
#[derive(Resource)]
pub struct VoxelWorld {
    pub blocks: HashMap<IVec3, BlockType>,
}

impl Default for VoxelWorld {
    fn default() -> Self {
        Self {
            blocks: HashMap::new(),
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

    /// Check if there's a block at the given position
    pub fn is_block_at(&self, position: IVec3) -> bool {
        self.blocks.contains_key(&position)
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
}

/// Serializable representation of a voxel block
#[derive(Serialize, Deserialize)]
pub struct VoxelBlockData {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block_type: BlockType,
}

/// Serializable representation of the entire voxel map
#[derive(Serialize, Deserialize)]
pub struct VoxelMapData {
    pub blocks: Vec<VoxelBlockData>,
}
