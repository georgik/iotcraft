use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use super::chunk_types::*;
use super::{BlockType, VoxelBlockData, VoxelMapData};

/// Resource to manage the voxel world state using chunks
#[derive(Resource)]
pub struct ChunkedVoxelWorld {
    pub chunks: HashMap<ChunkCoordinate, ChunkData>,
    pub loaded_chunks: HashSet<ChunkCoordinate>,
    pub chunk_size: i32,
}

impl Default for ChunkedVoxelWorld {
    fn default() -> Self {
        Self {
            chunks: HashMap::new(),
            loaded_chunks: HashSet::new(),
            chunk_size: CHUNK_SIZE,
        }
    }
}

impl ChunkedVoxelWorld {
    /// Create a new chunked voxel world
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a block at the given position
    pub fn set_block(&mut self, position: IVec3, block_type: BlockType) {
        let chunk_coord = ChunkCoordinate::from_block_position(position);

        let chunk = self.chunks.entry(chunk_coord.clone()).or_insert_with(|| {
            let mut chunk = ChunkData::new(chunk_coord.clone());
            chunk.is_loaded = self.loaded_chunks.contains(&chunk_coord);
            chunk
        });

        chunk.set_block(position, block_type);
    }

    /// Remove a block at the given position
    pub fn remove_block(&mut self, position: &IVec3) -> Option<BlockType> {
        let chunk_coord = ChunkCoordinate::from_block_position(*position);

        if let Some(chunk) = self.chunks.get_mut(&chunk_coord) {
            let removed = chunk.remove_block(position);

            // Remove empty chunks that aren't loaded to save memory
            if chunk.is_empty() && !chunk.is_loaded {
                self.chunks.remove(&chunk_coord);
            }

            removed
        } else {
            None
        }
    }

    /// Get a block at the given position
    pub fn get_block(&self, position: &IVec3) -> Option<BlockType> {
        let chunk_coord = ChunkCoordinate::from_block_position(*position);

        self.chunks.get(&chunk_coord)?.get_block(position)
    }

    /// Check if there's a block at the given position
    pub fn is_block_at(&self, position: IVec3) -> bool {
        self.get_block(&position).is_some()
    }

    /// Load a chunk into active memory
    pub fn load_chunk(&mut self, chunk_coord: ChunkCoordinate) {
        self.loaded_chunks.insert(chunk_coord.clone());

        if let Some(chunk) = self.chunks.get_mut(&chunk_coord) {
            chunk.is_loaded = true;
        } else {
            // Create empty chunk if it doesn't exist
            let mut chunk = ChunkData::new(chunk_coord.clone());
            chunk.is_loaded = true;
            self.chunks.insert(chunk_coord, chunk);
        }
    }

    /// Unload a chunk from active memory (but keep data if it has blocks)
    pub fn unload_chunk(&mut self, chunk_coord: &ChunkCoordinate) {
        self.loaded_chunks.remove(chunk_coord);

        if let Some(chunk) = self.chunks.get_mut(chunk_coord) {
            chunk.is_loaded = false;

            // Remove empty chunks when unloading to save memory
            if chunk.is_empty() {
                self.chunks.remove(chunk_coord);
            }
        }
    }

    /// Get all loaded chunks
    pub fn get_loaded_chunks(&self) -> &HashSet<ChunkCoordinate> {
        &self.loaded_chunks
    }

    /// Get chunk data if it exists
    pub fn get_chunk(&self, chunk_coord: &ChunkCoordinate) -> Option<&ChunkData> {
        self.chunks.get(chunk_coord)
    }

    /// Get mutable chunk data if it exists
    pub fn get_chunk_mut(&mut self, chunk_coord: &ChunkCoordinate) -> Option<&mut ChunkData> {
        self.chunks.get_mut(chunk_coord)
    }

    /// Get chunks within a radius of a position (in chunk coordinates)
    pub fn get_chunks_in_radius(&self, center: ChunkCoordinate, radius: i32) -> Vec<&ChunkData> {
        let mut chunks = Vec::new();

        for x in (center.x - radius)..=(center.x + radius) {
            for y in (center.y - radius)..=(center.y + radius) {
                for z in (center.z - radius)..=(center.z + radius) {
                    let coord = ChunkCoordinate::new(x, y, z);
                    if let Some(chunk) = self.chunks.get(&coord) {
                        chunks.push(chunk);
                    }
                }
            }
        }

        chunks
    }

