use bevy::prelude::*;
use log::{error, info};
use std::sync::Mutex;
use std::sync::mpsc;

use super::chunk_events::*;
use super::chunk_types::*;
use crate::multiplayer::MultiplayerMode;

/// Resource for MQTT chunk publishing
#[derive(Resource)]
pub struct ChunkMqttPublisher {
    pub publish_tx: Mutex<Option<mpsc::Sender<ChunkMqttMessage>>>,
}

impl Default for ChunkMqttPublisher {
    fn default() -> Self {
        Self {
            publish_tx: Mutex::new(None),
        }
    }
}

/// MQTT messages for chunk synchronization
#[derive(Debug, Clone)]
pub enum ChunkMqttMessage {
    /// Publish chunk data to MQTT
    PublishChunkData {
        world_id: String,
        chunk_data: ChunkData,
    },
    /// Publish block change within a chunk
    PublishBlockChange {
        world_id: String,
        block_change: ChunkBlockChange,
    },
    /// Publish chunk metadata
    PublishChunkMetadata {
        world_id: String,
        metadata: ChunkMetadata,
    },
    /// Publish world metadata
    PublishWorldMetadata {
        world_id: String,
        metadata: ChunkedWorldMetadata,
    },
    /// Request chunk data from other players
    RequestChunkData {
        world_id: String,
        chunk_coordinate: ChunkCoordinate,
        requester_player_id: String,
    },
}

/// Resource for receiving MQTT chunk messages
#[derive(Resource)]
pub struct ChunkMqttReceiver {
    pub message_rx: Mutex<Option<mpsc::Receiver<ChunkMqttResponse>>>,
}

impl Default for ChunkMqttReceiver {
    fn default() -> Self {
        Self {
            message_rx: Mutex::new(None),
        }
    }
}

/// Responses from MQTT chunk system
#[derive(Debug, Clone)]
pub enum ChunkMqttResponse {
    /// Chunk data received from MQTT
    ChunkDataReceived {
        world_id: String,
        chunk_data: ChunkData,
        sender_player_id: String,
    },
    /// Block change received from MQTT
    BlockChangeReceived {
        world_id: String,
        block_change: ChunkBlockChange,
    },
    /// Chunk metadata received from MQTT
    ChunkMetadataReceived {
        world_id: String,
        metadata: ChunkMetadata,
    },
    /// World metadata received from MQTT
    WorldMetadataReceived {
        world_id: String,
        metadata: ChunkedWorldMetadata,
    },
    /// Chunk data request received from MQTT
    ChunkRequestReceived {
        world_id: String,
        chunk_coordinate: ChunkCoordinate,
        requester_player_id: String,
    },
}

/// Plugin for MQTT chunk synchronization
pub struct ChunkMqttPlugin;

impl Plugin for ChunkMqttPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkMqttPublisher>()
            .init_resource::<ChunkMqttReceiver>()
            .add_systems(
                Update,
                (
                    handle_chunk_change_events,
                    handle_chunk_mqtt_responses,
                    handle_world_metadata_publish_events,
                    handle_chunk_data_request_events,
                ),
            );
    }
}

/// System to handle chunk change events and publish to MQTT
fn handle_chunk_change_events(
    mut chunk_events: EventReader<ChunkChangeEvent>,
    chunk_publisher: Res<ChunkMqttPublisher>,
    multiplayer_mode: Res<MultiplayerMode>,
) {
    for event in chunk_events.read() {
        // Only publish in multiplayer mode
        match &*multiplayer_mode {
            MultiplayerMode::HostingWorld { world_id, .. }
            | MultiplayerMode::JoinedWorld { world_id, .. } => {
                if let Some(tx) = chunk_publisher.publish_tx.lock().unwrap().as_ref() {
                    let message = match &event.change_type {
                        ChunkChangeType::BlockPlaced {
                            position,
                            block_type,
                        } => ChunkMqttMessage::PublishBlockChange {
                            world_id: world_id.clone(),
                            block_change: ChunkBlockChange {
                                position: *position,
                                change_type: ChunkBlockChangeType::Placed {
                                    block_type: *block_type,
                                },
                                timestamp: now_timestamp(),
                                player_id: event.player_id.clone(),
                                chunk_coordinate: event.chunk_coordinate.clone(),
                            },
                        },
                        ChunkChangeType::BlockRemoved { position } => {
                            ChunkMqttMessage::PublishBlockChange {
                                world_id: world_id.clone(),
                                block_change: ChunkBlockChange {
                                    position: *position,
                                    change_type: ChunkBlockChangeType::Removed,
                                    timestamp: now_timestamp(),
                                    player_id: event.player_id.clone(),
                                    chunk_coordinate: event.chunk_coordinate.clone(),
                                },
                            }
                        }
                        ChunkChangeType::ChunkLoaded { chunk_data } => {
                            ChunkMqttMessage::PublishChunkData {
                                world_id: world_id.clone(),
                                chunk_data: chunk_data.clone(),
                            }
                        }
                        _ => continue, // Skip other change types for now
                    };

                    if let Err(e) = tx.send(message) {
                        error!("Failed to send chunk MQTT message: {}", e);
                    }
                }
            }
            MultiplayerMode::SinglePlayer => {
                // Don't publish in single player mode
            }
        }
    }
}

