use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::environment::BlockType;

use crate::inventory::PlayerInventory;

/// Represents a saved world's metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMetadata {
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub last_played: String,
    pub version: String,
}

/// Serializable representation of a VoxelBlock
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxelBlockData {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block_type: BlockType,
}

/// Complete world save data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSaveData {
    pub metadata: WorldMetadata,
    pub blocks: Vec<VoxelBlockData>,
    pub player_position: Vec3,
    pub player_rotation: Quat,
    #[serde(default)]
    pub inventory: PlayerInventory,
}

/// Resource to track the current world
#[derive(Resource, Debug, Clone)]
pub struct CurrentWorld {
    pub name: String,
    pub path: PathBuf,
    pub metadata: WorldMetadata,
}

/// Resource to hold discovered worlds
#[derive(Resource, Debug, Clone, Default)]
pub struct DiscoveredWorlds {
    pub worlds: Vec<WorldInfo>,
}

/// Information about a discovered world
#[derive(Debug, Clone)]
pub struct WorldInfo {
    pub name: String,
    pub path: PathBuf,
    pub metadata: WorldMetadata,
}

/// Event to request world loading
#[derive(Event)]
pub struct LoadWorldEvent {
    pub world_name: String,
}

/// Event to request world saving
#[derive(Event)]
pub struct SaveWorldEvent {
    pub world_name: String,
}

/// Event to request creating a new world
#[derive(Event)]
pub struct CreateWorldEvent {
    pub world_name: String,
    pub description: String,
}

impl Default for WorldMetadata {
    fn default() -> Self {
        Self {
            name: "New World".to_string(),
            description: "A new world to explore".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_played: chrono::Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
        }
    }
}