    /// Get chunks around a block position
    pub fn get_chunks_around_position(
        &self,
        position: IVec3,
        chunk_radius: i32,
    ) -> Vec<&ChunkData> {
        let center_chunk = ChunkCoordinate::from_block_position(position);
        self.get_chunks_in_radius(center_chunk, chunk_radius)
    }

    /// Generate a flat grass terrain (for compatibility)
    pub fn generate_flat_terrain(&mut self, size: i32, height: i32) {
        for x in -size..=size {
            for z in -size..=size {
                self.set_block(IVec3::new(x, height, z), BlockType::Grass);

                // Ensure the chunk containing this block is loaded
                let chunk_coord = ChunkCoordinate::from_block_position(IVec3::new(x, height, z));
                self.load_chunk(chunk_coord);
            }
        }
    }

    /// Get total number of blocks across all chunks
    pub fn total_block_count(&self) -> usize {
        self.chunks.values().map(|chunk| chunk.block_count()).sum()
    }

    /// Get number of active chunks
    pub fn active_chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Get world metadata for MQTT publishing
    pub fn get_world_metadata(&self, world_id: String) -> ChunkedWorldMetadata {
        let active_chunks: Vec<ChunkCoordinate> = self.loaded_chunks.iter().cloned().collect();

        // Calculate bounding box
        let bounding_box = if !self.chunks.is_empty() {
            let chunk_coords: Vec<&ChunkCoordinate> = self.chunks.keys().collect();

            let min_x = chunk_coords.iter().map(|c| c.x).min().unwrap();
            let max_x = chunk_coords.iter().map(|c| c.x).max().unwrap();
            let min_y = chunk_coords.iter().map(|c| c.y).min().unwrap();
            let max_y = chunk_coords.iter().map(|c| c.y).max().unwrap();
            let min_z = chunk_coords.iter().map(|c| c.z).min().unwrap();
            let max_z = chunk_coords.iter().map(|c| c.z).max().unwrap();

            Some(ChunkBoundingBox {
                min: ChunkCoordinate::new(min_x, min_y, min_z),
                max: ChunkCoordinate::new(max_x, max_y, max_z),
            })
        } else {
            None
        };

        ChunkedWorldMetadata {
            world_id,
            chunk_size: self.chunk_size,
            active_chunks,
            total_blocks: self.total_block_count(),
            last_updated: now_timestamp(),
            bounding_box,
        }
    }

    /// Save to legacy VoxelMapData format for compatibility
    pub fn to_legacy_map_data(&self) -> VoxelMapData {
        let mut blocks = Vec::new();

        for chunk in self.chunks.values() {
            for (position, block_type) in &chunk.blocks {
                blocks.push(VoxelBlockData {
                    x: position.x,
                    y: position.y,
                    z: position.z,
                    block_type: *block_type,
                });
            }
        }

        VoxelMapData { blocks }
    }

    /// Load from legacy VoxelMapData format for compatibility
    pub fn from_legacy_map_data(&mut self, map_data: VoxelMapData) {
        // Clear existing data
        self.chunks.clear();
        self.loaded_chunks.clear();

        // Load blocks and automatically create chunks
        for block_data in map_data.blocks {
            let position = IVec3::new(block_data.x, block_data.y, block_data.z);
            self.set_block(position, block_data.block_type);

            // Mark chunk as loaded
            let chunk_coord = ChunkCoordinate::from_block_position(position);
            self.load_chunk(chunk_coord);
        }
    }

    /// Save the chunked world to a JSON file (compatibility with VoxelWorld)
    pub fn save_to_file(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let map_data = self.to_legacy_map_data();
        let json = serde_json::to_string_pretty(&map_data)?;
        std::fs::write(filename, json)?;
        Ok(())
    }

    /// Load the chunked world from a JSON file (compatibility with VoxelWorld)
    pub fn load_from_file(&mut self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(filename)?;
        let map_data: VoxelMapData = serde_json::from_str(&content)?;
        self.from_legacy_map_data(map_data);
        Ok(())
    }

    /// Get all chunks that have been modified since a given timestamp
    pub fn get_modified_chunks_since(&self, timestamp: u64) -> Vec<&ChunkData> {
        self.chunks
            .values()
            .filter(|chunk| chunk.last_modified > timestamp)
            .collect()
    }

    /// Clear all chunk data
    pub fn clear(&mut self) {
        self.chunks.clear();
        self.loaded_chunks.clear();
    }
}

