use bevy::prelude::*;
use log::{error, info};
use rumqttc::{Client, QoS};
use std::sync::mpsc;

use super::shared_world::*;
use crate::config::MqttConfig;
use crate::profile::PlayerProfile;
use crate::world::WorldSaveData;

/// Maximum size for MQTT messages (5MB to handle large world data)
const MAX_MQTT_MESSAGE_SIZE: usize = 5242880;

/// Chunked message for large world data
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChunkedWorldData {
    pub chunk_id: String,  // Unique ID for this chunking operation
    pub chunk_index: u32,  // Index of this chunk (0-based)
    pub total_chunks: u32, // Total number of chunks
    pub data: Vec<u8>,     // Chunk data (compressed)
    pub world_id: String,  // Which world this chunk belongs to
}

/// Split world data into chunks that fit within MQTT message limits
fn chunk_world_data(
    world_data: &WorldSaveData,
    world_id: &str,
) -> Result<Vec<ChunkedWorldData>, String> {
    // Serialize and compress the world data
    let serialized = serde_json::to_vec(world_data)
        .map_err(|e| format!("Failed to serialize world data: {}", e))?;

    // Use deflate compression to reduce size
    let mut compressed = Vec::new();
    {
        use std::io::Write;
        let mut encoder =
            flate2::write::DeflateEncoder::new(&mut compressed, flate2::Compression::best());
        encoder
            .write_all(&serialized)
            .map_err(|e| format!("Failed to compress world data: {}", e))?;
        encoder
            .finish()
            .map_err(|e| format!("Failed to finish compression: {}", e))?;
    }

    info!(
        "World data compressed from {} bytes to {} bytes ({:.1}% reduction)",
        serialized.len(),
        compressed.len(),
        (1.0 - compressed.len() as f64 / serialized.len() as f64) * 100.0
    );

    // Calculate target chunk size with conservative overhead estimate
    // We need to account for: chunk_id (~50 chars), chunk_index (4 bytes),
    // total_chunks (4 bytes), world_id (~50 chars), JSON structure (~200 bytes)
    // Base64 encoding increases data size by ~33%, so be extra conservative
    let target_chunk_data_size = (MAX_MQTT_MESSAGE_SIZE as f64 * 0.60) as usize; // Use 60% of limit for safety

    let chunk_id = format!("{}_{}", world_id, chrono::Utc::now().timestamp_millis());
    let mut chunks = Vec::new();
    let mut offset = 0;
    let mut chunk_index = 0;

    // Split data into chunks and validate each one fits within MQTT limits
    while offset < compressed.len() {
        let remaining = compressed.len() - offset;
        let chunk_size = std::cmp::min(target_chunk_data_size, remaining);
        let chunk_data = compressed[offset..offset + chunk_size].to_vec();

        let test_chunk = ChunkedWorldData {
            chunk_id: chunk_id.clone(),
            chunk_index,
            total_chunks: 0, // Will be set later
            data: chunk_data.clone(),
            world_id: world_id.to_string(),
        };

        // Test if this chunk serializes within limits
        if let Ok(serialized) = serde_json::to_vec(&test_chunk) {
            if serialized.len() > MAX_MQTT_MESSAGE_SIZE {
                if chunk_size <= 100 {
                    return Err(format!(
                        "Cannot create chunk small enough: {} bytes minimum",
                        serialized.len()
                    ));
                }
                // Reduce chunk size by 10% and try again
                let new_chunk_size = (chunk_size as f64 * 0.9) as usize;
                let new_chunk_data = compressed[offset..offset + new_chunk_size].to_vec();
                chunks.push(ChunkedWorldData {
                    chunk_id: chunk_id.clone(),
                    chunk_index,
                    total_chunks: 0, // Will be set later
                    data: new_chunk_data,
                    world_id: world_id.to_string(),
                });
                offset += new_chunk_size;
            } else {
                chunks.push(test_chunk);
                offset += chunk_size;
            }
        } else {
            return Err("Failed to serialize test chunk".to_string());
        }

        chunk_index += 1;

        if chunk_index > 1000 {
            return Err(format!(
                "World too large: would require more than 1000 chunks"
            ));
        }
    }

    // Set the correct total_chunks for all chunks
    let total_chunks = chunks.len() as u32;
    for chunk in &mut chunks {
        chunk.total_chunks = total_chunks;
    }

    info!(
        "Split world data into {} chunks (target size: {}KB each)",
        total_chunks,
        target_chunk_data_size / 1024
    );
    Ok(chunks)
}

