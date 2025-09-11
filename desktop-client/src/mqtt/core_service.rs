use bevy::app::AppExit;
use bevy::prelude::*;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use tokio::sync::mpsc;
use tokio::time::Duration;

use super::mqtt_types::*;
use crate::config::MqttConfig;
use crate::devices::DeviceAnnouncementReceiver;
use crate::multiplayer::mqtt_utils::generate_unique_client_id;
use crate::multiplayer::{PoseRx, PoseTx};

// Re-export key multiplayer types that are now handled by core service
pub use crate::multiplayer::PoseMessage;
use crate::profile::PlayerProfile;

#[cfg(not(target_arch = "wasm32"))]
use crate::multiplayer::{PublishWorldEvent, SharedWorldInfo};

#[cfg(target_arch = "wasm32")]
use crate::ui::main_menu::multiplayer_stubs::{PublishWorldEvent, SharedWorldInfo};

use crate::world::WorldSaveData;

/// Unified outgoing message types for the core MQTT service
#[derive(Debug, Clone)]
pub enum OutgoingMqttMessage {
    /// Publish player pose data
    PublishPose { topic: String, payload: String },
    /// Publish world discovery information (with retain flag)
    PublishWorldInfo {
        topic: String,
        payload: String,
        retain: bool,
    },
    /// Publish world data chunk
    PublishWorldChunk { topic: String, payload: Vec<u8> },
    /// Generic publish for other use cases
    GenericPublish {
        topic: String,
        payload: String,
        qos: QoS,
        retain: bool,
    },
}

/// Resource for sending outgoing MQTT messages
#[derive(Resource)]
pub struct MqttOutgoingTx(pub Mutex<mpsc::UnboundedSender<OutgoingMqttMessage>>);

/// Resource for receiving world discovery messages
#[derive(Resource)]
pub struct WorldDiscoveryReceiver(pub Mutex<std::sync::mpsc::Receiver<SharedWorldInfo>>);

/// Resource for receiving world data messages (complete world save data)
#[derive(Resource)]
pub struct WorldDataReceiver(pub Mutex<std::sync::mpsc::Receiver<(String, WorldSaveData)>>);

/// Resource for sending world publishing events
#[derive(Resource)]
pub struct WorldPublishEventTx(pub Mutex<std::sync::mpsc::Sender<PublishWorldEvent>>);

/// Resource for receiving MQTT connection status updates
#[derive(Resource)]
pub struct MqttConnectionStatusReceiver(pub Mutex<std::sync::mpsc::Receiver<bool>>);

/// Resource tracking the Core MQTT Service connection status
#[derive(Resource, Default)]
pub struct CoreMqttConnectionStatus {
    pub is_connected: bool,
}

/// Global shutdown flag for Core MQTT Service
static MQTT_SERVICE_SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// Core MQTT service plugin that consolidates all MQTT functionality
pub struct CoreMqttServicePlugin;

impl Plugin for CoreMqttServicePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TemperatureResource::default())
            .insert_resource(CoreMqttConnectionStatus::default())
            .add_systems(Startup, initialize_core_mqtt_service)
            .add_systems(
                Update,
                (
                    update_temperature,
                    update_mqtt_connection_status,
                    handle_app_exit,
                ),
            );
    }
}

