use bevy::prelude::*;

use super::chunk_types::*;

/// Events for chunk-based MQTT synchronization
#[derive(Event, BufferedEvent)]
pub struct ChunkChangeEvent {
    // pub chunk_coordinate: ChunkCoordinate,
    pub change_type: ChunkChangeType,
    // pub player_id: String,
}

/// Types of chunk changes
#[derive(Debug, Clone)]
pub enum ChunkChangeType {
    /// Placeholder for future chunk change types
    _Unused,
}

/// Component to mark entities that should trigger chunk loading
#[derive(Component)]
pub struct ChunkLoader {
    pub load_radius: i32,
    pub last_chunk_position: Option<ChunkCoordinate>,
}

impl Default for ChunkLoader {
    fn default() -> Self {
        Self {
            load_radius: 2, // Load chunks within 2 chunk radius (5x5x5 chunks)
            last_chunk_position: None,
        }
    }
}

/// Resource to track chunk loading state
#[derive(Resource, Default)]
pub struct ChunkLoadingState {
    pub loading_chunks: std::collections::HashSet<ChunkCoordinate>,
    pub requested_chunks: std::collections::HashMap<ChunkCoordinate, std::time::Instant>,
    pub chunk_request_timeout: std::time::Duration,
}

impl ChunkLoadingState {
    pub fn finish_loading(&mut self, chunk: &ChunkCoordinate) {
        self.loading_chunks.remove(chunk);
        self.requested_chunks.remove(chunk);
    }

    pub fn get_timed_out_requests(&self) -> Vec<ChunkCoordinate> {
        let now = std::time::Instant::now();
        self.requested_chunks
            .iter()
            .filter(|&(_, &time)| now.duration_since(time) > self.chunk_request_timeout)
            .map(|(coord, _)| coord.clone())
            .collect()
    }

    pub fn cleanup_timed_out_requests(&mut self) {
        let timed_out = self.get_timed_out_requests();
        for chunk in timed_out {
            self.finish_loading(&chunk);
        }
    }
}

/// Plugin for chunk-based events and systems
pub struct ChunkEventsPlugin;

impl Plugin for ChunkEventsPlugin {
    fn build(&self, app: &mut App) {
        app
            // Resources
            .init_resource::<ChunkLoadingState>()
            // Events
            .add_event::<ChunkChangeEvent>()
            // Systems
            .add_systems(Update, (chunk_loader_system, cleanup_chunk_requests_system));
    }
}

/// System to handle chunk loading around entities with ChunkLoader component
fn chunk_loader_system(
    mut chunk_loaders: Query<(&Transform, &mut ChunkLoader)>,
    mut chunked_world: ResMut<crate::environment::ChunkedVoxelWorld>,
) {
    for (transform, mut loader) in chunk_loaders.iter_mut() {
        let current_chunk = ChunkCoordinate::from_block_position(IVec3::new(
            transform.translation.x as i32,
            transform.translation.y as i32,
            transform.translation.z as i32,
        ));

        // Check if we've moved to a different chunk
        let should_load = match &loader.last_chunk_position {
            Some(last_pos) => *last_pos != current_chunk,
            None => true,
        };

        if should_load {
            loader.last_chunk_position = Some(current_chunk.clone());

            // Load chunks in radius around current position
            for x in (current_chunk.x - loader.load_radius)..=(current_chunk.x + loader.load_radius)
            {
                for y in
                    (current_chunk.y - loader.load_radius)..=(current_chunk.y + loader.load_radius)
                {
                    for z in (current_chunk.z - loader.load_radius)
                        ..=(current_chunk.z + loader.load_radius)
                    {
                        let chunk_coord = ChunkCoordinate::new(x, y, z);
                        chunked_world.load_chunk(chunk_coord);
                    }
                }
            }

            // Chunk loading completed (no event needed as no system consumes it)
        }
    }
}

/// System to cleanup timed-out chunk requests
fn cleanup_chunk_requests_system(mut loading_state: ResMut<ChunkLoadingState>) {
    loading_state.cleanup_timed_out_requests();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_loader_default() {
        let loader = ChunkLoader::default();
        assert_eq!(loader.load_radius, 2);
        assert!(loader.last_chunk_position.is_none());
    }
}