/// Compatibility methods to match VoxelWorld interface
impl ChunkedVoxelWorld {
    /// Get all blocks as a HashMap for compatibility
    pub fn get_all_blocks(&self) -> HashMap<IVec3, BlockType> {
        let mut all_blocks = HashMap::new();

        for chunk in self.chunks.values() {
            all_blocks.extend(chunk.blocks.clone());
        }

        all_blocks
    }

    /// Get blocks field for compatibility (read-only reference)
    pub fn blocks(&self) -> HashMap<IVec3, BlockType> {
        self.get_all_blocks()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunked_voxel_world_operations() {
        let mut world = ChunkedVoxelWorld::new();

        // Test setting blocks in different chunks
        world.set_block(IVec3::new(5, 5, 5), BlockType::Stone);
        world.set_block(IVec3::new(20, 20, 20), BlockType::Grass);

        assert_eq!(
            world.get_block(&IVec3::new(5, 5, 5)),
            Some(BlockType::Stone)
        );
        assert_eq!(
            world.get_block(&IVec3::new(20, 20, 20)),
            Some(BlockType::Grass)
        );
        assert_eq!(world.active_chunk_count(), 2);
        assert_eq!(world.total_block_count(), 2);

        // Test removing blocks
        assert_eq!(
            world.remove_block(&IVec3::new(5, 5, 5)),
            Some(BlockType::Stone)
        );
        assert_eq!(world.get_block(&IVec3::new(5, 5, 5)), None);
        assert_eq!(world.total_block_count(), 1);
    }

    #[test]
    fn test_chunk_loading_unloading() {
        let mut world = ChunkedVoxelWorld::new();
        let chunk_coord = ChunkCoordinate::new(0, 0, 0);

        // Load chunk
        world.load_chunk(chunk_coord.clone());
        assert!(world.loaded_chunks.contains(&chunk_coord));

        // Add block to loaded chunk
        world.set_block(IVec3::new(5, 5, 5), BlockType::Stone);
        assert_eq!(
            world.get_block(&IVec3::new(5, 5, 5)),
            Some(BlockType::Stone)
        );

        // Unload chunk - should keep data since it has blocks
        world.unload_chunk(&chunk_coord);
        assert!(!world.loaded_chunks.contains(&chunk_coord));
        assert_eq!(
            world.get_block(&IVec3::new(5, 5, 5)),
            Some(BlockType::Stone)
        );
        assert_eq!(world.active_chunk_count(), 1);
    }

    #[test]
    fn test_compatibility_with_legacy_format() {
        let mut world = ChunkedVoxelWorld::new();

        // Add some blocks
        world.set_block(IVec3::new(0, 0, 0), BlockType::Grass);
        world.set_block(IVec3::new(16, 16, 16), BlockType::Stone);

        // Convert to legacy format
        let legacy_data = world.to_legacy_map_data();
        assert_eq!(legacy_data.blocks.len(), 2);

        // Create new world from legacy data
        let mut new_world = ChunkedVoxelWorld::new();
        new_world.from_legacy_map_data(legacy_data);

        assert_eq!(
            new_world.get_block(&IVec3::new(0, 0, 0)),
            Some(BlockType::Grass)
        );
        assert_eq!(
            new_world.get_block(&IVec3::new(16, 16, 16)),
            Some(BlockType::Stone)
        );
        assert_eq!(new_world.total_block_count(), 2);
    }

    #[test]
    fn test_chunks_in_radius() {
        let mut world = ChunkedVoxelWorld::new();

        // Add blocks in different chunks
        world.set_block(IVec3::new(0, 0, 0), BlockType::Grass); // Chunk (0,0,0)
        world.set_block(IVec3::new(16, 0, 0), BlockType::Stone); // Chunk (1,0,0)
        world.set_block(IVec3::new(32, 0, 0), BlockType::Dirt); // Chunk (2,0,0)

        // Get chunks in radius 1 from (0,0,0)
        let chunks = world.get_chunks_in_radius(ChunkCoordinate::new(0, 0, 0), 1);
        assert_eq!(chunks.len(), 2); // Should include chunks (0,0,0) and (1,0,0)

        // Get chunks in radius 0 from (1,0,0)
        let chunks = world.get_chunks_in_radius(ChunkCoordinate::new(1, 0, 0), 0);
        assert_eq!(chunks.len(), 1); // Should include only chunk (1,0,0)
    }
}
