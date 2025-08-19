use bevy::prelude::*;
use log::{error, info, warn};
use rumqttc::{Client, Event, Incoming, MqttOptions, QoS};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::shared_world::*;
use crate::config::MqttConfig;
use crate::profile::PlayerProfile;
use crate::world::WorldSaveData;

/// Resource for managing world publishing
#[derive(Resource)]
pub struct WorldPublisher {
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
    // New message types for state synchronization
    PublishWorldState {
        world_id: String,
        world_data: WorldSaveData,
        is_snapshot: bool,
    },
    PublishBlockChange {
        world_id: String,
        player_id: String,
        player_name: String,
        change_type: BlockChangeType,
    },
    PublishInventoryChange {
        world_id: String,
        player_id: String,
        inventory: crate::inventory::PlayerInventory,
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
    mqtt_config: Res<MqttConfig>,
    world_publisher: ResMut<WorldPublisher>,
) {
    let (publish_tx, publish_rx) = mpsc::channel::<PublishMessage>();

    // Store the sender in the resource
    *world_publisher.publish_tx.lock().unwrap() = Some(publish_tx);

    let mqtt_host = mqtt_config.host.clone();
    let mqtt_port = mqtt_config.port;

    // Spawn publisher thread
    thread::spawn(move || {
        info!("Starting world publisher thread...");

        // Test initial connection
        let mut opts = MqttOptions::new("iotcraft-world-publisher", &mqtt_host, mqtt_port);
        opts.set_keep_alive(Duration::from_secs(30));
        opts.set_clean_session(true);

        let (_client, mut conn) = Client::new(opts, 10);

        let mut initial_connection_success = false;
        let mut connection_attempts = 0;

        // Try initial connection
        for event in conn.iter() {
            match event {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    info!("World publisher connected successfully - world sharing enabled");
                    initial_connection_success = true;
                    break;
                }
                Err(e) => {
                    error!("Initial world publisher connection failed: {:?}", e);
                    connection_attempts += 1;
                    if connection_attempts > 2 {
                        break;
                    }
                }
                Ok(_) => {}
            }
        }

        if !initial_connection_success {
            info!("MQTT connection not available - world publishing disabled");
            return; // Exit thread - world publishing is disabled
        }

        // Continue with normal world publishing
        loop {
            let mut opts = MqttOptions::new("iotcraft-world-publisher", &mqtt_host, mqtt_port);
            opts.set_keep_alive(Duration::from_secs(30));
            opts.set_clean_session(true);

            let (client, mut conn) = Client::new(opts, 10);
            let mut connected = false;
            let mut reconnect = false;

            // Wait for connection
            for event in conn.iter() {
                match event {
                    Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                        connected = true;
                        break;
                    }
                    Err(e) => {
                        error!("World publisher connection error: {:?}", e);
                        reconnect = true;
                        break;
                    }
                    Ok(_) => {}
                }
            }

            if !connected {
                error!("Failed to establish world publisher connection");
                thread::sleep(Duration::from_secs(5));
                continue;
            }

            // Handle messages
            loop {
                // Handle connection events (non-blocking)
                match conn.try_recv() {
                    Ok(Ok(_)) => {}
                    Ok(Err(e)) => {
                        error!("World publisher connection error: {:?}", e);
                        let _ = reconnect; // Intentionally unused for now
                        break;
                    }
                    Err(rumqttc::TryRecvError::Empty) => {}
                    Err(rumqttc::TryRecvError::Disconnected) => {
                        error!("World publisher connection lost");
                        let _ = reconnect; // Intentionally unused for now
                        break;
                    }
                }

                // Handle publish messages (non-blocking)
                match publish_rx.try_recv() {
                    Ok(msg) => {
                        if connected {
                            handle_publish_message(&client, msg);
                        }
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                    Err(mpsc::TryRecvError::Disconnected) => {
                        warn!("World publisher channel disconnected");
                        break;
                    }
                }

                if reconnect {
                    break;
                }

                thread::sleep(Duration::from_millis(10));
            }

            error!("World publisher disconnected, reconnecting in 5 seconds...");
            thread::sleep(Duration::from_secs(5));
        }
    });

    info!("World publisher initialized");
}

