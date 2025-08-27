use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use log::{error, info, warn};
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep as async_sleep, timeout};

use super::mqtt_utils::generate_unique_client_id;
use super::shared_world::*;
use super::world_publisher::ChunkedWorldData; // Import from publisher
use crate::config::MqttConfig;
use crate::world::*;

/// Data structure to hold information about the last received message on a topic
#[derive(Debug, Clone)]
pub struct LastMessage {
    pub content: String,
    pub timestamp: u64, // Unix timestamp in seconds
}

/// Resource for managing world discovery
#[derive(Resource)]
pub struct WorldDiscovery {
    pub discovery_tx: std::sync::Mutex<Option<mpsc::Sender<DiscoveryMessage>>>,
    pub world_rx: std::sync::Mutex<Option<mpsc::Receiver<DiscoveryResponse>>>,
    pub subscribed_topics: std::sync::Mutex<Vec<String>>,
    pub connection_status: std::sync::Mutex<String>,
    pub last_messages: std::sync::Mutex<HashMap<String, LastMessage>>,
}

impl Default for WorldDiscovery {
    fn default() -> Self {
        Self {
            discovery_tx: std::sync::Mutex::new(None),
            world_rx: std::sync::Mutex::new(None),
            subscribed_topics: std::sync::Mutex::new(Vec::new()),
            connection_status: std::sync::Mutex::new("Disconnected".to_string()),
            last_messages: std::sync::Mutex::new(HashMap::new()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DiscoveryMessage {
    RefreshWorlds,
}

#[derive(Debug, Clone)]
pub enum DiscoveryResponse {
    WorldListUpdated {
        worlds: HashMap<String, SharedWorldInfo>,
    },
    WorldDataReceived {
        world_id: String,
        world_data: WorldSaveData,
    },
    WorldChangeReceived {
        change: WorldChange,
    },
    BlockChangeReceived {
        world_id: String,
        player_id: String,
        player_name: String,
        change_type: super::shared_world::BlockChangeType,
    },
    LastMessagesUpdated {
        last_messages: HashMap<String, LastMessage>,
    },
}

pub struct WorldDiscoveryPlugin;

impl Plugin for WorldDiscoveryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldDiscovery>()
            .add_systems(Startup, initialize_world_discovery)
            .add_systems(
                Update,
                (
                    handle_discovery_requests,
                    process_discovery_responses,
                    auto_refresh_worlds,
                ),
            );
    }
}

fn initialize_world_discovery(
    _commands: Commands,
    mqtt_config: Res<MqttConfig>,
    world_discovery: ResMut<WorldDiscovery>,
) {
    let (discovery_tx, discovery_rx) = mpsc::channel::<DiscoveryMessage>();
    let (response_tx, response_rx) = mpsc::channel::<DiscoveryResponse>();

    // Store channels in the resource
    *world_discovery.discovery_tx.lock().unwrap() = Some(discovery_tx);
    *world_discovery.world_rx.lock().unwrap() = Some(response_rx);

    let mqtt_host = mqtt_config.host.clone();
    let mqtt_port = mqtt_config.port;

    // Spawn async discovery task using Bevy's task pool
    let task_pool = AsyncComputeTaskPool::get();
    task_pool.spawn(async move {
        info!("Starting world discovery async task...");

        // Main discovery loop with reconnection handling
        loop {
            let client_id = generate_unique_client_id("iotcraft-world-discovery");
            info!("World discovery reconnecting with client ID: {}", client_id);
            let mut opts = MqttOptions::new(&client_id, &mqtt_host, mqtt_port);
            opts.set_keep_alive(Duration::from_secs(30));
            opts.set_clean_session(false);
            opts.set_max_packet_size(1048576, 1048576); // Set max packet size to 1MB to match server

            let (client, mut eventloop) = AsyncClient::new(opts, 10);
            let mut connected = false;
            let mut subscribed = false;
            let mut world_cache: HashMap<String, SharedWorldInfo> = HashMap::new();
            // Track chunks for reassembly
            let mut chunk_cache: HashMap<String, HashMap<u32, ChunkedWorldData>> = HashMap::new();
            // Track last messages for each topic
            let mut last_messages: HashMap<String, LastMessage> = HashMap::new();
            let mut last_cache_log = std::time::Instant::now();

            // Wait for connection and subscribe using async stream
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                        connected = true;
                        info!("World discovery connected, subscribing to topics...");

                        // Subscribe to topics
                        if let Err(e) = client.subscribe("iotcraft/worlds/+/info", QoS::AtLeastOnce).await
                        {
                            error!("Failed to subscribe to world info: {}", e);
                            break;
                        }
                        info!("Subscribed to iotcraft/worlds/+/info");

                        if let Err(e) = client.subscribe("iotcraft/worlds/+/data", QoS::AtLeastOnce).await
                        {
                            error!("Failed to subscribe to world data: {}", e);
                            break;
                        }
                        info!("Subscribed to iotcraft/worlds/+/data");

                        if let Err(e) =
                            client.subscribe("iotcraft/worlds/+/data/chunk", QoS::AtLeastOnce).await
                        {
                            error!("Failed to subscribe to world data chunks: {}", e);
                            break;
                        }
                        info!("Subscribed to iotcraft/worlds/+/data/chunk");

                        if let Err(e) =
                            client.subscribe("iotcraft/worlds/+/changes", QoS::AtLeastOnce).await
                        {
                            error!("Failed to subscribe to world changes: {}", e);
                            break;
                        }
                        info!("Subscribed to iotcraft/worlds/+/changes");

                        // Subscribe to block change topics for real-time synchronization
                        if let Err(e) = client
                            .subscribe("iotcraft/worlds/+/state/blocks/placed", QoS::AtLeastOnce).await
                        {
                            error!("Failed to subscribe to block placements: {}", e);
                            break;
                        }
                        info!("Subscribed to iotcraft/worlds/+/state/blocks/placed");

                        if let Err(e) = client
                            .subscribe("iotcraft/worlds/+/state/blocks/removed", QoS::AtLeastOnce).await
                        {
                            error!("Failed to subscribe to block removals: {}", e);
                            break;
                        }
                        info!("Subscribed to iotcraft/worlds/+/state/blocks/removed");

                        subscribed = true;
                        info!(
                            "Successfully subscribed to all world discovery topics, waiting for retained messages..."
                        );
                    }
                    Ok(Event::Incoming(Incoming::Publish(p))) => {
                        if subscribed {
                            info!(
                                "Received message on topic: {} (retained: {})",
                                p.topic, p.retain
                            );
                            // Process retained messages immediately after subscription
                            handle_discovery_message(
                                &p,
                                &mut world_cache,
                                &mut chunk_cache,
                                &mut last_messages,
                                &response_tx,
                            );

                            // If this was a retained world info message, send immediate update
                            if p.topic.contains("/info") {
                                info!("Sending world list update after world info message");
                                let _ = response_tx.send(DiscoveryResponse::WorldListUpdated {
                                    worlds: world_cache.clone(),
                                });
                            }
                        }
                    }
                    Ok(Event::Incoming(Incoming::SubAck(_))) => {
                        if subscribed {
                            info!(
                                "Subscription acknowledged, continuing to wait for retained messages..."
                            );
                            // Continue polling for a bit longer to collect retained messages
                            let start_time = std::time::Instant::now();
                            let mut retained_messages_received = 0;

                            // Wait up to 2 seconds for retained messages after subscription acknowledgment
                            while start_time.elapsed() < Duration::from_secs(2) {
                                match timeout(Duration::from_millis(100), eventloop.poll()).await {
                                    Ok(Ok(Event::Incoming(Incoming::Publish(p)))) => {
                                        info!(
                                            "Processing retained/initial message on topic: {} (retained: {})",
                                            p.topic, p.retain
                                        );
                                        handle_discovery_message(
                                            &p,
                                            &mut world_cache,
                                            &mut chunk_cache,
                                            &mut last_messages,
                                            &response_tx,
                                        );

                                        if p.topic.contains("/info") {
                                            retained_messages_received += 1;
                                            info!(
                                                "Processed world info message ({})",
                                                retained_messages_received
                                            );
                                        }
                                    }
                                    Ok(Ok(_)) => {} // Other events
                                    Ok(Err(e)) => {
                                        error!(
                                            "Connection error while waiting for retained messages: {:?}",
                                            e
                                        );
                                        break;
                                    }
                                    Err(_) => {
                                        // Timeout - no more messages, continue waiting
                                        async_sleep(Duration::from_millis(50)).await;
                                    }
                                }
                            }

                            info!(
                                "Finished collecting retained messages. Found {} worlds in cache",
                                world_cache.len()
                            );
                            if !world_cache.is_empty() {
                                info!(
                                    "Sending initial world list update with {} cached worlds",
                                    world_cache.len()
                                );
                                let _ = response_tx.send(DiscoveryResponse::WorldListUpdated {
                                    worlds: world_cache.clone(),
                                });
                            }

                            // Now break to enter the main processing loop
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        error!("World discovery connection error: {:?}", e);
                        break;
                    }
                }
            }

            if !connected {
                error!("Failed to establish world discovery connection");
                async_sleep(Duration::from_secs(5)).await;
                continue;
            }

            // Handle discovery and connection events
            let mut loop_counter = 0;
            let mut last_debug_log = std::time::Instant::now();
            let mut messages_received_count = 0;

            info!("🔄 Discovery service entering main processing loop...");

            loop {
                // Handle connection events using async polling with timeout
                match timeout(Duration::from_millis(10), eventloop.poll()).await {
                    Ok(Ok(Event::Incoming(Incoming::Publish(p)))) => {
                        messages_received_count += 1;
                        info!(
                            "📬 Main loop received MQTT message #{} on topic: {} (retained: {}, payload_len: {})",
                            messages_received_count,
                            p.topic,
                            p.retain,
                            p.payload.len()
                        );

                        // Log payload content for world info messages
                        if p.topic.contains("/info") {
                            let payload_preview = if p.payload.len() > 100 {
                                format!("{}...", String::from_utf8_lossy(&p.payload[..100]))
                            } else {
                                String::from_utf8_lossy(&p.payload).to_string()
                            };
                            info!("📄 World info payload preview: {}", payload_preview);
                        }

                        handle_discovery_message(
                            &p,
                            &mut world_cache,
                            &mut chunk_cache,
                            &mut last_messages,
                            &response_tx,
                        );

                        info!(
                            "✅ Message processing complete. World cache now has {} worlds",
                            world_cache.len()
                        );
                    }
                    Ok(Ok(other_event)) => {
                        // Log other events more frequently to track connection health
                        if last_debug_log.elapsed() > Duration::from_secs(10) {
                            info!(
                                "📡 Main loop received other MQTT event: {:?} (total messages: {})",
                                other_event, messages_received_count
                            );
                            last_debug_log = std::time::Instant::now();
                        }
                    }
                    Ok(Err(e)) => {
                        error!("World discovery connection error: {:?}", e);
                        break;
                    }
                    Err(_) => {
                        // Timeout - no immediate messages, continue with other tasks
                    }
                }

                loop_counter += 1;
                if loop_counter % 10000 == 0 && last_debug_log.elapsed() > Duration::from_secs(60)
                {
                    info!(
                        "🔄 Main MQTT processing loop is active (iteration {})",
                        loop_counter
                    );
                    last_debug_log = std::time::Instant::now();
                }

                // Handle discovery requests (non-blocking)
                match discovery_rx.try_recv() {
                    Ok(DiscoveryMessage::RefreshWorlds) => {
                        info!(
                            "Received RefreshWorlds request, sending {} cached worlds",
                            world_cache.len()
                        );
                        // Send current world cache
                        let _ = response_tx.send(DiscoveryResponse::WorldListUpdated {
                            worlds: world_cache.clone(),
                        });
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                    Err(mpsc::TryRecvError::Disconnected) => {
                        warn!("World discovery channel disconnected");
                        break;
                    }
                }

                // Periodically log world cache state for debugging
                if last_cache_log.elapsed() > Duration::from_secs(10) {
                    info!(
                        "World discovery cache status: {} worlds cached",
                        world_cache.len()
                    );
                    for (world_id, world_info) in &world_cache {
                        info!("  - {} ({})", world_info.world_name, world_id);
                    }
                    last_cache_log = std::time::Instant::now();
                }

                // Small async yield to avoid busy waiting
                async_sleep(Duration::from_millis(1)).await;
            }

            error!("World discovery disconnected, reconnecting in 5 seconds...");
            async_sleep(Duration::from_secs(5)).await;
        }
    }).detach();

    info!("World discovery initialized");
}

fn handle_discovery_message(
    publish: &rumqttc::Publish,
    world_cache: &mut HashMap<String, SharedWorldInfo>,
    chunk_cache: &mut HashMap<String, HashMap<u32, ChunkedWorldData>>,
    last_messages: &mut HashMap<String, LastMessage>,
    response_tx: &mpsc::Sender<DiscoveryResponse>,
) {
    info!("Received MQTT message on topic: {}", publish.topic);

    // Track last message for this topic
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let payload_str = String::from_utf8_lossy(&publish.payload);
    let truncated_content = if payload_str.len() > 40 {
        format!("{}...", &payload_str[..40])
    } else {
        payload_str.to_string()
    };

    last_messages.insert(
        publish.topic.clone(),
        LastMessage {
            content: truncated_content,
            timestamp,
        },
    );

    // Send last messages update
    let _ = response_tx.send(DiscoveryResponse::LastMessagesUpdated {
        last_messages: last_messages.clone(),
    });

    let topic_parts: Vec<&str> = publish.topic.split('/').collect();

    if topic_parts.len() < 4 {
        warn!("Invalid topic structure: {}", publish.topic);
        return;
    }

    let world_id = topic_parts[2];
    let message_type = topic_parts[3];

    info!(
        "Processing message for world_id: {}, type: {}",
        world_id, message_type
    );

    match message_type {
        "info" => {
            if publish.payload.is_empty() {
                // Empty message means world was unpublished
                world_cache.remove(world_id);
                info!("World {} was unpublished", world_id);
            } else {
                // Parse world info
                match String::from_utf8(publish.payload.to_vec()) {
                    Ok(payload_str) => {
                        match serde_json::from_str::<SharedWorldInfo>(&payload_str) {
                            Ok(world_info) => {
                                info!(
                                    "Discovered world: {} (ID: {}) by {} - adding to cache",
                                    world_info.world_name,
                                    world_info.world_id,
                                    world_info.host_name
                                );
                                world_cache.insert(world_id.to_string(), world_info.clone());
                                info!("World cache now contains {} worlds", world_cache.len());
                            }
                            Err(e) => {
                                error!(
                                    "Failed to parse world info payload '{}': {}",
                                    payload_str, e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to decode world info payload: {}", e);
                    }
                }
            }

            // Notify about updated world list
            let _ = response_tx.send(DiscoveryResponse::WorldListUpdated {
                worlds: world_cache.clone(),
            });
        }
        "data" => {
            if topic_parts.len() >= 5 && topic_parts[4] == "chunk" {
                // Handle chunked data
                handle_chunk_message(publish, world_id, chunk_cache, response_tx);
            } else if !publish.payload.is_empty() {
                // Handle regular (non-chunked) data - try both binary and JSON
                match try_parse_world_data(&publish.payload) {
                    Ok(world_data) => {
                        info!("Received world data for: {}", world_id);
                        let _ = response_tx.send(DiscoveryResponse::WorldDataReceived {
                            world_id: world_id.to_string(),
                            world_data,
                        });
                    }
                    Err(e) => {
                        error!("Failed to parse world data: {}", e);
                    }
                }
            }
        }
        "changes" => match String::from_utf8(publish.payload.to_vec()) {
            Ok(payload_str) => match serde_json::from_str::<WorldChange>(&payload_str) {
                Ok(change) => {
                    let _ = response_tx.send(DiscoveryResponse::WorldChangeReceived { change });
                }
                Err(e) => {
                    error!("Failed to parse world change: {}", e);
                }
            },
            Err(e) => {
                error!("Failed to decode world change payload: {}", e);
            }
        },
        "state" => {
            // Handle block change messages
            if topic_parts.len() >= 6 && topic_parts[4] == "blocks" {
                let block_action = topic_parts[5]; // "placed" or "removed"

                match String::from_utf8(publish.payload.to_vec()) {
                    Ok(payload_str) => {
                        match serde_json::from_str::<serde_json::Value>(&payload_str) {
                            Ok(v) => {
                                // Parse the block change message
                                let player_id = v["player_id"].as_str().unwrap_or("").to_string();
                                let player_name =
                                    v["player_name"].as_str().unwrap_or("").to_string();

                                if let Some(change_obj) = v["change"].as_object() {
                                    let change_type = if block_action == "placed" {
                                        if let Some(placed) = change_obj.get("Placed") {
                                            super::shared_world::BlockChangeType::Placed {
                                            x: placed["x"].as_i64().unwrap_or(0) as i32,
                                            y: placed["y"].as_i64().unwrap_or(0) as i32,
                                            z: placed["z"].as_i64().unwrap_or(0) as i32,
                                            block_type: match placed["block_type"].as_str().unwrap_or("Stone") {
                                                "Grass" => crate::environment::BlockType::Grass,
                                                "Dirt" => crate::environment::BlockType::Dirt,
                                                "Stone" => crate::environment::BlockType::Stone,
                                                "QuartzBlock" => crate::environment::BlockType::QuartzBlock,
                                                "GlassPane" => crate::environment::BlockType::GlassPane,
                                                "CyanTerracotta" => crate::environment::BlockType::CyanTerracotta,
                                                "Water" => crate::environment::BlockType::Water,
                                                _ => crate::environment::BlockType::Stone,
                                            },
                                        }
                                        } else {
                                            error!("Invalid placed block change format");
                                            return;
                                        }
                                    } else if block_action == "removed" {
                                        if let Some(removed) = change_obj.get("Removed") {
                                            super::shared_world::BlockChangeType::Removed {
                                                x: removed["x"].as_i64().unwrap_or(0) as i32,
                                                y: removed["y"].as_i64().unwrap_or(0) as i32,
                                                z: removed["z"].as_i64().unwrap_or(0) as i32,
                                            }
                                        } else {
                                            error!("Invalid removed block change format");
                                            return;
                                        }
                                    } else {
                                        error!("Unknown block action: {}", block_action);
                                        return;
                                    };

                                    info!(
                                        "🔄 Parsed block change for world {}: {:?} by {} - sending to response channel",
                                        world_id, change_type, player_name
                                    );

                                    let send_result =
                                        response_tx.send(DiscoveryResponse::BlockChangeReceived {
                                            world_id: world_id.to_string(),
                                            player_id,
                                            player_name,
                                            change_type,
                                        });

                                    if send_result.is_ok() {
                                        info!(
                                            "✅ Block change message sent to response channel successfully"
                                        );
                                    } else {
                                        error!(
                                            "❌ Failed to send block change to response channel: {:?}",
                                            send_result
                                        );
                                    }
                                } else {
                                    error!("Block change message missing 'change' object");
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse block change JSON: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to decode block change payload: {}", e);
                    }
                }
            }
        }
        _ => {
            // Unknown message type
        }
    }
}

/// Try to parse world data from both binary and JSON formats
fn try_parse_world_data(payload: &[u8]) -> Result<WorldSaveData, String> {
    // First try direct JSON deserialization (for non-chunked data)
    if let Ok(payload_str) = String::from_utf8(payload.to_vec()) {
        if let Ok(world_data) = serde_json::from_str::<WorldSaveData>(&payload_str) {
            return Ok(world_data);
        }
    }

    // If that fails, try binary deserialization (for binary payloads)
    serde_json::from_slice::<WorldSaveData>(payload)
        .map_err(|e| format!("Failed to parse world data: {}", e))
}

/// Handle incoming chunk message and reassemble if complete
fn handle_chunk_message(
    publish: &rumqttc::Publish,
    world_id: &str,
    chunk_cache: &mut HashMap<String, HashMap<u32, ChunkedWorldData>>,
    response_tx: &mpsc::Sender<DiscoveryResponse>,
) {
    // Parse the chunk from binary payload
    match serde_json::from_slice::<ChunkedWorldData>(&publish.payload) {
        Ok(chunk) => {
            info!(
                "Received chunk {}/{} for world {} (chunk_id: {})",
                chunk.chunk_index + 1,
                chunk.total_chunks,
                world_id,
                chunk.chunk_id
            );

            // Store the chunk
            let chunk_map = chunk_cache
                .entry(chunk.chunk_id.clone())
                .or_insert_with(HashMap::new);
            chunk_map.insert(chunk.chunk_index, chunk.clone());

            // Check if we have all chunks
            if chunk_map.len() == chunk.total_chunks as usize {
                info!(
                    "All chunks received for {}, reassembling...",
                    chunk.chunk_id
                );

                match reassemble_chunks(chunk_map) {
                    Ok(world_data) => {
                        info!("Successfully reassembled world data for: {}", world_id);
                        let _ = response_tx.send(DiscoveryResponse::WorldDataReceived {
                            world_id: world_id.to_string(),
                            world_data,
                        });

                        // Clean up the chunk cache for this chunk_id
                        chunk_cache.remove(&chunk.chunk_id);
                    }
                    Err(e) => {
                        error!("Failed to reassemble chunks for {}: {}", chunk.chunk_id, e);
                        // Clean up failed assembly
                        chunk_cache.remove(&chunk.chunk_id);
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to parse chunk message: {}", e);
        }
    }
}

/// Reassemble chunks into world data
fn reassemble_chunks(chunk_map: &HashMap<u32, ChunkedWorldData>) -> Result<WorldSaveData, String> {
    // Sort chunks by index and concatenate data
    let mut sorted_chunks: Vec<_> = chunk_map.iter().collect();
    sorted_chunks.sort_by_key(|(index, _)| *index);

    let mut reassembled_data = Vec::new();
    for (expected_index, (actual_index, chunk)) in sorted_chunks.iter().enumerate() {
        if **actual_index != expected_index as u32 {
            return Err(format!(
                "Missing chunk at index {}, found index {}",
                expected_index, actual_index
            ));
        }
        reassembled_data.extend_from_slice(&chunk.data);
    }

    // Decompress the data
    let decompressed = decompress_data(&reassembled_data)
        .map_err(|e| format!("Failed to decompress data: {}", e))?;

    // Deserialize the world data
    serde_json::from_slice::<WorldSaveData>(&decompressed)
        .map_err(|e| format!("Failed to deserialize world data: {}", e))
}

/// Decompress data using deflate
fn decompress_data(compressed: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    use std::io::Read;
    let mut decoder = flate2::read::DeflateDecoder::new(compressed);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

fn handle_discovery_requests(
    mut refresh_events: EventReader<RefreshOnlineWorldsEvent>,
    world_discovery: Res<WorldDiscovery>,
) {
    for _event in refresh_events.read() {
        if let Some(tx) = world_discovery.discovery_tx.lock().unwrap().as_ref() {
            let _ = tx.send(DiscoveryMessage::RefreshWorlds);
        }
    }
}

fn process_discovery_responses(
    world_discovery: Res<WorldDiscovery>,
    mut online_worlds: ResMut<OnlineWorlds>,
    mut commands: Commands,
    mut voxel_world: ResMut<crate::environment::VoxelWorld>,
    existing_blocks_query: Query<Entity, With<crate::environment::VoxelBlock>>,
    voxel_blocks_query: Query<(Entity, &crate::environment::VoxelBlock)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut inventory: ResMut<crate::inventory::PlayerInventory>,
    camera_query: Query<Entity, With<crate::camera_controllers::CameraController>>,
    multiplayer_mode: Res<MultiplayerMode>,
    player_profile: Res<crate::profile::PlayerProfile>,
) {
    if let Some(rx) = world_discovery.world_rx.lock().unwrap().as_ref() {
        while let Ok(response) = rx.try_recv() {
            match response {
                DiscoveryResponse::WorldListUpdated { worlds } => {
                    online_worlds.worlds = worlds;
                    online_worlds.last_updated = Some(std::time::Instant::now());
                    info!(
                        "Updated online worlds list: {} worlds available",
                        online_worlds.worlds.len()
                    );
                }
                DiscoveryResponse::WorldDataReceived {
                    world_id,
                    world_data,
                } => {
                    // Always cache the world data when received
                    info!(
                        "Caching world data for: {} ({} blocks)",
                        world_id,
                        world_data.blocks.len()
                    );
                    online_worlds
                        .world_data_cache
                        .insert(world_id.clone(), world_data.clone());

                    // Also load it immediately if we're already in this world
                    if let MultiplayerMode::JoinedWorld {
                        world_id: joined_id,
                        ..
                    } = &*multiplayer_mode
                    {
                        if *joined_id == world_id {
                            info!("Loading shared world data for: {}", world_id);
                            load_shared_world_data(
                                world_data,
                                &mut commands,
                                &mut voxel_world,
                                &existing_blocks_query,
                                &mut meshes,
                                &mut materials,
                                &asset_server,
                                &mut inventory,
                                &camera_query,
                            );
                        }
                    }
                }
                DiscoveryResponse::WorldChangeReceived { change } => {
                    // Apply world changes if we're in the same world
                    match &*multiplayer_mode {
                        MultiplayerMode::JoinedWorld { world_id, .. }
                        | MultiplayerMode::HostingWorld { world_id, .. } => {
                            if *world_id == change.world_id {
                                apply_world_change(
                                    change,
                                    &mut commands,
                                    &mut voxel_world,
                                    &mut meshes,
                                    &mut materials,
                                    &asset_server,
                                );
                            }
                        }
                        MultiplayerMode::SinglePlayer => {}
                    }
                }
                DiscoveryResponse::BlockChangeReceived {
                    world_id,
                    player_id,
                    player_name,
                    change_type,
                } => {
                    info!(
                        "📨 Received block change for world {}: {:?} by {} (player_id: {})",
                        world_id, change_type, player_name, player_id
                    );
                    info!("🔍 Current multiplayer mode: {:?}", &*multiplayer_mode);
                    info!("👤 Current player ID: {}", player_profile.player_id);

                    // Check if this change is from the current player (avoid duplicate block creation)
                    if player_id == player_profile.player_id {
                        info!("🚫 Ignoring block change from self to prevent duplicate creation");
                        continue; // Skip processing our own changes, but continue processing other messages
                    }

                    // Apply block changes if we're in the same world and from a different player
                    match &*multiplayer_mode {
                        MultiplayerMode::JoinedWorld {
                            world_id: joined_world,
                            ..
                        }
                        | MultiplayerMode::HostingWorld {
                            world_id: joined_world,
                            ..
                        } => {
                            info!(
                                "🌍 Checking world match: joined_world={} vs received_world={}",
                                joined_world, world_id
                            );
                            if *joined_world == world_id {
                                info!(
                                    "✅ World matches! Applying block change from other player: {:?}",
                                    change_type
                                );
                                apply_block_change(
                                    change_type,
                                    &player_name,
                                    &mut commands,
                                    &mut voxel_world,
                                    &voxel_blocks_query,
                                    &mut meshes,
                                    &mut materials,
                                    &asset_server,
                                );
                            } else {
                                info!("❌ World doesn't match: {} != {}", joined_world, world_id);
                            }
                        }
                        MultiplayerMode::SinglePlayer => {
                            info!("🚫 In SinglePlayer mode, ignoring block change");
                        }
                    }
                }
                DiscoveryResponse::LastMessagesUpdated { last_messages } => {
                    // Update last messages in the WorldDiscovery resource
                    if let Ok(mut resource_last_messages) = world_discovery.last_messages.try_lock()
                    {
                        *resource_last_messages = last_messages;
                    }
                }
            }
        }
    }
}

fn load_shared_world_data(
    world_data: WorldSaveData,
    commands: &mut Commands,
    voxel_world: &mut crate::environment::VoxelWorld,
    existing_blocks_query: &Query<Entity, With<crate::environment::VoxelBlock>>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    inventory: &mut crate::inventory::PlayerInventory,
    camera_query: &Query<Entity, With<crate::camera_controllers::CameraController>>,
) {
    info!(
        "Loading shared world with {} blocks",
        world_data.blocks.len()
    );

    // Clear existing blocks
    for entity in existing_blocks_query.iter() {
        commands.entity(entity).despawn();
    }
    voxel_world.blocks.clear();

    // Load blocks
    for block_data in world_data.blocks {
        voxel_world.blocks.insert(
            IVec3::new(block_data.x, block_data.y, block_data.z),
            block_data.block_type,
        );
    }

    // Spawn visual blocks
    for (pos, block_type) in voxel_world.blocks.iter() {
        let cube_mesh = meshes.add(Cuboid::new(
            crate::environment::CUBE_SIZE,
            crate::environment::CUBE_SIZE,
            crate::environment::CUBE_SIZE,
        ));
        let texture_path = match block_type {
            crate::environment::BlockType::Grass => "textures/grass.webp",
            crate::environment::BlockType::Dirt => "textures/dirt.webp",
            crate::environment::BlockType::Stone => "textures/stone.webp",
            crate::environment::BlockType::QuartzBlock => "textures/quartz_block.webp",
            crate::environment::BlockType::GlassPane => "textures/glass_pane.webp",
            crate::environment::BlockType::CyanTerracotta => "textures/cyan_terracotta.webp",
            crate::environment::BlockType::Water => "textures/water.webp",
        };
        let texture: Handle<Image> = asset_server.load(texture_path);
        let material = materials.add(StandardMaterial {
            base_color_texture: Some(texture),
            ..default()
        });

        commands.spawn((
            Mesh3d(cube_mesh),
            MeshMaterial3d(material),
            Transform::from_translation(pos.as_vec3()),
            crate::environment::VoxelBlock { position: *pos },
        ));
    }

    // Load inventory
    *inventory = world_data.inventory;
    inventory.ensure_proper_size();
    // ResMut automatically marks resources as changed when mutated

    // Set player position if camera exists
    if let Ok(camera_entity) = camera_query.single() {
        commands.entity(camera_entity).insert(Transform {
            translation: world_data.player_position,
            rotation: world_data.player_rotation,
            ..default()
        });
    }

    info!("Successfully loaded shared world");
}

fn apply_world_change(
    change: WorldChange,
    commands: &mut Commands,
    voxel_world: &mut crate::environment::VoxelWorld,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) {
    match change.change_type {
        WorldChangeType::BlockPlaced {
            x,
            y,
            z,
            block_type,
        } => {
            let pos = IVec3::new(x, y, z);
            voxel_world.blocks.insert(pos, block_type);

            // Spawn visual block
            let cube_mesh = meshes.add(Cuboid::new(
                crate::environment::CUBE_SIZE,
                crate::environment::CUBE_SIZE,
                crate::environment::CUBE_SIZE,
            ));
            let texture_path = match block_type {
                crate::environment::BlockType::Grass => "textures/grass.webp",
                crate::environment::BlockType::Dirt => "textures/dirt.webp",
                crate::environment::BlockType::Stone => "textures/stone.webp",
                crate::environment::BlockType::QuartzBlock => "textures/quartz_block.webp",
                crate::environment::BlockType::GlassPane => "textures/glass_pane.webp",
                crate::environment::BlockType::CyanTerracotta => "textures/cyan_terracotta.webp",
                crate::environment::BlockType::Water => "textures/water.webp",
            };
            let texture: Handle<Image> = asset_server.load(texture_path);
            let material = materials.add(StandardMaterial {
                base_color_texture: Some(texture),
                ..default()
            });

            commands.spawn((
                Mesh3d(cube_mesh),
                MeshMaterial3d(material),
                Transform::from_translation(pos.as_vec3()),
                crate::environment::VoxelBlock { position: pos },
            ));

            info!(
                "Applied block placement from {}: {:?} at ({}, {}, {})",
                change.player_name, block_type, x, y, z
            );
        }
        WorldChangeType::BlockRemoved { x, y, z } => {
            let pos = IVec3::new(x, y, z);
            voxel_world.blocks.remove(&pos);

            info!(
                "Applied block removal from {}: ({}, {}, {})",
                change.player_name, x, y, z
            );
        }
        WorldChangeType::PlayerJoined { player_name, .. } => {
            info!("Player joined: {}", player_name);
        }
        WorldChangeType::PlayerLeft { player_name, .. } => {
            info!("Player left: {}", player_name);
        }
    }
}

fn apply_block_change(
    change_type: super::shared_world::BlockChangeType,
    player_name: &str,
    commands: &mut Commands,
    voxel_world: &mut crate::environment::VoxelWorld,
    voxel_blocks_query: &Query<(Entity, &crate::environment::VoxelBlock)>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
) {
    match change_type {
        super::shared_world::BlockChangeType::Placed {
            x,
            y,
            z,
            block_type,
        } => {
            let pos = IVec3::new(x, y, z);
            voxel_world.blocks.insert(pos, block_type);

            // Spawn visual block
            let cube_mesh = meshes.add(Cuboid::new(
                crate::environment::CUBE_SIZE,
                crate::environment::CUBE_SIZE,
                crate::environment::CUBE_SIZE,
            ));
            let texture_path = match block_type {
                crate::environment::BlockType::Grass => "textures/grass.webp",
                crate::environment::BlockType::Dirt => "textures/dirt.webp",
                crate::environment::BlockType::Stone => "textures/stone.webp",
                crate::environment::BlockType::QuartzBlock => "textures/quartz_block.webp",
                crate::environment::BlockType::GlassPane => "textures/glass_pane.webp",
                crate::environment::BlockType::CyanTerracotta => "textures/cyan_terracotta.webp",
                crate::environment::BlockType::Water => "textures/water.webp",
            };
            let texture: Handle<Image> = asset_server.load(texture_path);
            let material = materials.add(StandardMaterial {
                base_color_texture: Some(texture),
                ..default()
            });

            commands.spawn((
                Mesh3d(cube_mesh),
                MeshMaterial3d(material),
                Transform::from_translation(pos.as_vec3()),
                crate::environment::VoxelBlock { position: pos },
            ));

            info!(
                "Applied block placement from {}: {:?} at ({}, {}, {})",
                player_name, block_type, x, y, z
            );
        }
        super::shared_world::BlockChangeType::Removed { x, y, z } => {
            let pos = IVec3::new(x, y, z);
            voxel_world.blocks.remove(&pos);

            // Despawn visual block by finding the entity at this position
            for (entity, block) in voxel_blocks_query.iter() {
                if block.position == pos {
                    commands.entity(entity).despawn();
                    info!("Despawned block entity at position ({}, {}, {})", x, y, z);
                    break;
                }
            }

            info!(
                "Applied block removal from {}: ({}, {}, {})",
                player_name, x, y, z
            );
        }
    }
}

fn auto_refresh_worlds(
    online_worlds: ResMut<OnlineWorlds>,
    mut refresh_events: EventWriter<RefreshOnlineWorldsEvent>,
) {
    // Auto-refresh worlds every 30 seconds
    if let Some(last_updated) = online_worlds.last_updated {
        if last_updated.elapsed() > Duration::from_secs(30) {
            refresh_events.write(RefreshOnlineWorldsEvent);
        }
    } else {
        // First time, refresh immediately
        refresh_events.write(RefreshOnlineWorldsEvent);
    }
}
