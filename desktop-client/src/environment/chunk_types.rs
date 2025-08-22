use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::BlockType;

/// Chunk size following Luanti's approach - 16x16x16 blocks
pub const CHUNK_SIZE: i32 = 16;

/// Simple bounding box for block coordinates
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub min: IVec3,
    pub max: IVec3,
}

/// Coordinate system for chunks in the world
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkCoordinate {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkCoordinate {
    /// Create a new chunk coordinate
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Convert a block position to its containing chunk coordinate
    /// Uses floor division to handle negative coordinates correctly
    pub fn from_block_position(block_pos: IVec3) -> Self {
        Self {
            x: block_pos.x.div_euclid(CHUNK_SIZE),
            y: block_pos.y.div_euclid(CHUNK_SIZE),
            z: block_pos.z.div_euclid(CHUNK_SIZE),
        }
    }

    /// Convert chunk coordinate to MQTT topic path
    pub fn to_topic_path(&self) -> String {
        format!("{}/{}/{}", self.x, self.y, self.z)
    }

    /// Get the minimum block position for this chunk
    pub fn min_block_position(&self) -> IVec3 {
        IVec3::new(
            self.x * CHUNK_SIZE,
            self.y * CHUNK_SIZE,
            self.z * CHUNK_SIZE,
        )
    }

    /// Get the maximum block position for this chunk (inclusive)
    pub fn max_block_position(&self) -> IVec3 {
        IVec3::new(
            self.x * CHUNK_SIZE + CHUNK_SIZE - 1,
            self.y * CHUNK_SIZE + CHUNK_SIZE - 1,
            self.z * CHUNK_SIZE + CHUNK_SIZE - 1,
        )
    }

    /// Check if a block position is within this chunk
    pub fn contains_block(&self, block_pos: IVec3) -> bool {
        let min = self.min_block_position();
        let max = self.max_block_position();
        block_pos.x >= min.x
            && block_pos.x <= max.x
            && block_pos.y >= min.y
            && block_pos.y <= max.y
            && block_pos.z >= min.z
            && block_pos.z <= max.z
    }

    /// Get the bounding box of this chunk in block coordinates
    pub fn bounding_box(&self) -> BoundingBox {
        BoundingBox {
            min: self.min_block_position(),
            max: self.max_block_position(),
        }
    }

    /// Get the center position of this chunk in world coordinates
    pub fn center(&self) -> Vec3 {
        let min = self.min_block_position().as_vec3();
        let max = self.max_block_position().as_vec3();
        (min + max) / 2.0
    }

    /// Get neighboring chunk coordinates
    pub fn neighbors(&self) -> Vec<ChunkCoordinate> {
        let mut neighbors = Vec::with_capacity(26); // 3x3x3 - 1
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue; // Skip self
                    }
                    neighbors.push(ChunkCoordinate::new(self.x + dx, self.y + dy, self.z + dz));
                }
            }
        }
        neighbors
    }
}

/// Data stored in a single chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkData {
    pub coordinate: ChunkCoordinate,
    pub blocks: HashMap<IVec3, BlockType>,
    pub last_modified: u64,
    pub version: u32,
    pub is_loaded: bool,
}

impl ChunkData {
    /// Create a new empty chunk
    pub fn new(coordinate: ChunkCoordinate) -> Self {
        Self {
            coordinate,
            blocks: HashMap::new(),
            last_modified: now_timestamp(),
            version: 1,
            is_loaded: false,
        }
    }

    /// Set a block in this chunk
    pub fn set_block(&mut self, position: IVec3, block_type: BlockType) -> bool {
        if !self.coordinate.contains_block(position) {
            return false; // Block not in this chunk
        }

        self.blocks.insert(position, block_type);
        self.last_modified = now_timestamp();
        self.version += 1;
        true
    }

    /// Remove a block from this chunk
    pub fn remove_block(&mut self, position: &IVec3) -> Option<BlockType> {
        if !self.coordinate.contains_block(*position) {
            return None; // Block not in this chunk
        }

        let removed = self.blocks.remove(position);
        if removed.is_some() {
            self.last_modified = now_timestamp();
            self.version += 1;
        }
        removed
    }

    /// Get a block from this chunk
    pub fn get_block(&self, position: &IVec3) -> Option<BlockType> {
        self.blocks.get(position).copied()
    }

    /// Check if this chunk has any blocks
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Get the number of blocks in this chunk
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Get all block positions in this chunk
    pub fn get_all_positions(&self) -> Vec<IVec3> {
        self.blocks.keys().copied().collect()
    }
}