fn handle_publish_message(client: &Client, message: PublishMessage) {
    match message {
        PublishMessage::PublishWorld {
            world_info,
            world_data,
        } => {
            // Publish world info to discovery topic
            let info_topic = format!("iotcraft/worlds/{}/info", world_info.world_id);
            if let Ok(payload) = serde_json::to_string(&world_info) {
                if let Err(e) = client.publish(&info_topic, QoS::AtLeastOnce, true, payload) {
                    error!("Failed to publish world info: {}", e);
                } else {
                    info!("Published world info for {}", world_info.world_name);
                }
            }

            // Publish world data to data topic
            let data_topic = format!("iotcraft/worlds/{}/data", world_info.world_id);
            if let Ok(payload) = serde_json::to_string(&world_data) {
                if let Err(e) = client.publish(&data_topic, QoS::AtLeastOnce, true, payload) {
                    error!("Failed to publish world data: {}", e);
                } else {
                    info!("Published world data for {}", world_info.world_name);
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
        // New synchronization message handlers
        PublishMessage::PublishWorldState {
            world_id,
            world_data,
            is_snapshot,
        } => {
            let topic = if is_snapshot {
                format!("iotcraft/worlds/{}/state/snapshot", world_id)
            } else {
                format!("iotcraft/worlds/{}/state/update", world_id)
            };

            if let Ok(payload) = serde_json::to_string(&world_data) {
                // Use retain=true for snapshots, false for updates
                if let Err(e) = client.publish(&topic, QoS::AtLeastOnce, is_snapshot, payload) {
                    error!("Failed to publish world state to {}: {}", topic, e);
                } else {
                    info!("Published world state to {}", topic);
                }
            }
        }
        PublishMessage::PublishBlockChange {
            world_id,
            player_id,
            player_name,
            change_type,
        } => {
            let topic = match &change_type {
                BlockChangeType::Placed { .. } => {
                    format!("iotcraft/worlds/{}/state/blocks/placed", world_id)
                }
                BlockChangeType::Removed { .. } => {
                    format!("iotcraft/worlds/{}/state/blocks/removed", world_id)
                }
            };

            let change_message = serde_json::json!({
                "player_id": player_id,
                "player_name": player_name,
                "timestamp": chrono::Utc::now().timestamp_millis(),
                "change": change_type
            });

            if let Ok(payload) = serde_json::to_string(&change_message) {
                if let Err(e) = client.publish(&topic, QoS::AtLeastOnce, false, payload) {
                    error!("Failed to publish block change to {}: {}", topic, e);
                }
            }
        }
        PublishMessage::PublishInventoryChange {
            world_id,
            player_id,
            inventory,
        } => {
            let topic = format!("iotcraft/worlds/{}/state/inventory/updates", world_id);

            let inventory_message = serde_json::json!({
                "player_id": player_id,
                "timestamp": chrono::Utc::now().timestamp_millis(),
                "inventory": inventory
            });

            if let Ok(payload) = serde_json::to_string(&inventory_message) {
                // Retain inventory updates so new joiners get latest state
                if let Err(e) = client.publish(&topic, QoS::AtLeastOnce, true, payload) {
                    error!("Failed to publish inventory change to {}: {}", topic, e);
                }
            }
        }
    }
}

fn handle_world_publishing(
    mut publish_events: EventReader<PublishWorldEvent>,
    mut unpublish_events: EventReader<UnpublishWorldEvent>,
    world_publisher: Res<WorldPublisher>,
    current_world: Option<Res<crate::world::CurrentWorld>>,
    voxel_world: Res<crate::environment::VoxelWorld>,
    inventory: Res<crate::inventory::PlayerInventory>,
    camera_query: Query<&Transform, With<crate::camera_controllers::CameraController>>,
    player_profile: Res<PlayerProfile>,
    multiplayer_mode: Res<MultiplayerMode>,
) {
    // Handle publish events
    for event in publish_events.read() {
        if let Some(current_world) = &current_world {
            if let Some(tx) = world_publisher.publish_tx.lock().unwrap().as_ref() {
                // Get current player position
                let (player_position, player_rotation) =
                    if let Ok(transform) = camera_query.single() {
                        (transform.translation, transform.rotation)
                    } else {
                        (Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY)
                    };

                // Convert blocks from VoxelWorld
                let blocks: Vec<crate::world::VoxelBlockData> = voxel_world
                    .blocks
                    .iter()
                    .map(|(pos, block_type)| crate::world::VoxelBlockData {
                        x: pos.x,
                        y: pos.y,
                        z: pos.z,
                        block_type: *block_type,
                    })
                    .collect();

                let world_data = WorldSaveData {
                    metadata: current_world.metadata.clone(),
                    blocks,
                    player_position,
                    player_rotation,
                    inventory: (*inventory).clone(),
                };

                if let MultiplayerMode::HostingWorld { world_id, .. } = &*multiplayer_mode {
                    let world_info = SharedWorldInfo {
                        world_id: world_id.clone(),
                        world_name: current_world.name.clone(),
                        description: current_world.metadata.description.clone(),
                        host_player: player_profile.player_id.clone(),
                        host_name: player_profile.player_name.clone(),
                        created_at: current_world.metadata.created_at.clone(),
                        last_updated: chrono::Utc::now().to_rfc3339(),
                        player_count: 1, // For now, just the host
                        max_players: event.max_players,
                        is_public: event.is_public,
                        version: current_world.metadata.version.clone(),
                    };

                    if let Err(e) = tx.send(PublishMessage::PublishWorld {
                        world_info,
                        world_data,
                    }) {
                        error!("Failed to send publish message: {}", e);
                    }
                }
            }
        }
    }

    // Handle unpublish events
    for event in unpublish_events.read() {
        if let Some(tx) = world_publisher.publish_tx.lock().unwrap().as_ref() {
            if let Err(e) = tx.send(PublishMessage::UnpublishWorld {
                world_id: event.world_id.clone(),
            }) {
                error!("Failed to send unpublish message: {}", e);
            }
        }
    }
}

fn handle_world_changes_for_publishing(
    mut change_events: EventReader<WorldChangeEvent>,
    world_publisher: Res<WorldPublisher>,
    multiplayer_mode: Res<MultiplayerMode>,
) {
    for event in change_events.read() {
        match &*multiplayer_mode {
            MultiplayerMode::HostingWorld { .. } => {
                // Broadcast changes when hosting
                if let Some(tx) = world_publisher.publish_tx.lock().unwrap().as_ref() {
                    if let Err(e) = tx.send(PublishMessage::BroadcastChange {
                        change: event.change.clone(),
                    }) {
                        error!("Failed to send broadcast message: {}", e);
                    }
                }
            }
            MultiplayerMode::JoinedWorld { .. } => {
                // Forward changes to host when joined
                if let Some(tx) = world_publisher.publish_tx.lock().unwrap().as_ref() {
                    if let Err(e) = tx.send(PublishMessage::BroadcastChange {
                        change: event.change.clone(),
                    }) {
                        error!("Failed to send change message: {}", e);
                    }
                }
            }
            MultiplayerMode::SinglePlayer => {
                // No broadcasting in single player mode
            }
        }
    }
}