/// Resource for managing world publishing - now uses Core MQTT Service
#[derive(Resource)]
pub struct WorldPublisher {
    // Legacy field kept for compatibility but no longer used
    #[allow(dead_code)]
    pub publish_tx: std::sync::Mutex<Option<mpsc::Sender<PublishMessage>>>,
}

impl Default for WorldPublisher {
    fn default() -> Self {
        Self {
            publish_tx: std::sync::Mutex::new(None),
        }
    }
}

#[derive(Debug, Clone)]
pub enum PublishMessage {
    PublishWorld {
        world_info: SharedWorldInfo,
        world_data: WorldSaveData,
    },
    UnpublishWorld {
        world_id: String,
    },
    BroadcastChange {
        change: WorldChange,
    },
    PublishBlockChange {
        world_id: String,
        player_id: String,
        player_name: String,
        change_type: BlockChangeType,
    },
}

pub struct WorldPublisherPlugin;

impl Plugin for WorldPublisherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldPublisher>()
            .add_systems(Startup, initialize_world_publisher)
            .add_systems(
                Update,
                (handle_world_publishing, handle_world_changes_for_publishing),
            );
    }
}

fn initialize_world_publisher(
    _commands: Commands,
    _mqtt_config: Res<MqttConfig>,
    _world_publisher: ResMut<WorldPublisher>,
) {
    // No longer creates internal channels - uses Core MQTT Service directly
    info!("✅ World publisher initialized - using Core MQTT Service for publishing");
}

