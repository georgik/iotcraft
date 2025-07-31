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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::BlockType;

    #[test]
    fn test_world_metadata_default() {
        let metadata = WorldMetadata::default();
        assert_eq!(metadata.name, "New World");
        assert_eq!(metadata.description, "A new world to explore");
        assert_eq!(metadata.version, "1.0.0");
        // created_at and last_played should be set to current time
        assert!(!metadata.created_at.is_empty());
        assert!(!metadata.last_played.is_empty());
    }

    #[test]
    fn test_voxel_block_data_serialization() {
        let block_data = VoxelBlockData {
            x: 10,
            y: 20,
            z: 30,
            block_type: BlockType::Stone,
        };

        // Test that it can be serialized and deserialized
        let json = serde_json::to_string(&block_data).unwrap();
        let deserialized: VoxelBlockData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.x, 10);
        assert_eq!(deserialized.y, 20);
        assert_eq!(deserialized.z, 30);
        assert_eq!(deserialized.block_type, BlockType::Stone);
    }

    #[test]
    fn test_world_save_data_serialization() {
        let metadata = WorldMetadata {
            name: "Test World".to_string(),
            description: "A test world".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            last_played: "2024-01-01T12:00:00Z".to_string(),
            version: "1.0.0".to_string(),
        };

        let blocks = vec![
            VoxelBlockData {
                x: 0,
                y: 0,
                z: 0,
                block_type: BlockType::Grass,
            },
            VoxelBlockData {
                x: 1,
                y: 0,
                z: 0,
                block_type: BlockType::Stone,
            },
        ];

        let save_data = WorldSaveData {
            metadata,
            blocks,
            player_position: Vec3::new(5.0, 10.0, 15.0),
            player_rotation: Quat::from_rotation_y(1.57),
            inventory: crate::inventory::PlayerInventory::new(),
        };

        // Test serialization/deserialization
        let json = serde_json::to_string_pretty(&save_data).unwrap();
        let deserialized: WorldSaveData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.metadata.name, "Test World");
        assert_eq!(deserialized.blocks.len(), 2);
        assert_eq!(deserialized.player_position, Vec3::new(5.0, 10.0, 15.0));
        assert_eq!(deserialized.blocks[0].block_type, BlockType::Grass);
        assert_eq!(deserialized.blocks[1].block_type, BlockType::Stone);
    }

    #[test]
    fn test_discovered_worlds_default() {
        let discovered = DiscoveredWorlds::default();
        assert!(discovered.worlds.is_empty());
    }
}
