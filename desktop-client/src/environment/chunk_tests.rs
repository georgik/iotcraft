#[cfg(test)]
mod tests {
    use crate::environment::chunk_events::*;
    use crate::environment::chunk_types::*;
    use crate::environment::chunked_voxel_world::*;
    use bevy::prelude::*;

    #[test]
    fn test_chunk_coordinate_creation() {
        let coord = ChunkCoordinate::new(5, -3, 10);
        assert_eq!(coord.x, 5);
        assert_eq!(coord.y, -3);
        assert_eq!(coord.z, 10);
    }

    #[test]
    fn test_chunk_coordinate_from_block_position() {
        // Test positive coordinates
        let block_pos = IVec3::new(17, 33, 8);
        let chunk_coord = ChunkCoordinate::from_block_position(block_pos);
        assert_eq!(chunk_coord, ChunkCoordinate::new(1, 2, 0));

        // Test exact chunk boundaries
        let block_pos = IVec3::new(16, 16, 16);
        let chunk_coord = ChunkCoordinate::from_block_position(block_pos);
        assert_eq!(chunk_coord, ChunkCoordinate::new(1, 1, 1));

        // Test negative coordinates
        let block_pos = IVec3::new(-1, -17, -33);
        let chunk_coord = ChunkCoordinate::from_block_position(block_pos);
        assert_eq!(chunk_coord, ChunkCoordinate::new(-1, -2, -3));

        // Test zero
        let block_pos = IVec3::new(0, 0, 0);
        let chunk_coord = ChunkCoordinate::from_block_position(block_pos);
        assert_eq!(chunk_coord, ChunkCoordinate::new(0, 0, 0));
    }

    #[test]
    fn test_chunk_data_creation() {
        let coord = ChunkCoordinate::new(1, 2, 3);
        let chunk_data = ChunkData::new(coord.clone());

        assert_eq!(chunk_data.coordinate, coord);
        assert!(chunk_data.blocks.is_empty());
        assert!(!chunk_data.is_loaded);
        assert_eq!(chunk_data.version, 1);
    }

    #[test]
    fn test_chunked_voxel_world_creation() {
        let world = ChunkedVoxelWorld::default();
        assert_eq!(world.chunks.len(), 0);
        assert_eq!(world.loaded_chunks.len(), 0);
    }

    #[test]
    fn test_chunked_voxel_world_chunk_loading() {
        let mut world = ChunkedVoxelWorld::default();
        let chunk_coord = ChunkCoordinate::new(1, 2, 3);

        // Load chunk
        world.load_chunk(chunk_coord.clone());
        assert!(world.loaded_chunks.contains(&chunk_coord));
        assert!(world.chunks.contains_key(&chunk_coord));
        assert!(world.chunks[&chunk_coord].is_loaded);
    }

    #[test]
    fn test_chunk_loader_component() {
        let loader = ChunkLoader::default();
        assert_eq!(loader.load_radius, 2);
        assert!(loader.last_chunk_position.is_none());
    }

    #[test]
    fn test_chunk_loading_state_default() {
        let state = ChunkLoadingState::default();
        assert_eq!(state.loading_chunks.len(), 0);
        assert_eq!(state.requested_chunks.len(), 0);
    }
}
