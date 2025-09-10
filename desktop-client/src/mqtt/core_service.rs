use bevy::prelude::*;
use rumqttc::{Client, Event, Incoming, MqttOptions, QoS};
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

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
pub struct MqttOutgoingTx(pub Mutex<mpsc::Sender<OutgoingMqttMessage>>);

/// Resource for receiving world discovery messages
#[derive(Resource)]
pub struct WorldDiscoveryReceiver(pub Mutex<mpsc::Receiver<SharedWorldInfo>>);

/// Resource for receiving world data messages (complete world save data)
#[derive(Resource)]
pub struct WorldDataReceiver(pub Mutex<mpsc::Receiver<(String, WorldSaveData)>>);

/// Resource for sending world publishing events
#[derive(Resource)]
pub struct WorldPublishEventTx(pub Mutex<mpsc::Sender<PublishWorldEvent>>);

/// Core MQTT service plugin that consolidates all MQTT functionality
pub struct CoreMqttServicePlugin;

impl Plugin for CoreMqttServicePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TemperatureResource::default())
            .add_systems(Startup, initialize_core_mqtt_service)
            .add_systems(Update, update_temperature);
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
    let (temp_tx, temp_rx) = mpsc::channel::<f32>();
    let (device_tx, device_rx) = mpsc::channel::<String>();
    let (pose_tx, pose_rx) = mpsc::channel::<PoseMessage>();
    let (outgoing_pose_tx, outgoing_pose_rx) = mpsc::channel::<PoseMessage>();
    let (world_discovery_tx, world_discovery_rx) = mpsc::channel::<SharedWorldInfo>();
    let (world_data_tx, world_data_rx) = mpsc::channel::<(String, WorldSaveData)>();
    let (world_publish_event_tx, world_publish_event_rx) = mpsc::channel::<PublishWorldEvent>();
    let (mqtt_outgoing_tx, mqtt_outgoing_rx) = mpsc::channel::<OutgoingMqttMessage>();

    // Insert resources for other systems to use
    commands.insert_resource(TemperatureReceiver(Mutex::new(temp_rx)));
    commands.insert_resource(DeviceAnnouncementReceiver(Mutex::new(device_rx)));
    commands.insert_resource(PoseRx(Mutex::new(pose_rx)));
    commands.insert_resource(PoseTx(Mutex::new(outgoing_pose_tx)));
    commands.insert_resource(WorldDiscoveryReceiver(Mutex::new(world_discovery_rx)));
    commands.insert_resource(WorldDataReceiver(Mutex::new(world_data_rx)));
    commands.insert_resource(WorldPublishEventTx(Mutex::new(world_publish_event_tx)));
    commands.insert_resource(MqttOutgoingTx(Mutex::new(mqtt_outgoing_tx)));

    let client_id = generate_unique_client_id("iotcraft-core-service");
    let mqtt_host = mqtt_config.host.clone();
    let mqtt_port = mqtt_config.port;
    let player_id = profile.player_id.clone();

    info!(
        "🌐 Core MQTT Service using unified client ID: {}",
        client_id
    );

    // Spawn the unified MQTT service thread
    thread::spawn(move || {
        info!("🔌 Starting Core MQTT Service thread...");

        loop {
            let reconnect_client_id = generate_unique_client_id("iotcraft-core-service");
            info!(
                "🔄 Connecting Core MQTT Service with ID: {}",
                reconnect_client_id
            );

            let mut opts = MqttOptions::new(&reconnect_client_id, &mqtt_host, mqtt_port);
            opts.set_keep_alive(Duration::from_secs(30));
            opts.set_clean_session(true);
            opts.set_max_packet_size(1048576, 1048576); // 1MB to match server config

            let (client, mut conn) = Client::new(opts, 10);
            let mut connected = false;
            let mut subscribed_topics = Vec::new();

            // Wait for connection
            for event in conn.iter() {
                match event {
                    Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                        info!("✅ Core MQTT Service connected successfully");
                        connected = true;

                        // Subscribe to all required topics
                        let topics_to_subscribe = vec![
                            "home/sensor/temperature",
                            "devices/announce",
                            "iotcraft/worlds/+/info",
                            "iotcraft/worlds/+/data",
                            "iotcraft/worlds/+/players/+/pose", // Note: using wildcard for all worlds
                        ];

                        for topic in &topics_to_subscribe {
                            match client.subscribe(*topic, QoS::AtMostOnce) {
                                Ok(_) => {
                                    info!("📡 Subscribed to topic: {}", topic);
                                    subscribed_topics.push(topic.to_string());
                                }
                                Err(e) => {
                                    error!("❌ Failed to subscribe to {}: {}", topic, e);
                                }
                            }
                        }
                        break;
                    }
                    Err(e) => {
                        error!("❌ Core MQTT Service connection error: {:?}", e);
                        break;
                    }
                    Ok(_) => {}
                }
            }

            if !connected {
                error!("❌ Failed to establish Core MQTT Service connection");
                thread::sleep(Duration::from_secs(5));
                continue;
            }

            // Main event loop - handle both incoming messages and outgoing publishes
            loop {
                // Handle connection events (non-blocking)
                match conn.try_recv() {
                    Ok(Ok(Event::Incoming(Incoming::Publish(p)))) => {
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
                    Ok(Ok(Event::Incoming(Incoming::SubAck(_)))) => {
                        info!("📨 Subscription acknowledged by broker");
                    }
                    Ok(Ok(Event::Outgoing(_))) => {
                        // Outgoing events (publishes, etc.) - keep quiet unless error
                    }
                    Ok(Ok(_)) => {
                        // Other events we don't need to log
                    }
                    Ok(Err(e)) => {
                        error!("❌ Core MQTT Service connection error: {:?}", e);
                        break;
                    }
                    Err(rumqttc::TryRecvError::Empty) => {
                        // No connection events right now, that's fine
                    }
                    Err(rumqttc::TryRecvError::Disconnected) => {
                        error!("❌ Core MQTT Service connection lost");
                        break;
                    }
                }

                // Handle outgoing pose messages
                while let Ok(pose_msg) = outgoing_pose_rx.try_recv() {
                    if connected {
                        let topic = format!("iotcraft/worlds/default/players/{}/pose", player_id);
                        if let Ok(payload) = serde_json::to_string(&pose_msg) {
                            if let Err(e) =
                                client.publish(&topic, QoS::AtMostOnce, false, payload.as_bytes())
                            {
                                error!("❌ Failed to publish pose: {}", e);
                            }
                        }
                    }
                }

                // Handle world publish events
                while let Ok(publish_event) = world_publish_event_rx.try_recv() {
                    if connected {
                        info!(
                            "🌍 Processing world publish event: {}",
                            publish_event.world_name
                        );

                        // Generate world ID (similar to desktop client)
                        let world_id = format!(
                            "{}_{}",
                            publish_event.world_name,
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis()
                        );
                        let topic = format!("iotcraft/worlds/{}/info", world_id);

                        // Create SharedWorldInfo
                        let world_info = SharedWorldInfo {
                            world_id: world_id.clone(),
                            world_name: publish_event.world_name.clone(),
                            description: format!(
                                "World shared from desktop client: {}",
                                publish_event.world_name
                            ),
                            host_player: player_id.clone(),
                            host_name: "DesktopHost".to_string(),
                            created_at: chrono::Utc::now().to_rfc3339(),
                            last_updated: chrono::Utc::now().to_rfc3339(),
                            player_count: 1,
                            max_players: publish_event.max_players,
                            is_public: publish_event.is_public,
                            version: "1.0.0".to_string(),
                        };

                        if let Ok(payload) = serde_json::to_string(&world_info) {
                            // Use retained publish for world info
                            info!(
                                "🌍 Publishing world info to topic '{}' with retain=true",
                                topic
                            );
                            if let Err(e) =
                                client.publish(&topic, QoS::AtMostOnce, true, payload.as_bytes())
                            {
                                error!("❌ Failed to publish world info: {:?}", e);
                            } else {
                                info!(
                                    "✅ Successfully published world '{}'",
                                    world_info.world_name
                                );
                            }
                        }
                    }
                }

                // Handle generic outgoing messages
                while let Ok(outgoing_msg) = mqtt_outgoing_rx.try_recv() {
                    if connected {
                        match outgoing_msg {
                            OutgoingMqttMessage::PublishPose { topic, payload } => {
                                if let Err(e) = client.publish(
                                    &topic,
                                    QoS::AtMostOnce,
                                    false,
                                    payload.as_bytes(),
                                ) {
                                    error!("❌ Failed to publish pose to {}: {}", topic, e);
                                }
                            }
                            OutgoingMqttMessage::PublishWorldInfo {
                                topic,
                                payload,
                                retain,
                            } => {
                                if let Err(e) = client.publish(
                                    &topic,
                                    QoS::AtMostOnce,
                                    retain,
                                    payload.as_bytes(),
                                ) {
                                    error!("❌ Failed to publish world info to {}: {}", topic, e);
                                }
                            }
                            OutgoingMqttMessage::PublishWorldChunk { topic, payload } => {
                                if let Err(e) =
                                    client.publish(&topic, QoS::AtMostOnce, false, payload)
                                {
                                    error!("❌ Failed to publish world chunk to {}: {}", topic, e);
                                }
                            }
                            OutgoingMqttMessage::GenericPublish {
                                topic,
                                payload,
                                qos,
                                retain,
                            } => {
                                if let Err(e) =
                                    client.publish(&topic, qos, retain, payload.as_bytes())
                                {
                                    error!("❌ Failed to publish to {}: {}", topic, e);
                                }
                            }
                        }
                    }
                }

                // Small sleep to avoid busy waiting
                thread::sleep(Duration::from_millis(10));
            }

            error!("🔄 Core MQTT Service disconnected, reconnecting in 5 seconds...");
            thread::sleep(Duration::from_secs(5));
        }
    });

    info!("✅ Core MQTT Service initialized");
}