// Note: handle_publish_message is now handled by Core MQTT Service
// The function below is kept for reference but no longer used
#[allow(dead_code)]
fn _legacy_handle_publish_message(client: &Client, message: PublishMessage) {
    match message {
        PublishMessage::PublishWorld {
            world_info,
            world_data,
        } => {
            // Publish world info to discovery topic
            let info_topic = format!("iotcraft/worlds/{}/info", world_info.world_id);
            if let Ok(payload) = serde_json::to_string(&world_info) {
                info!(
                    "Publishing world info to topic: {} with payload: {}",
                    info_topic, payload
                );
                if let Err(e) = client.publish(&info_topic, QoS::AtLeastOnce, true, payload) {
                    error!("Failed to publish world info: {}", e);
                } else {
                    info!(
                        "Successfully published world info for {} with world_id: {}",
                        world_info.world_name, world_info.world_id
                    );
                }
            } else {
                error!(
                    "Failed to serialize world info for {}",
                    world_info.world_name
                );
            }

            // Check if world data needs chunking by measuring serialized byte length
            if let Ok(payload_bytes) = serde_json::to_vec(&world_data) {
                let payload_size = payload_bytes.len();
                if payload_size <= MAX_MQTT_MESSAGE_SIZE {
                    // Small enough to send as single message
                    let data_topic = format!("iotcraft/worlds/{}/data", world_info.world_id);
                    if let Err(e) =
                        client.publish(&data_topic, QoS::AtLeastOnce, true, payload_bytes)
                    {
                        error!("Failed to publish world data: {}", e);
                    } else {
                        info!(
                            "Published world data for {} in single message ({} bytes)",
                            world_info.world_name, payload_size
                        );
                    }
                } else {
                    // Too large, use chunking
                    info!(
                        "World data is {} bytes, needs chunking (limit: {})",
                        payload_bytes.len(),
                        MAX_MQTT_MESSAGE_SIZE
                    );
                    match chunk_world_data(&world_data, &world_info.world_id) {
                        Ok(chunks) => {
                            let total_chunks = chunks.len();
                            info!("Publishing large world data in {} chunks", total_chunks);

                            // Send each chunk with size validation
                            for chunk in &chunks {
                                let chunk_topic =
                                    format!("iotcraft/worlds/{}/data/chunk", world_info.world_id);
                                if let Ok(chunk_payload_bytes) = serde_json::to_vec(chunk) {
                                    let chunk_size = chunk_payload_bytes.len();
                                    if chunk_size > MAX_MQTT_MESSAGE_SIZE {
                                        error!(
                                            "Chunk {}/{} is {} bytes, exceeds limit of {}",
                                            chunk.chunk_index + 1,
                                            chunk.total_chunks,
                                            chunk_size,
                                            MAX_MQTT_MESSAGE_SIZE
                                        );
                                        break;
                                    }

                                    if let Err(e) = client.publish(
                                        &chunk_topic,
                                        QoS::AtLeastOnce,
                                        false,
                                        chunk_payload_bytes,
                                    ) {
                                        error!(
                                            "Failed to publish chunk {}/{}: {}",
                                            chunk.chunk_index + 1,
                                            chunk.total_chunks,
                                            e
                                        );
                                        break;
                                    } else {
                                        info!(
                                            "Published chunk {}/{} ({} bytes)",
                                            chunk.chunk_index + 1,
                                            chunk.total_chunks,
                                            chunk_size
                                        );
                                    }
                                } else {
                                    error!(
                                        "Failed to serialize chunk {}/{}",
                                        chunk.chunk_index + 1,
                                        chunk.total_chunks
                                    );
                                    break;
                                }
                            }

                            info!(
                                "Successfully published all {} chunks for {}",
                                total_chunks, world_info.world_name
                            );
                        }
                        Err(e) => {
                            error!("Failed to chunk world data: {}", e);
                        }
                    }
                }
            }
        }
        PublishMessage::UnpublishWorld { world_id } => {
            // Remove world from discovery by publishing empty message with retain
            let info_topic = format!("iotcraft/worlds/{}/info", world_id);
            let data_topic = format!("iotcraft/worlds/{}/data", world_id);

            if let Err(e) = client.publish(&info_topic, QoS::AtLeastOnce, true, "") {
                error!("Failed to unpublish world info: {}", e);
            }
            if let Err(e) = client.publish(&data_topic, QoS::AtLeastOnce, true, "") {
                error!("Failed to unpublish world data: {}", e);
            }

            info!("Unpublished world {}", world_id);
        }
        PublishMessage::BroadcastChange { change } => {
            let change_topic = format!("iotcraft/worlds/{}/changes", change.world_id);
            if let Ok(payload) = serde_json::to_string(&change) {
                if let Err(e) = client.publish(&change_topic, QoS::AtLeastOnce, false, payload) {
                    error!("Failed to broadcast world change: {}", e);
                }
            }
        }
        PublishMessage::PublishBlockChange {
            world_id,
            player_id,
            player_name,
            change_type,
        } => {
            info!(
                "🚀 MQTT Publisher: Received block change to publish - world: {}, player: {} ({}), change: {:?}",
                world_id, player_name, player_id, change_type
            );

            let topic = match &change_type {
                BlockChangeType::Placed { .. } => {
                    format!("iotcraft/worlds/{}/state/blocks/placed", world_id)
                }
                BlockChangeType::Removed { .. } => {
                    format!("iotcraft/worlds/{}/state/blocks/removed", world_id)
                }
            };

            info!("📡 Publishing to MQTT topic: {}", topic);

            let change_message = serde_json::json!({
                "player_id": player_id,
                "player_name": player_name,
                "timestamp": chrono::Utc::now().timestamp_millis(),
                "change": change_type
            });

            match serde_json::to_string(&change_message) {
                Ok(payload) => {
                    info!("📦 Serialized payload: {}", payload);

                    match client.publish(&topic, QoS::AtLeastOnce, false, payload) {
                        Ok(()) => {
                            info!(
                                "✅ Successfully published block change to MQTT topic {}",
                                topic
                            );
                        }
                        Err(e) => {
                            error!("❌ Failed to publish block change to {}: {}", topic, e);
                        }
                    }
                }
                Err(e) => {
                    error!("❌ Failed to serialize block change message: {}", e);
                }
            }
        }
    }
}