/// Initialize the core MQTT service with unified connection and message routing
pub fn initialize_core_mqtt_service(
    mut commands: Commands,
    mqtt_config: Res<MqttConfig>,
    profile: Res<PlayerProfile>,
) {
    info!("🚀 Initializing Core MQTT Service (unified connection)");

    // Create channels for all message types
    let (temp_tx, temp_rx) = std::sync::mpsc::channel::<f32>();
    let (device_tx, device_rx) = std::sync::mpsc::channel::<String>();
    let (pose_tx, pose_rx) = std::sync::mpsc::channel::<PoseMessage>();
    let (outgoing_pose_tx, outgoing_pose_rx) = std::sync::mpsc::channel::<PoseMessage>();
    let (world_discovery_tx, world_discovery_rx) = std::sync::mpsc::channel::<SharedWorldInfo>();
    let (world_data_tx, world_data_rx) = std::sync::mpsc::channel::<(String, WorldSaveData)>();
    let (world_publish_event_tx, world_publish_event_rx) =
        std::sync::mpsc::channel::<PublishWorldEvent>();
    let (mqtt_outgoing_tx, mut mqtt_outgoing_rx) = mpsc::unbounded_channel::<OutgoingMqttMessage>();
    let (connection_status_tx, connection_status_rx) = std::sync::mpsc::channel::<bool>();

    // Insert resources for other systems to use
    commands.insert_resource(TemperatureReceiver(Mutex::new(temp_rx)));
    commands.insert_resource(DeviceAnnouncementReceiver(Mutex::new(device_rx)));
    commands.insert_resource(PoseRx(Mutex::new(pose_rx)));
    commands.insert_resource(PoseTx(Mutex::new(outgoing_pose_tx)));
    commands.insert_resource(WorldDiscoveryReceiver(Mutex::new(world_discovery_rx)));
    commands.insert_resource(WorldDataReceiver(Mutex::new(world_data_rx)));
    commands.insert_resource(WorldPublishEventTx(Mutex::new(world_publish_event_tx)));
    commands.insert_resource(MqttOutgoingTx(Mutex::new(mqtt_outgoing_tx)));
    commands.insert_resource(MqttConnectionStatusReceiver(Mutex::new(
        connection_status_rx,
    )));

    let client_id = generate_unique_client_id("iotcraft-core-service");
    let mqtt_host = mqtt_config.host.clone();
    let mqtt_port = mqtt_config.port;
    let player_id = profile.player_id.clone();

    info!(
        "🌐 Core MQTT Service using unified client ID: {}",
        client_id
    );

    // Spawn the unified MQTT service thread with Tokio runtime
    thread::spawn(move || {
        // Create Tokio runtime inside the thread
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                error!(
                    "❌ Failed to create Tokio runtime for Core MQTT Service: {}",
                    e
                );
                return;
            }
        };

        rt.block_on(async move {
        info!("🔔 Starting async Core MQTT Service...");

        let mut reconnect_attempts = 0;
        loop {
            // Check for shutdown signal in outer loop
            if MQTT_SERVICE_SHUTDOWN.load(Ordering::Relaxed) {
                info!("🛑 Core MQTT Service received shutdown signal, exiting reconnection loop");
                return; // Exit the async function, which will end the thread
            }

            let reconnect_client_id = generate_unique_client_id("iotcraft-core-service");
            info!(
                "🔄 Connecting Core MQTT Service with ID: {} (attempt {})",
                reconnect_client_id, reconnect_attempts + 1
            );

            let mut opts = MqttOptions::new(&reconnect_client_id, &mqtt_host, mqtt_port);
            opts.set_keep_alive(std::time::Duration::from_secs(30));
            opts.set_clean_session(true);
            // Increase max packet size to 5MB to handle large world data messages
            opts.set_max_packet_size(5242880, 5242880);
            info!("🔧 Core MQTT Service configured with max packet size: 5MB");

            let (client, mut eventloop) = AsyncClient::new(opts, 100);
            let mut current_world_id = String::from("default");

            // Send initial connection status
            let _ = connection_status_tx.send(false);

            // Subscribe to required topics
            let topics = vec![
                "home/sensor/temperature",
                "devices/announce",
                "iotcraft/worlds/+/info",
                "iotcraft/worlds/+/data",
                "iotcraft/worlds/+/players/+/pose",
            ];

            for topic in &topics {
                if let Err(e) = client.subscribe(*topic, QoS::AtMostOnce).await {
                    error!("❌ Failed to subscribe to {}: {}", topic, e);
                } else {
                    info!("📡 Subscribed to topic: {}", topic);
                }
            }

            let mut connected = false;

            // Main async event loop with shutdown check
            loop {
                // Check for shutdown signal first
                if MQTT_SERVICE_SHUTDOWN.load(Ordering::Relaxed) {
                    info!("🛑 Core MQTT Service received shutdown signal, exiting gracefully");
                    return; // Exit the async function, which will end the thread
                }

                tokio::select! {
                    // Handle MQTT events
                    event = eventloop.poll() => {
                        match event {
                            Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                                if !connected {
                                    info!("✅ Core MQTT Service connected successfully");
                                    connected = true;
                                    reconnect_attempts = 0;
                                    let _ = connection_status_tx.send(true);
                                }
                            }
                            Ok(Event::Incoming(Incoming::Publish(p))) => {
                                info!("📥 MQTT message received on topic '{}' - Size: {} bytes ({}MB)", 
                                     p.topic, p.payload.len(), p.payload.len() as f64 / 1048576.0);
                                route_incoming_message(
                                    &p.topic,
                                    &p.payload,
                                    &temp_tx,
                                    &device_tx,
                                    &pose_tx,
                                    &world_discovery_tx,
                                    &world_data_tx,
                                );
                            }
                            Ok(Event::Incoming(Incoming::SubAck(_))) => {
                                info!("📨 Subscription acknowledged");
                            }
                            Ok(_) => {} // Other events
                            Err(e) => {
                                error!("❌ MQTT connection error: {:?}", e);
                                let _ = connection_status_tx.send(false);
                                connected = false;
                                break;
                            }
                        }
                    }

                    // Handle outgoing messages (non-blocking)
                    msg = mqtt_outgoing_rx.recv() => {
                        if let Some(outgoing_msg) = msg {
                            if connected {
                                info!("📤 Processing outgoing message: {:?}", outgoing_msg);
                                match outgoing_msg {
                                    OutgoingMqttMessage::PublishWorldInfo { topic, payload, retain } => {
                                        info!("🌍 Publishing world info to topic: {} (retain={})", topic, retain);
                                        if let Err(e) = client.publish(&topic, QoS::AtMostOnce, retain, payload.as_bytes()).await {
                                            error!("❌ Failed to publish world info: {}", e);
                                        } else {
                                            info!("✅ Successfully published world info to {}", topic);
                                        }
                                    }
                                    OutgoingMqttMessage::PublishWorldChunk { topic, payload } => {
                                        info!("📡 Publishing world chunk to topic: {} ({} bytes)", topic, payload.len());
                                        if let Err(e) = client.publish(&topic, QoS::AtMostOnce, false, payload).await {
                                            error!("❌ Failed to publish world chunk: {}", e);
                                        } else {
                                            info!("✅ Successfully published world chunk to {}", topic);
                                        }
                                    }
                                    OutgoingMqttMessage::PublishPose { topic, payload } => {
                                        if let Err(e) = client.publish(&topic, QoS::AtMostOnce, false, payload.as_bytes()).await {
                                            error!("❌ Failed to publish pose: {}", e);
                                        }
                                    }
                                    OutgoingMqttMessage::GenericPublish { topic, payload, qos, retain } => {
                                        if let Err(e) = client.publish(&topic, qos, retain, payload.as_bytes()).await {
                                            error!("❌ Failed to publish to {}: {}", topic, e);
                                        } else {
                                            info!("✅ Successfully published to {}", topic);
                                        }
                                    }
                                }
                            } else {
                                warn!("⚠️ Dropping outgoing message - not connected: {:?}", outgoing_msg);
                            }
                        }
                    }

                    // Handle pose messages (from sync channels)
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {
                        // Process sync channel messages periodically
                        while let Ok(pose_msg) = outgoing_pose_rx.try_recv() {
                            if connected {
                                let topic = format!("iotcraft/worlds/{}/players/{}/pose", current_world_id, player_id);
                                if let Ok(payload) = serde_json::to_string(&pose_msg) {
                                    if let Err(e) = client.publish(&topic, QoS::AtMostOnce, false, payload.as_bytes()).await {
                                        error!("❌ Failed to publish pose: {}", e);
                                    }
                                }
                            }
                        }

                        // Handle world publish events
                        while let Ok(publish_event) = world_publish_event_rx.try_recv() {
                            if connected {
                                info!("🌍 Processing world publish event: {}", publish_event.world_name);
                                let world_id = format!("{}_{}", publish_event.world_name, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
                                current_world_id = world_id.clone();

                                let world_info = SharedWorldInfo {
                                    world_id: world_id.clone(),
                                    world_name: publish_event.world_name.clone(),
                                    description: format!("World shared from desktop: {}", publish_event.world_name),
                                    host_player: player_id.clone(),
                                    host_name: "DesktopHost".to_string(),
                                    created_at: chrono::Utc::now().to_rfc3339(),
                                    last_updated: chrono::Utc::now().to_rfc3339(),
                                    player_count: 1,
                                    max_players: publish_event.max_players,
                                    is_public: publish_event.is_public,
                                    version: "1.0.0".to_string(),
                                };

                                let topic = format!("iotcraft/worlds/{}/info", world_id);
                                if let Ok(payload) = serde_json::to_string(&world_info) {
                                    info!("🌍 Publishing world '{}' to topic '{}' (retain=true)", world_info.world_name, topic);
                                    if let Err(e) = client.publish(&topic, QoS::AtMostOnce, true, payload.as_bytes()).await {
                                        error!("❌ Failed to publish world info: {}", e);
                                    } else {
                                        info!("✅ Successfully published world '{}' to MQTT topic '{}'", world_info.world_name, topic);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Reconnection backoff
            reconnect_attempts += 1;
            let backoff = std::cmp::min(reconnect_attempts * 2, 30);
            warn!("❌ Disconnected, reconnecting in {} seconds...", backoff);
            tokio::time::sleep(Duration::from_secs(backoff)).await;
        }
        }); // End rt.block_on
    }); // End thread::spawn

    info!("✅ Core MQTT Service initialized");
}

/// Route incoming MQTT messages to the appropriate channels based on topic
fn route_incoming_message(
    topic: &str,
    payload: &[u8],
    temp_tx: &std::sync::mpsc::Sender<f32>,
    device_tx: &std::sync::mpsc::Sender<String>,
    pose_tx: &std::sync::mpsc::Sender<PoseMessage>,
    world_discovery_tx: &std::sync::mpsc::Sender<SharedWorldInfo>,
    world_data_tx: &std::sync::mpsc::Sender<(String, WorldSaveData)>,
) {
    match topic {
        "home/sensor/temperature" => {
            if let Ok(temp_str) = String::from_utf8(payload.to_vec()) {
                if let Ok(temp_val) = temp_str.parse::<f32>() {
                    let _ = temp_tx.send(temp_val);
                    info!("🌡️ Temperature update: {}°C", temp_val);
                }
            }
        }
        "devices/announce" => {
            if let Ok(device_msg) = String::from_utf8(payload.to_vec()) {
                info!("📢 Device announcement received: {}", device_msg);
                let _ = device_tx.send(device_msg);
            }
        }
        _ => {
            // Handle pattern-based topics
            if topic.starts_with("iotcraft/worlds/") && topic.ends_with("/info") {
                // World discovery messages
                if let Ok(world_info_str) = String::from_utf8(payload.to_vec()) {
                    if !world_info_str.is_empty() {
                        info!("🌍 Received world info on topic: {}", topic);
                        if let Ok(world_info) =
                            serde_json::from_str::<SharedWorldInfo>(&world_info_str)
                        {
                            info!(
                                "🌍 Discovered world: {} ({})",
                                world_info.world_name, world_info.world_id
                            );
                            let _ = world_discovery_tx.send(world_info);
                        } else {
                            error!("❌ Failed to parse world info JSON: {}", world_info_str);
                        }
                    } else {
                        info!("🌍 Empty world info (world unpublished): {}", topic);
                    }
                }
            } else if topic.starts_with("iotcraft/worlds/") && topic.ends_with("/data") {
                // World data messages (complete world save data)
                if let Ok(world_data_str) = String::from_utf8(payload.to_vec()) {
                    if !world_data_str.is_empty() {
                        info!(
                            "🌍 Received world data on topic: {} - Raw payload size: {} bytes ({:.2}MB)",
                            topic,
                            payload.len(),
                            payload.len() as f64 / 1048576.0
                        );
                        info!(
                            "🌍 World data UTF-8 string length: {} chars",
                            world_data_str.len()
                        );

                        // Extract world ID from topic (iotcraft/worlds/{world_id}/data)
                        let topic_parts: Vec<&str> = topic.split('/').collect();
                        if topic_parts.len() >= 3 {
                            let world_id = topic_parts[2].to_string();
                            info!(
                                "🔍 Attempting to parse world data JSON for world_id: {}",
                                world_id
                            );

                            match serde_json::from_str::<WorldSaveData>(&world_data_str) {
                                Ok(world_data) => {
                                    info!(
                                        "✅ Successfully parsed world data for: {} ({} blocks)",
                                        world_id,
                                        world_data.blocks.len()
                                    );
                                    let _ = world_data_tx.send((world_id, world_data));
                                }
                                Err(e) => {
                                    error!(
                                        "❌ Failed to parse world data JSON for: {} - Error: {}",
                                        world_id, e
                                    );
                                    error!(
                                        "❌ JSON payload preview (first 500 chars): {}",
                                        &world_data_str[..std::cmp::min(500, world_data_str.len())]
                                    );
                                }
                            }
                        }
                    } else {
                        info!("🌍 Empty world data (world removed): {}", topic);
                    }
                } else {
                    error!(
                        "❌ Failed to convert world data payload to UTF-8 string for topic: {}",
                        topic
                    );
                }
            } else if topic.starts_with("iotcraft/worlds/")
                && topic.contains("/players/")
                && topic.ends_with("/pose")
            {
                // Multiplayer pose messages
                if let Ok(pose_str) = String::from_utf8(payload.to_vec()) {
                    if let Ok(pose_msg) = serde_json::from_str::<PoseMessage>(&pose_str) {
                        let player_name = pose_msg.player_name.clone();
                        let _ = pose_tx.send(pose_msg);
                        info!("📡 Received pose from player: {}", player_name);
                    } else {
                        error!("❌ Failed to parse pose message: {}", pose_str);
                    }
                }
            } else {
                info!("📨 Unhandled topic: {}", topic);
            }
        }
    }
}

pub fn update_temperature(
    mut temp_res: ResMut<TemperatureResource>,
    receiver: Res<TemperatureReceiver>,
) {
    if let Ok(rx) = receiver.0.lock() {
        if let Ok(val) = rx.try_recv() {
            temp_res.value = Some(val);
        }
    }
}

/// Update MQTT connection status from Core MQTT Service thread
pub fn update_mqtt_connection_status(
    mut status: ResMut<CoreMqttConnectionStatus>,
    receiver: Res<MqttConnectionStatusReceiver>,
) {
    if let Ok(rx) = receiver.0.lock() {
        // Process all available status updates (keep the latest)
        while let Ok(is_connected) = rx.try_recv() {
            status.is_connected = is_connected;
        }
    }
}

/// Handle application exit events and trigger MQTT service shutdown
pub fn handle_app_exit(mut exit_events: EventReader<AppExit>) {
    for _event in exit_events.read() {
        info!("🛑 Application exit detected, shutting down Core MQTT Service");
        MQTT_SERVICE_SHUTDOWN.store(true, Ordering::Relaxed);
        // Give a small moment for the MQTT thread to process the shutdown signal
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