/// Route incoming MQTT messages to the appropriate channels based on topic
fn route_incoming_message(
    topic: &str,
    payload: &[u8],
    temp_tx: &mpsc::Sender<f32>,
    device_tx: &mpsc::Sender<String>,
    pose_tx: &mpsc::Sender<PoseMessage>,
    world_discovery_tx: &mpsc::Sender<SharedWorldInfo>,
    world_data_tx: &mpsc::Sender<(String, WorldSaveData)>,
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
                        info!("🌍 Received world data on topic: {}", topic);
                        // Extract world ID from topic (iotcraft/worlds/{world_id}/data)
                        let topic_parts: Vec<&str> = topic.split('/').collect();
                        if topic_parts.len() >= 3 {
                            let world_id = topic_parts[2].to_string();
                            if let Ok(world_data) =
                                serde_json::from_str::<WorldSaveData>(&world_data_str)
                            {
                                info!(
                                    "🌍 Parsed world data for: {} ({} blocks)",
                                    world_id,
                                    world_data.blocks.len()
                                );
                                let _ = world_data_tx.send((world_id, world_data));
                            } else {
                                error!("❌ Failed to parse world data JSON for: {}", world_id);
                            }
                        }
                    } else {
                        info!("🌍 Empty world data (world removed): {}", topic);
                    }
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