fn handle_world_publishing(
    mut publish_events: EventReader<PublishWorldEvent>,
    mut unpublish_events: EventReader<UnpublishWorldEvent>,
    mqtt_outgoing_tx: Option<Res<crate::mqtt::core_service::MqttOutgoingTx>>,
    current_world: Option<Res<crate::world::CurrentWorld>>,
    voxel_world: Res<crate::environment::VoxelWorld>,
    inventory: Res<crate::inventory::PlayerInventory>,
    camera_query: Query<&Transform, With<crate::camera_controllers::CameraController>>,
    player_profile: Res<PlayerProfile>,
    multiplayer_mode: Res<MultiplayerMode>,
) {
    // RE-ENABLED: World data sharing for multiplayer synchronization
    for event in publish_events.read() {
        info!(
            "🌍 Publishing world '{}' with full data sharing enabled",
            event.world_name
        );

        if let Some(mqtt_tx) = &mqtt_outgoing_tx {
            // Generate world_id from world_name and timestamp since PublishWorldEvent doesn't include it
            let world_id = match &*multiplayer_mode {
                MultiplayerMode::HostingWorld { world_id, .. } => world_id.clone(),
                _ => format!("{}_{}", event.world_name, chrono::Utc::now().timestamp()),
            };

            // Create world info for discovery
            let world_info = SharedWorldInfo {
                world_id: world_id.clone(),
                world_name: event.world_name.clone(),
                description: current_world
                    .as_ref()
                    .map(|w| w.metadata.description.clone())
                    .unwrap_or_else(|| "Shared world".to_string()),
                host_player: player_profile.player_id.clone(),
                host_name: player_profile.player_name.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
                last_updated: chrono::Utc::now().to_rfc3339(),
                player_count: 1,
                max_players: event.max_players,
                is_public: event.is_public,
                version: "1.0.0".to_string(),
            };

            // Create world data from current voxel world
            use crate::world::{VoxelBlockData, WorldMetadata, WorldSaveData};

            let blocks: Vec<VoxelBlockData> = voxel_world
                .blocks
                .iter()
                .map(|(pos, block_type)| VoxelBlockData {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                    block_type: *block_type,
                })
                .collect();

            // Get player position from camera
            let player_position = camera_query
                .iter()
                .next()
                .map(|transform| transform.translation)
                .unwrap_or(bevy::math::Vec3::ZERO);

            // Create metadata
            let metadata = current_world
                .as_ref()
                .map(|w| w.metadata.clone())
                .unwrap_or_else(|| WorldMetadata {
                    name: event.world_name.clone(),
                    description: "Shared world".to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    last_played: chrono::Utc::now().to_rfc3339(),
                    version: "1.0.0".to_string(),
                });

            let world_data = WorldSaveData {
                metadata,
                blocks,
                player_position,
                player_rotation: bevy::math::Quat::IDENTITY,
                inventory: inventory.clone(),
            };

            info!(
                "📤 Publishing world '{}' with {} blocks via MQTT",
                event.world_name,
                world_data.blocks.len()
            );

            // Publish world info
            let info_topic = format!("iotcraft/worlds/{}/info", world_id);
            if let Ok(info_payload) = serde_json::to_string(&world_info) {
                let info_msg = crate::mqtt::core_service::OutgoingMqttMessage::PublishWorldInfo {
                    topic: info_topic,
                    payload: info_payload,
                    retain: true,
                };

                if let Ok(tx) = mqtt_tx.0.lock() {
                    if let Err(e) = tx.send(info_msg) {
                        error!("Failed to publish world info: {}", e);
                    } else {
                        info!("✅ Published world info for '{}'", event.world_name);
                    }
                }
            }

            // Publish world data
            let data_topic = format!("iotcraft/worlds/{}/data", world_id);
            if let Ok(data_payload) = serde_json::to_vec(&world_data) {
                // Check if we need to use chunking for large worlds
                if data_payload.len() > MAX_MQTT_MESSAGE_SIZE {
                    info!(
                        "World data ({} bytes) exceeds MQTT limit, using chunking",
                        data_payload.len()
                    );

                    match chunk_world_data(&world_data, &world_id) {
                        Ok(chunks) => {
                            info!("Publishing world data in {} chunks", chunks.len());

                            for chunk in chunks {
                                if let Ok(chunk_payload) = serde_json::to_vec(&chunk) {
                                    let chunk_topic =
                                        format!("iotcraft/worlds/{}/data/chunk", world_id);
                                    let chunk_msg = crate::mqtt::core_service::OutgoingMqttMessage::PublishWorldChunk {
                                        topic: chunk_topic,
                                        payload: chunk_payload,
                                    };

                                    if let Ok(tx) = mqtt_tx.0.lock() {
                                        if let Err(e) = tx.send(chunk_msg) {
                                            error!("Failed to publish world chunk: {}", e);
                                            break;
                                        }
                                    }
                                } else {
                                    error!("Failed to serialize world chunk");
                                    break;
                                }
                            }

                            info!(
                                "✅ Published world data for '{}' in chunks",
                                event.world_name
                            );
                        }
                        Err(e) => {
                            error!("Failed to chunk world data: {}", e);
                        }
                    }
                } else {
                    // Small enough for single message
                    let data_msg = crate::mqtt::core_service::OutgoingMqttMessage::GenericPublish {
                        topic: data_topic,
                        payload: String::from_utf8_lossy(&data_payload).to_string(),
                        qos: rumqttc::QoS::AtLeastOnce,
                        retain: true,
                    };

                    if let Ok(tx) = mqtt_tx.0.lock() {
                        if let Err(e) = tx.send(data_msg) {
                            error!("Failed to publish world data: {}", e);
                        } else {
                            info!(
                                "✅ Published world data for '{}' ({} bytes)",
                                event.world_name,
                                data_payload.len()
                            );
                        }
                    }
                }
            }
        } else {
            error!("MQTT service not available for world publishing");
        }
    }

    // Handle unpublish events
    for event in unpublish_events.read() {
        info!("🗑️ Unpublishing world '{}'", event.world_id);

        if let Some(mqtt_tx) = &mqtt_outgoing_tx {
            // Clear world info and data by publishing empty messages with retain
            let info_topic = format!("iotcraft/worlds/{}/info", event.world_id);
            let data_topic = format!("iotcraft/worlds/{}/data", event.world_id);

            let info_msg = crate::mqtt::core_service::OutgoingMqttMessage::GenericPublish {
                topic: info_topic,
                payload: "".to_string(),
                qos: rumqttc::QoS::AtLeastOnce,
                retain: true,
            };

            let data_msg = crate::mqtt::core_service::OutgoingMqttMessage::GenericPublish {
                topic: data_topic,
                payload: "".to_string(),
                qos: rumqttc::QoS::AtLeastOnce,
                retain: true,
            };

            if let Ok(tx) = mqtt_tx.0.lock() {
                let _ = tx.send(info_msg);
                let _ = tx.send(data_msg);
                info!("✅ Unpublished world '{}'", event.world_id);
            }
        }
    }
}

