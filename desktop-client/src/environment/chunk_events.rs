use bevy::prelude::*;

use super::BlockType;
use super::chunk_types::*;

/// Events for chunk-based MQTT synchronization
#[derive(Event)]
pub struct ChunkChangeEvent {
    pub chunk_coordinate: ChunkCoordinate,
    pub change_type: ChunkChangeType,
    pub world_id: String,
    pub player_id: String,
    pub player_name: String,
}

/// Types of chunk changes
#[derive(Debug, Clone)]
pub enum ChunkChangeType {
    /// A block was placed in the chunk
    BlockPlaced {
        position: IVec3,
        block_type: BlockType,
    },
    /// A block was removed from the chunk  
    BlockRemoved { position: IVec3 },
    /// An entire chunk was loaded/created
    ChunkLoaded { chunk_data: ChunkData },
    /// A chunk was unloaded (but data may be retained)
    ChunkUnloaded,
    /// Request to load a chunk from MQTT
    ChunkLoadRequested,
}

/// Event for publishing world metadata
#[derive(Event)]
pub struct PublishWorldMetadataEvent {
    pub world_id: String,
    pub metadata: ChunkedWorldMetadata,
}

/// Event for requesting chunk data from MQTT
#[derive(Event)]
pub struct RequestChunkDataEvent {
    pub world_id: String,
    pub chunk_coordinate: ChunkCoordinate,
    pub requester_player_id: String,
}

/// Event for receiving chunk data from MQTT
#[derive(Event)]
pub struct ChunkDataReceivedEvent {
    pub world_id: String,
    pub chunk_data: ChunkData,
    pub sender_player_id: String,
}

/// Event for chunk metadata updates
#[derive(Event)]
pub struct ChunkMetadataUpdateEvent {
    pub world_id: String,
    pub metadata: ChunkMetadata,
}

/// Event to signal that chunks should be loaded around a player
#[derive(Event)]
pub struct LoadChunksAroundPlayerEvent {
    pub player_id: String,
    pub position: Vec3,
    pub load_radius: i32, // Radius in chunks
}

/// Event to signal that chunks should be unloaded
#[derive(Event)]
pub struct UnloadChunksEvent {
    pub chunk_coordinates: Vec<ChunkCoordinate>,
    pub world_id: String,
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
    pub fn new() -> Self {
        Self {
            loading_chunks: std::collections::HashSet::new(),
            requested_chunks: std::collections::HashMap::new(),
            chunk_request_timeout: std::time::Duration::from_secs(10),
        }
    }

    pub fn is_loading(&self, chunk: &ChunkCoordinate) -> bool {
        self.loading_chunks.contains(chunk)
    }

    pub fn start_loading(&mut self, chunk: ChunkCoordinate) {
        self.loading_chunks.insert(chunk.clone());
        self.requested_chunks
            .insert(chunk, std::time::Instant::now());
    }

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
            .add_event::<PublishWorldMetadataEvent>()
            .add_event::<RequestChunkDataEvent>()
            .add_event::<ChunkDataReceivedEvent>()
            .add_event::<ChunkMetadataUpdateEvent>()
            .add_event::<LoadChunksAroundPlayerEvent>()
            .add_event::<UnloadChunksEvent>()
            // Systems
            .add_systems(Update, (chunk_loader_system, cleanup_chunk_requests_system));
    }
}

/// System to handle chunk loading around entities with ChunkLoader component
fn chunk_loader_system(
    mut chunk_loaders: Query<(&Transform, &mut ChunkLoader)>,
    mut load_events: EventWriter<LoadChunksAroundPlayerEvent>,
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

            // Send event for MQTT synchronization
            load_events.send(LoadChunksAroundPlayerEvent {
                player_id: "local_player".to_string(), // TODO: Get actual player ID
                position: transform.translation,
                load_radius: loader.load_radius,
            });
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
    fn test_chunk_loading_state() {
        let mut state = ChunkLoadingState::new();
        let chunk = ChunkCoordinate::new(0, 0, 0);

        assert!(!state.is_loading(&chunk));

        state.start_loading(chunk.clone());
        assert!(state.is_loading(&chunk));

        state.finish_loading(&chunk);
        assert!(!state.is_loading(&chunk));
    }

    #[test]
    fn test_chunk_loader_default() {
        let loader = ChunkLoader::default();
        assert_eq!(loader.load_radius, 2);
        assert!(loader.last_chunk_position.is_none());
    }
}