/// System to handle MQTT responses and update local world
fn handle_chunk_mqtt_responses(
    chunk_receiver: Res<ChunkMqttReceiver>,
    mut chunk_data_events: EventWriter<ChunkDataReceivedEvent>,
    mut chunk_world: ResMut<crate::environment::ChunkedVoxelWorld>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    if let Some(rx) = chunk_receiver.message_rx.lock().unwrap().as_ref() {
        while let Ok(response) = rx.try_recv() {
            match response {
                ChunkMqttResponse::ChunkDataReceived {
                    world_id,
                    chunk_data,
                    sender_player_id,
                } => {
                    info!(
                        "Received chunk data for chunk {:?} from player {}",
                        chunk_data.coordinate, sender_player_id
                    );

                    // Update local world with received chunk data
                    let chunk_coord = chunk_data.coordinate.clone();
                    chunk_world
                        .chunks
                        .insert(chunk_coord.clone(), chunk_data.clone());
                    chunk_world.load_chunk(chunk_coord);

                    // Spawn visual blocks for the received chunk
                    spawn_chunk_blocks(
                        &chunk_data,
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        &asset_server,
                    );

                    // Send event for further processing
                    chunk_data_events.send(ChunkDataReceivedEvent {
                        world_id,
                        chunk_data,
                        sender_player_id,
                    });
                }
                ChunkMqttResponse::BlockChangeReceived {
                    world_id,
                    block_change,
                } => {
                    info!(
                        "Received block change: {:?} at {:?}",
                        block_change.change_type, block_change.position
                    );

                    // Apply block change to local world
                    match block_change.change_type {
                        ChunkBlockChangeType::Placed { block_type } => {
                            chunk_world.set_block(block_change.position, block_type);

                            // Spawn visual block
                            spawn_single_block(
                                block_change.position,
                                block_type,
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &asset_server,
                            );
                        }
                        ChunkBlockChangeType::Removed => {
                            chunk_world.remove_block(&block_change.position);

                            // Remove visual block (simplified - in reality we'd need to track entities)
                            // TODO: Implement proper entity tracking for block removal
                        }
                    }
                }
                ChunkMqttResponse::ChunkRequestReceived {
                    world_id,
                    chunk_coordinate,
                    requester_player_id,
                } => {
                    info!(
                        "Received chunk request for {:?} from player {}",
                        chunk_coordinate, requester_player_id
                    );

                    // If we have the chunk, send it
                    if let Some(chunk_data) = chunk_world.get_chunk(&chunk_coordinate) {
                        if let Some(tx) = chunk_receiver.message_rx.lock().unwrap().as_ref() {
                            let message = ChunkMqttMessage::PublishChunkData {
                                world_id,
                                chunk_data: chunk_data.clone(),
                            };

                            // We need to send this through the publisher, not receiver
                            // This is a bit of a design issue - we should restructure
                            info!(
                                "Would send chunk data for {:?} to {}",
                                chunk_coordinate, requester_player_id
                            );
                        }
                    }
                }
                _ => {
                    // Handle other response types
                }
            }
        }
    }
}

/// System to handle world metadata publish events
fn handle_world_metadata_publish_events(
    mut metadata_events: EventReader<PublishWorldMetadataEvent>,
    chunk_publisher: Res<ChunkMqttPublisher>,
) {
    for event in metadata_events.read() {
        if let Some(tx) = chunk_publisher.publish_tx.lock().unwrap().as_ref() {
            let message = ChunkMqttMessage::PublishWorldMetadata {
                world_id: event.world_id.clone(),
                metadata: event.metadata.clone(),
            };

            if let Err(e) = tx.send(message) {
                error!("Failed to send world metadata MQTT message: {}", e);
            }
        }
    }
}

