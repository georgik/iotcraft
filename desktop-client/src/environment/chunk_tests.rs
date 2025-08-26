#[cfg(test)]
mod tests {
    use crate::environment::BlockType;
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
    fn test_chunk_coordinate_contains_block() {
        let chunk_coord = ChunkCoordinate::new(1, 0, -1);

        // Test blocks within chunk
        assert!(chunk_coord.contains_block(IVec3::new(16, 0, -16)));
        assert!(chunk_coord.contains_block(IVec3::new(31, 15, -1)));

        // Test blocks outside chunk
        assert!(!chunk_coord.contains_block(IVec3::new(15, 0, -16)));
        assert!(!chunk_coord.contains_block(IVec3::new(32, 0, -16)));
        assert!(!chunk_coord.contains_block(IVec3::new(16, 16, -16)));
        assert!(!chunk_coord.contains_block(IVec3::new(16, 0, 0)));
    }

    #[test]
    fn test_chunk_coordinate_bounding_box() {
        let chunk_coord = ChunkCoordinate::new(2, -1, 0);
        let bbox = chunk_coord.bounding_box();

        assert_eq!(bbox.min, IVec3::new(32, -16, 0));
        assert_eq!(bbox.max, IVec3::new(47, -1, 15));
    }

    #[test]
    fn test_chunk_coordinate_center() {
        let chunk_coord = ChunkCoordinate::new(0, 0, 0);
        let center = chunk_coord.center();
        assert_eq!(center, Vec3::new(7.5, 7.5, 7.5));

        let chunk_coord = ChunkCoordinate::new(1, -1, 2);
        let center = chunk_coord.center();
        assert_eq!(center, Vec3::new(23.5, -8.5, 39.5));
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
    fn test_chunk_data_block_operations() {
        let coord = ChunkCoordinate::new(0, 0, 0);
        let mut chunk_data = ChunkData::new(coord);

        let block_pos = IVec3::new(5, 10, 8);

        // Test setting a block
        chunk_data.set_block(block_pos, BlockType::Stone);
        assert_eq!(chunk_data.blocks.get(&block_pos), Some(&BlockType::Stone));
        assert!(!chunk_data.is_empty());

        // Test removing a block
        let removed = chunk_data.remove_block(&block_pos);
        assert_eq!(removed, Some(BlockType::Stone));
        assert_eq!(chunk_data.blocks.get(&block_pos), None);
        assert!(chunk_data.is_empty());
    }

    #[test]
    fn test_chunk_data_is_empty() {
        let coord = ChunkCoordinate::new(0, 0, 0);
        let mut chunk_data = ChunkData::new(coord);

        assert!(chunk_data.is_empty());

        chunk_data.set_block(IVec3::new(0, 0, 0), BlockType::Grass);
        assert!(!chunk_data.is_empty());

        chunk_data.remove_block(&IVec3::new(0, 0, 0));
        assert!(chunk_data.is_empty());
    }

    #[test]
    fn test_chunked_voxel_world_creation() {
        let world = ChunkedVoxelWorld::new();
        assert_eq!(world.chunks.len(), 0);
        assert_eq!(world.loaded_chunks.len(), 0);
        assert_eq!(world.chunk_size, CHUNK_SIZE);
    }

    #[test]
    fn test_chunked_voxel_world_block_operations() {
        let mut world = ChunkedVoxelWorld::new();

        let pos1 = IVec3::new(5, 10, 8);
        let pos2 = IVec3::new(25, 5, 12); // Different chunk

        // Set blocks in different chunks
        world.set_block(pos1, BlockType::Stone);
        world.set_block(pos2, BlockType::Grass);

        assert_eq!(world.get_block(&pos1), Some(BlockType::Stone));
        assert_eq!(world.get_block(&pos2), Some(BlockType::Grass));
        assert_eq!(world.active_chunk_count(), 2);
        assert_eq!(world.total_block_count(), 2);

        // Remove blocks
        assert_eq!(world.remove_block(&pos1), Some(BlockType::Stone));
        assert_eq!(world.get_block(&pos1), None);
        assert_eq!(world.total_block_count(), 1);
    }

    #[test]
    fn test_chunked_voxel_world_chunk_loading() {
        let mut world = ChunkedVoxelWorld::new();
        let chunk_coord = ChunkCoordinate::new(1, 2, 3);

        // Load chunk
        world.load_chunk(chunk_coord.clone());
        assert!(world.loaded_chunks.contains(&chunk_coord));
        assert!(world.chunks.contains_key(&chunk_coord));
        assert!(world.chunks[&chunk_coord].is_loaded);

        // Unload chunk (empty chunks should be removed)
        world.unload_chunk(&chunk_coord);
        assert!(!world.loaded_chunks.contains(&chunk_coord));
        assert!(!world.chunks.contains_key(&chunk_coord)); // Empty chunk removed
    }

    #[test]
    fn test_chunked_voxel_world_chunk_loading_with_blocks() {
        let mut world = ChunkedVoxelWorld::new();
        let chunk_coord = ChunkCoordinate::new(0, 0, 0);
        let block_pos = IVec3::new(5, 5, 5);

        // Add block and load chunk
        world.set_block(block_pos, BlockType::Dirt);
        world.load_chunk(chunk_coord.clone());

        // Unload chunk (should keep data since it has blocks)
        world.unload_chunk(&chunk_coord);
        assert!(!world.loaded_chunks.contains(&chunk_coord));
        assert!(world.chunks.contains_key(&chunk_coord)); // Chunk kept because it has blocks
        assert!(!world.chunks[&chunk_coord].is_loaded);
        assert_eq!(world.get_block(&block_pos), Some(BlockType::Dirt));
    }

    #[test]
    fn test_chunked_voxel_world_legacy_compatibility() {
        let mut world = ChunkedVoxelWorld::new();

        // Add some blocks
        world.set_block(IVec3::new(0, 0, 0), BlockType::Grass);
        world.set_block(IVec3::new(16, 16, 16), BlockType::Stone);
        world.set_block(IVec3::new(-5, -10, 20), BlockType::Dirt);

        // Convert to legacy format
        let legacy_data = world.to_legacy_map_data();
        assert_eq!(legacy_data.blocks.len(), 3);

        // Verify blocks are present
        let grass_block = legacy_data
            .blocks
            .iter()
            .find(|b| b.x == 0 && b.y == 0 && b.z == 0)
            .expect("Grass block should be present");
        assert_eq!(grass_block.block_type, BlockType::Grass);

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
        assert_eq!(
            new_world.get_block(&IVec3::new(-5, -10, 20)),
            Some(BlockType::Dirt)
        );
        assert_eq!(new_world.total_block_count(), 3);
    }

    #[test]
    fn test_chunked_voxel_world_chunks_in_radius() {
        let mut world = ChunkedVoxelWorld::new();

        // Add blocks in different chunks
        world.set_block(IVec3::new(0, 0, 0), BlockType::Grass); // Chunk (0,0,0)
        world.set_block(IVec3::new(16, 0, 0), BlockType::Stone); // Chunk (1,0,0)
        world.set_block(IVec3::new(32, 0, 0), BlockType::Dirt); // Chunk (2,0,0)
        world.set_block(IVec3::new(0, 16, 0), BlockType::Water); // Chunk (0,1,0)

        // Get chunks in radius 1 from (0,0,0)
        let chunks = world.get_chunks_in_radius(ChunkCoordinate::new(0, 0, 0), 1);
        assert_eq!(chunks.len(), 3); // Should include chunks (0,0,0) and (1,0,0) and (0,1,0)

        // Get chunks in radius 0 from (1,0,0)
        let chunks = world.get_chunks_in_radius(ChunkCoordinate::new(1, 0, 0), 0);
        assert_eq!(chunks.len(), 1); // Should include only chunk (1,0,0)
    }

    #[test]
    fn test_chunk_loading_state() {
        let mut state = ChunkLoadingState::new();
        let chunk1 = ChunkCoordinate::new(0, 0, 0);
        let chunk2 = ChunkCoordinate::new(1, 1, 1);

        // Test initial state
        assert!(!state.is_loading(&chunk1));

        // Start loading
        state.start_loading(chunk1.clone());
        assert!(state.is_loading(&chunk1));
        assert!(!state.is_loading(&chunk2));

        // Finish loading
        state.finish_loading(&chunk1);
        assert!(!state.is_loading(&chunk1));
    }

    #[test]
    fn test_chunk_loader_component() {
        let loader = ChunkLoader::default();
        assert_eq!(loader.load_radius, 2);
        assert!(loader.last_chunk_position.is_none());
    }

    #[test]
    fn test_world_metadata_generation() {
        let mut world = ChunkedVoxelWorld::new();
        let world_id = "test_world_123".to_string();

        // Add blocks in multiple chunks
        world.set_block(IVec3::new(0, 0, 0), BlockType::Grass);
        world.set_block(IVec3::new(16, 16, 16), BlockType::Stone);
        world.set_block(IVec3::new(-16, -16, -16), BlockType::Dirt);

        // Load some chunks
        world.load_chunk(ChunkCoordinate::new(0, 0, 0));
        world.load_chunk(ChunkCoordinate::new(1, 1, 1));

        let metadata = world.get_world_metadata(world_id.clone());

        assert_eq!(metadata.world_id, world_id);
        assert_eq!(metadata.chunk_size, CHUNK_SIZE);
        assert_eq!(metadata.active_chunks.len(), 2);
        assert_eq!(metadata.total_blocks, 3);
        assert!(metadata.bounding_box.is_some());

        let bbox = metadata.bounding_box.unwrap();
        assert_eq!(bbox.min, ChunkCoordinate::new(-1, -1, -1));
        assert_eq!(bbox.max, ChunkCoordinate::new(1, 1, 1));
    }

    #[test]
    fn test_chunk_compatibility_methods() {
        let mut world = ChunkedVoxelWorld::new();

        // Add blocks using chunked interface
        world.set_block(IVec3::new(5, 10, 15), BlockType::Grass);
        world.set_block(IVec3::new(20, 25, 30), BlockType::Stone);

        // Test compatibility methods
        let all_blocks = world.get_all_blocks();
        assert_eq!(all_blocks.len(), 2);
        assert_eq!(
            all_blocks.get(&IVec3::new(5, 10, 15)),
            Some(&BlockType::Grass)
        );
        assert_eq!(
            all_blocks.get(&IVec3::new(20, 25, 30)),
            Some(&BlockType::Stone)
        );

        let blocks_method = world.blocks();
        assert_eq!(blocks_method.len(), 2);
        assert_eq!(blocks_method, all_blocks);
    }
}
