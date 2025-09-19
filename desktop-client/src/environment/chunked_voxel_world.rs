use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use super::chunk_types::*;

/// Resource to manage the voxel world state using chunks
#[derive(Resource)]
pub struct ChunkedVoxelWorld {
    pub chunks: HashMap<ChunkCoordinate, ChunkData>,
    pub loaded_chunks: HashSet<ChunkCoordinate>,
}

impl Default for ChunkedVoxelWorld {
    fn default() -> Self {
        Self {
            chunks: HashMap::new(),
            loaded_chunks: HashSet::new(),
        }
    }
}

impl ChunkedVoxelWorld {
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
}