/// System to handle chunk data request events
fn handle_chunk_data_request_events(
    mut request_events: EventReader<RequestChunkDataEvent>,
    chunk_publisher: Res<ChunkMqttPublisher>,
) {
    for event in request_events.read() {
        if let Some(tx) = chunk_publisher.publish_tx.lock().unwrap().as_ref() {
            let message = ChunkMqttMessage::RequestChunkData {
                world_id: event.world_id.clone(),
                chunk_coordinate: event.chunk_coordinate.clone(),
                requester_player_id: event.requester_player_id.clone(),
            };

            if let Err(e) = tx.send(message) {
                error!("Failed to send chunk request MQTT message: {}", e);
            }
        }
    }
}

/// Helper function to spawn visual blocks for a chunk
fn spawn_chunk_blocks(
    chunk_data: &ChunkData,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &AssetServer,
) {
    let cube_mesh = meshes.add(Cuboid::new(
        crate::environment::CUBE_SIZE,
        crate::environment::CUBE_SIZE,
        crate::environment::CUBE_SIZE,
    ));

    for (position, block_type) in &chunk_data.blocks {
        spawn_single_block_with_mesh(
            *position,
            *block_type,
            cube_mesh.clone(),
            commands,
            materials,
            asset_server,
        );
    }
}

/// Helper function to spawn a single visual block
fn spawn_single_block(
    position: IVec3,
    block_type: crate::environment::BlockType,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &AssetServer,
) {
    let cube_mesh = meshes.add(Cuboid::new(
        crate::environment::CUBE_SIZE,
        crate::environment::CUBE_SIZE,
        crate::environment::CUBE_SIZE,
    ));

    spawn_single_block_with_mesh(
        position,
        block_type,
        cube_mesh,
        commands,
        materials,
        asset_server,
    );
}

/// Helper function to spawn a single visual block with existing mesh
fn spawn_single_block_with_mesh(
    position: IVec3,
    block_type: crate::environment::BlockType,
    cube_mesh: Handle<Mesh>,
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &AssetServer,
) {
    use crate::environment::BlockType;

    let material = match block_type {
        BlockType::Water => materials.add(StandardMaterial {
            base_color: Color::srgba(0.0, 0.35, 0.9, 0.6),
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        _ => {
            let texture_path = match block_type {
                BlockType::Grass => "textures/grass.webp",
                BlockType::Dirt => "textures/dirt.webp",
                BlockType::Stone => "textures/stone.webp",
                BlockType::QuartzBlock => "textures/quartz_block.webp",
                BlockType::GlassPane => "textures/glass_pane.webp",
                BlockType::CyanTerracotta => "textures/cyan_terracotta.webp",
                _ => "textures/stone.webp",
            };
            let texture: Handle<Image> = asset_server.load(texture_path);
            materials.add(StandardMaterial {
                base_color_texture: Some(texture),
                ..default()
            })
        }
    };

    commands.spawn((
        Mesh3d(cube_mesh),
        MeshMaterial3d(material),
        Transform::from_translation(Vec3::new(
            position.x as f32,
            position.y as f32,
            position.z as f32,
        )),
        crate::environment::VoxelBlock { position },
    ));
}

/// Helper function to generate MQTT topics for chunks
pub fn generate_chunk_topic(
    world_id: &str,
    chunk_coord: &ChunkCoordinate,
    topic_type: &str,
) -> String {
    format!(
        "iotcraft/worlds/{}/chunks/{}/{}",
        world_id,
        chunk_coord.to_topic_path(),
        topic_type
    )
}

/// Helper function to generate world metadata topic
pub fn generate_world_metadata_topic(world_id: &str) -> String {
    format!("iotcraft/worlds/{}/metadata", world_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_generation() {
        let chunk_coord = ChunkCoordinate::new(1, -2, 3);
        let topic = generate_chunk_topic("world123", &chunk_coord, "data");
        assert_eq!(topic, "iotcraft/worlds/world123/chunks/1/-2/3/data");

        let metadata_topic = generate_world_metadata_topic("world123");
        assert_eq!(metadata_topic, "iotcraft/worlds/world123/metadata");
    }
}