fn handle_world_changes_for_publishing(
    mut change_events: EventReader<WorldChangeEvent>,
    mqtt_outgoing_tx: Option<Res<crate::mqtt::core_service::MqttOutgoingTx>>,
    multiplayer_mode: Res<MultiplayerMode>,
) {
    for event in change_events.read() {
        match &*multiplayer_mode {
            MultiplayerMode::HostingWorld { world_id, .. } => {
                // Broadcast changes when hosting
                if let Some(mqtt_tx) = &mqtt_outgoing_tx {
                    let change_topic = format!("iotcraft/worlds/{}/changes", world_id);
                    if let Ok(payload) = serde_json::to_string(&event.change) {
                        let change_msg =
                            crate::mqtt::core_service::OutgoingMqttMessage::GenericPublish {
                                topic: change_topic.clone(),
                                payload,
                                qos: rumqttc::QoS::AtLeastOnce,
                                retain: false,
                            };

                        if let Ok(tx) = mqtt_tx.0.lock() {
                            if let Err(e) = tx.send(change_msg) {
                                error!(
                                    "Failed to broadcast world change via Core MQTT Service: {}",
                                    e
                                );
                            } else {
                                info!(
                                    "📡 Broadcasted world change to {} via Core MQTT Service",
                                    change_topic
                                );
                            }
                        }
                    }
                } else {
                    error!("Core MQTT Service not available for world change broadcasting");
                }
            }
            MultiplayerMode::JoinedWorld { world_id, .. } => {
                // Forward changes to host when joined
                if let Some(mqtt_tx) = &mqtt_outgoing_tx {
                    let change_topic = format!("iotcraft/worlds/{}/changes", world_id);
                    if let Ok(payload) = serde_json::to_string(&event.change) {
                        let change_msg =
                            crate::mqtt::core_service::OutgoingMqttMessage::GenericPublish {
                                topic: change_topic.clone(),
                                payload,
                                qos: rumqttc::QoS::AtLeastOnce,
                                retain: false,
                            };

                        if let Ok(tx) = mqtt_tx.0.lock() {
                            if let Err(e) = tx.send(change_msg) {
                                error!("Failed to send world change via Core MQTT Service: {}", e);
                            } else {
                                info!(
                                    "📡 Sent world change to {} via Core MQTT Service",
                                    change_topic
                                );
                            }
                        }
                    }
                } else {
                    error!("Core MQTT Service not available for world change forwarding");
                }
            }
            MultiplayerMode::SinglePlayer => {
                // No broadcasting in single player mode
            }
        }
    }
}