/// Metadata about a chunk for MQTT publishing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub coordinate: ChunkCoordinate,
    pub block_count: usize,
    pub last_modified: u64,
    pub version: u32,
    pub is_active: bool,
}

impl From<&ChunkData> for ChunkMetadata {
    fn from(chunk: &ChunkData) -> Self {
        Self {
            coordinate: chunk.coordinate.clone(),
            block_count: chunk.blocks.len(),
            last_modified: chunk.last_modified,
            version: chunk.version,
            is_active: chunk.is_loaded,
        }
    }
}

/// Individual block change within a chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkBlockChange {
    pub position: IVec3,
    pub change_type: ChunkBlockChangeType,
    pub timestamp: u64,
    pub player_id: String,
    pub chunk_coordinate: ChunkCoordinate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkBlockChangeType {
    Placed { block_type: BlockType },
    Removed,
}

/// World metadata for chunk-based system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkedWorldMetadata {
    pub world_id: String,
    pub chunk_size: i32,
    pub active_chunks: Vec<ChunkCoordinate>,
    pub total_blocks: usize,
    pub last_updated: u64,
    pub bounding_box: Option<ChunkBoundingBox>,
}

/// Bounding box in chunk coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkBoundingBox {
    pub min: ChunkCoordinate,
    pub max: ChunkCoordinate,
}

/// Helper function to get current timestamp
pub fn now_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_coordinate_from_block_position() {
        // Test positive coordinates
        assert_eq!(
            ChunkCoordinate::from_block_position(IVec3::new(0, 0, 0)),
            ChunkCoordinate::new(0, 0, 0)
        );
        assert_eq!(
            ChunkCoordinate::from_block_position(IVec3::new(15, 15, 15)),
            ChunkCoordinate::new(0, 0, 0)
        );
        assert_eq!(
            ChunkCoordinate::from_block_position(IVec3::new(16, 16, 16)),
            ChunkCoordinate::new(1, 1, 1)
        );

        // Test negative coordinates
        assert_eq!(
            ChunkCoordinate::from_block_position(IVec3::new(-1, -1, -1)),
            ChunkCoordinate::new(-1, -1, -1)
        );
        assert_eq!(
            ChunkCoordinate::from_block_position(IVec3::new(-16, -16, -16)),
            ChunkCoordinate::new(-1, -1, -1)
        );
        assert_eq!(
            ChunkCoordinate::from_block_position(IVec3::new(-17, -17, -17)),
            ChunkCoordinate::new(-2, -2, -2)
        );
    }

    #[test]
    fn test_chunk_contains_block() {
        let chunk = ChunkCoordinate::new(0, 0, 0);

        // Should contain blocks 0-15 in each dimension
        assert!(chunk.contains_block(IVec3::new(0, 0, 0)));
        assert!(chunk.contains_block(IVec3::new(15, 15, 15)));
        assert!(!chunk.contains_block(IVec3::new(16, 16, 16)));
        assert!(!chunk.contains_block(IVec3::new(-1, -1, -1)));

        let negative_chunk = ChunkCoordinate::new(-1, -1, -1);
        assert!(negative_chunk.contains_block(IVec3::new(-1, -1, -1)));
        assert!(negative_chunk.contains_block(IVec3::new(-16, -16, -16)));
        assert!(!negative_chunk.contains_block(IVec3::new(0, 0, 0)));
        assert!(!negative_chunk.contains_block(IVec3::new(-17, -17, -17)));
    }

    #[test]
    fn test_chunk_data_operations() {
        let mut chunk = ChunkData::new(ChunkCoordinate::new(0, 0, 0));

        // Test setting blocks
        assert!(chunk.set_block(IVec3::new(5, 5, 5), BlockType::Stone));
        assert_eq!(
            chunk.get_block(&IVec3::new(5, 5, 5)),
            Some(BlockType::Stone)
        );
        assert_eq!(chunk.block_count(), 1);

        // Test setting block outside chunk bounds
        assert!(!chunk.set_block(IVec3::new(16, 16, 16), BlockType::Stone));
        assert_eq!(chunk.block_count(), 1);

        // Test removing blocks
        assert_eq!(
            chunk.remove_block(&IVec3::new(5, 5, 5)),
            Some(BlockType::Stone)
        );
        assert_eq!(chunk.get_block(&IVec3::new(5, 5, 5)), None);
        assert_eq!(chunk.block_count(), 0);
        assert!(chunk.is_empty());
    }

    #[test]
    fn test_topic_path_generation() {
        let chunk = ChunkCoordinate::new(1, -2, 3);
        assert_eq!(chunk.to_topic_path(), "1/-2/3");
    }
}
