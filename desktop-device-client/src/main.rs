use clap::Parser;
use log::{error, info, warn};
use rand::Rng;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::signal;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval, MissedTickBehavior};

#[cfg(test)]
mod tests;

#[derive(Parser)]
#[command(name = "desktop-device-client")]
#[command(about = "A desktop MQTT device client that simulates ESP32 and player functionality")]
struct Args {
    /// Device ID (if not provided, generates a random one)
    #[arg(short, long)]
    device_id: Option<String>,

    /// Device type (lamp or door)
    #[arg(short = 't', long, default_value = "lamp")]
    device_type: String,

    /// MQTT broker host
    #[arg(long, default_value = "localhost")]
    host: String,

    /// MQTT broker port
    #[arg(long, default_value_t = 1883)]
    port: u16,

    /// Initial X position
    #[arg(long, default_value_t = 1.0)]
    x: f32,

    /// Initial Y position
    #[arg(long, default_value_t = 0.5)]
    y: f32,

    /// Initial Z position  
    #[arg(long, default_value_t = 2.0)]
    z: f32,

    /// Enable player emulation (publishes player poses)
    #[arg(long)]
    emulate_player: bool,

    /// Player ID for multiplayer (if not provided, generates a random one)
    #[arg(long)]
    player_id: Option<String>,

    /// Player name for multiplayer (defaults to system username)
    #[arg(long)]
    player_name: Option<String>,

    /// World ID for multiplayer
    #[arg(long, default_value = "default")]
    world_id: String,

    /// Movement pattern for player emulation (static, circle, random)
    #[arg(long, default_value = "static")]
    movement_pattern: String,
    
    /// Enable world publishing (publishes world info to MQTT)
    #[arg(long)]
    publish_world: bool,
    
    /// World name (if different from ID)
    #[arg(long)]
    world_name: Option<String>,
    
    /// World description
    #[arg(long, default_value = "A new world")]
    world_description: String,
}

/// Device properties structure matching ESP32-C6 implementation
#[derive(Serialize, Deserialize, Debug, Clone)]
struct DeviceProperties {
    x: f32,
    y: f32,
    z: f32,
}

/// Position update structure for MQTT messages
#[derive(Deserialize, Debug)]
struct PositionUpdate {
    x: f32,
    y: f32,
    z: f32,
}

/// Device announcement structure matching ESP32-C6 format
#[derive(Serialize, Debug)]
struct DeviceAnnouncement {
    device_id: String,
    device_type: String,
    state: String,
    location: DeviceLocation,
}

#[derive(Serialize, Debug)]
struct DeviceLocation {
    x: f32,
    y: f32,
    z: f32,
}

/// Virtual device state
#[derive(Debug, Clone)]
struct DeviceState {
    properties: DeviceProperties,
    light_state: bool, // true = ON, false = OFF
}

impl Default for DeviceState {
    fn default() -> Self {
        Self {
            properties: DeviceProperties {
                x: 1.0,
                y: 0.5,
                z: 2.0,
            },
            light_state: false,
        }
    }
}

/// Player pose message structure matching desktop client multiplayer system
#[derive(Serialize, Deserialize, Debug, Clone)]
struct PoseMessage {
    player_id: String,
    player_name: String,
    pos: [f32; 3],
    yaw: f32,
    pitch: f32,
    ts: u64,
}

/// Player state for emulation
#[derive(Debug, Clone)]
struct PlayerState {
    position: [f32; 3],
    yaw: f32,
    pitch: f32,
    movement_time: f32,
}

/// World info message structure matching desktop client world discovery system
#[derive(Serialize, Deserialize, Debug, Clone)]
struct WorldInfoMessage {
    world_id: String,
    world_name: String,
    description: String,
    host_player: String,
    host_name: String,
    created_at: String,
    last_updated: String,
    player_count: u32,
    max_players: u32,
    is_public: bool,
    version: String,
}

/// Block data for world data message
#[derive(Serialize, Deserialize, Debug, Clone)]
struct BlockData {
    x: i32,
    y: i32,
    z: i32,
    block_type: String,
}

/// World data message structure matching desktop client world discovery system
#[derive(Serialize, Deserialize, Debug, Clone)]
struct WorldDataMessage {
    metadata: WorldMetadata,
    blocks: Vec<BlockData>,
}

/// World metadata for world data message
#[derive(Serialize, Deserialize, Debug, Clone)]
struct WorldMetadata {
    name: String,
    description: String,
    created_at: String,
    last_played: String,
    version: String,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            position: [0.0, 2.0, 0.0],
            yaw: 0.0,
            pitch: 0.0,
            movement_time: 0.0,
        }
    }
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as u64
}

fn generate_player_id() -> String {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 8];
    rng.fill(&mut bytes);
    format!("player-{}", hex::encode(bytes))
}

fn update_player_position(player_state: &mut PlayerState, movement_pattern: &str, delta_time: f32) {
    player_state.movement_time += delta_time;

    match movement_pattern {
        "circle" => {
            let radius = 5.0;
            let speed = 0.5; // radians per second
            let angle = player_state.movement_time * speed;
            player_state.position[0] = radius * angle.cos();
            player_state.position[2] = radius * angle.sin();
            player_state.yaw = angle + std::f32::consts::PI / 2.0; // Look in movement direction
        }
        "random" => {
            if player_state.movement_time % 3.0 < delta_time {
                // Change direction every 3 seconds
                let mut rng = rand::thread_rng();
                player_state.position[0] += rng.gen_range(-2.0..2.0);
                player_state.position[2] += rng.gen_range(-2.0..2.0);
                player_state.yaw = rng.gen_range(0.0..2.0 * std::f32::consts::PI);
            }
        }
        "static" | _ => {
            // No movement, keep current position
        }
    }
}

async fn run_player_emulation(
    player_id: String,
    player_name: String,
    world_id: String,
    movement_pattern: String,
    initial_position: [f32; 3],
    client: AsyncClient,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    info!(
        "👤 Starting player emulation: {} ({})",
        player_name, player_id
    );
    info!("🌍 World ID: {}", world_id);
    info!("🚶 Movement pattern: {}", movement_pattern);

    let mut player_state = PlayerState {
        position: initial_position,
        ..Default::default()
    };

    let topic = format!("iotcraft/worlds/{}/players/{}/pose", world_id, player_id);

    let mut pose_interval = interval(Duration::from_millis(100)); // 10 Hz
    pose_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut last_time = std::time::Instant::now();

    loop {
        tokio::select! {
            _ = pose_interval.tick() => {
                let now = std::time::Instant::now();
                let delta_time = now.duration_since(last_time).as_secs_f32();
                last_time = now;

                // Update player position based on movement pattern
                update_player_position(&mut player_state, &movement_pattern, delta_time);

                let pose_msg = PoseMessage {
                    player_id: player_id.clone(),
                    player_name: player_name.clone(),
                    pos: player_state.position,
                    yaw: player_state.yaw,
                    pitch: player_state.pitch,
                    ts: now_ts(),
                };

                if let Ok(payload) = serde_json::to_string(&pose_msg) {
                    if let Err(e) = client
                        .publish(&topic, QoS::AtMostOnce, false, payload)
                        .await
                    {
                        warn!("Failed to publish pose: {}", e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("👋 Player {} disconnecting", player_name);
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    // Check if device ID was explicitly provided for player-only mode detection
    let device_id_provided = args.device_id.is_some();

    // Generate device ID if not provided
    let device_id = args.device_id.unwrap_or_else(|| {
        let suffix: u32 = rand::thread_rng().gen();
        format!("desktop-{:08x}", suffix)
    });

    info!("🚀 Starting desktop device client with ID: {}", device_id);
    info!("📝 Device type: {}", args.device_type);
    info!("🌐 MQTT broker: {}:{}", args.host, args.port);
    info!("📍 Initial position: ({}, {}, {})", args.x, args.y, args.z);

    if args.emulate_player {
        info!("👤 Player emulation enabled");
    }
    
    if args.publish_world {
        info!("🌍 World publishing enabled for world: {}", args.world_id);
    }

    // Initialize device state with provided position
    let initial_state = DeviceState {
        properties: DeviceProperties {
            x: args.x,
            y: args.y,
            z: args.z,
        },
        light_state: false,
    };

    let device_state = Arc::new(RwLock::new(initial_state));

    // Create shutdown signal channel
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);

    // Configure MQTT client
    let mut mqttoptions = MqttOptions::new(&device_id, &args.host, args.port);
    mqttoptions.set_keep_alive(Duration::from_secs(30));
    mqttoptions.set_clean_session(true);

    let (client, eventloop) = AsyncClient::new(mqttoptions, 10);

    // Extract player info that might be needed for both player emulation and world publishing
    let player_id_for_emulation = args.player_id.clone();
    let player_name_for_emulation = args.player_name.clone();
    
    // Spawn player emulation if enabled
    if args.emulate_player {
        let player_id = player_id_for_emulation.unwrap_or_else(generate_player_id);
        let player_name = player_name_for_emulation.unwrap_or_else(|| whoami::username());
        let initial_position = [args.x, args.y, args.z];

        info!(
            "👤 Starting player emulation: {} ({})",
            player_name, player_id
        );
        info!(
            "🌍 World: {}, Movement: {}",
            args.world_id, args.movement_pattern
        );

        let player_client = client.clone();
        let player_shutdown_rx = shutdown_tx.subscribe();

        tokio::spawn(run_player_emulation(
            player_id,
            player_name,
            args.world_id.clone(),
            args.movement_pattern.clone(),
            initial_position,
            player_client,
            player_shutdown_rx,
        ));
    }

    // Spawn world publishing task if enabled
    if args.publish_world {
        let world_client = client.clone();
        let world_id = args.world_id.clone();
        let world_name = args.world_name.unwrap_or_else(|| args.world_id.clone());
        let world_description = args.world_description.clone();
        let player_id = args.player_id.unwrap_or_else(generate_player_id);
        let player_name = args.player_name.unwrap_or_else(|| whoami::username());
        let world_shutdown_rx = shutdown_tx.subscribe();

        tokio::spawn(run_world_publisher(
            world_id,
            world_name,
            world_description,
            player_id,
            player_name,
            world_client,
            world_shutdown_rx,
        ));
    }

    // Clone client for cleanup
    let cleanup_client = client.clone();
    let cleanup_device_id = device_id.clone();
    let cleanup_device_type = args.device_type.clone();
    let cleanup_device_state = device_state.clone();

    // Spawn signal handler task
    let shutdown_tx_signal = shutdown_tx.clone();
    tokio::spawn(async move {
        // Handle CTRL+C signal
        if let Err(e) = signal::ctrl_c().await {
            error!("Failed to listen for ctrl-c signal: {}", e);
            return;
        }

        info!("🛑 Received CTRL+C, initiating graceful shutdown...");

        // Send offline announcement
        let offline_announcement = {
            let state = cleanup_device_state.read().await;
            DeviceAnnouncement {
                device_id: cleanup_device_id,
                device_type: cleanup_device_type,
                state: "offline".to_string(),
                location: DeviceLocation {
                    x: state.properties.x,
                    y: state.properties.y,
                    z: state.properties.z,
                },
            }
        };

        if let Ok(payload) = serde_json::to_string(&offline_announcement) {
            if let Err(e) = cleanup_client
                .publish("devices/announce", QoS::AtLeastOnce, false, payload)
                .await
            {
                error!("❌ Failed to send offline announcement: {}", e);
            } else {
                info!("📤 Offline announcement sent successfully");
            }
        }

        // Give some time for the message to be sent
        tokio::time::sleep(Duration::from_millis(100)).await;

        if let Err(e) = shutdown_tx_signal.send(()) {
            error!("Failed to send shutdown signal: {}", e);
        }
    });

    // Start MQTT client (always needed for basic MQTT connectivity)
    if args.emulate_player && !device_id_provided && args.device_type == "lamp" {
        // Player-only mode - don't create device, just handle MQTT connectivity
        info!("👤 Running in player-only mode (no device emulation)");
        tokio::select! {
            result = run_minimal_mqtt_client(client, eventloop) => {
                if let Err(e) = result {
                    error!("❌ MQTT client error: {}", e);
                }
            }
            _ = shutdown_rx.recv() => {
                info!("✅ Graceful shutdown completed");
            }
        }
    } else {
        // Normal device mode (with or without player emulation)
        tokio::select! {
            result = run_device_client(device_id, args.device_type, device_state, client, eventloop) => {
                if let Err(e) = result {
                    error!("❌ MQTT client error: {}", e);
                }
            }
            _ = shutdown_rx.recv() => {
                info!("✅ Graceful shutdown completed");
            }
        }
    }

    Ok(())
}

async fn run_minimal_mqtt_client(
    _client: AsyncClient,
    mut eventloop: rumqttc::EventLoop,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("👤 Running minimal MQTT client for player-only mode");

    // Just maintain the connection, don't subscribe to device topics
    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                info!("✅ Connected to MQTT broker successfully");
            }
            Ok(_) => {
                // Handle other events silently
            }
            Err(e) => {
                error!("❌ MQTT connection error: {:?}", e);
                info!("🔄 Reconnecting in 5 seconds...");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn run_world_publisher(
    world_id: String,
    world_name: String,
    world_description: String,
    host_player_id: String,
    host_player_name: String,
    client: AsyncClient,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    info!("🌍 Starting world publisher for world: {}", world_id);
    
    // Create timestamps
    let created_at = chrono::Utc::now().to_rfc3339();
    let last_played = created_at.clone();
    
    // Create world info message
    let world_info = WorldInfoMessage {
        world_id: format!("{}_", world_id) + &now_ts().to_string(),
        world_name: world_name.clone(),
        description: world_description.clone(),
        host_player: host_player_id.clone(),
        host_name: host_player_name.clone(),
        created_at: created_at.clone(),
        last_updated: created_at.clone(),
        player_count: 1,
        max_players: 10,
        is_public: true,
        version: "1.0.0".to_string(),
    };
    
    // Create world data message with some sample blocks
    let world_data = WorldDataMessage {
        metadata: WorldMetadata {
            name: world_name,
            description: world_description,
            created_at: created_at.clone(),
            last_played,
            version: "1.0.0".to_string(),
        },
        blocks: generate_sample_blocks(100),
    };
    
    // Topic for world info
    let info_topic = format!("iotcraft/worlds/{}/info", world_id);
    
    // Topic for world data
    let data_topic = format!("iotcraft/worlds/{}/data", world_id);
    
    // Serialize messages to JSON
    let info_payload = match serde_json::to_string(&world_info) {
        Ok(payload) => payload,
        Err(e) => {
            error!("Failed to serialize world info: {}", e);
            return;
        }
    };
    
    let data_payload = match serde_json::to_string(&world_data) {
        Ok(payload) => payload,
        Err(e) => {
            error!("Failed to serialize world data: {}", e);
            return;
        }
    };
    
    // Publish world info and data - retain flag set to true for discovery
    if let Err(e) = client.publish(&info_topic, QoS::AtLeastOnce, true, info_payload.clone()).await {
        error!("Failed to publish world info: {}", e);
    } else {
        info!("✅ Published world info to {}", info_topic);
    }
    
    if let Err(e) = client.publish(&data_topic, QoS::AtLeastOnce, true, data_payload.clone()).await {
        error!("Failed to publish world data: {}", e);
    } else {
        info!("✅ Published world data to {}", data_topic);
    }
    
    // Set up interval to periodically update world info (last_updated field)
    let mut update_interval = interval(Duration::from_secs(30)); // Update every 30 seconds
    update_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    
    loop {
        tokio::select! {
            _ = update_interval.tick() => {
                // Update the last_updated timestamp
                let mut updated_info = world_info.clone();
                updated_info.last_updated = chrono::Utc::now().to_rfc3339();
                
                if let Ok(payload) = serde_json::to_string(&updated_info) {
                    if let Err(e) = client.publish(&info_topic, QoS::AtLeastOnce, true, payload).await {
                        warn!("Failed to update world info: {}", e);
                    } else {
                        info!("🔄 Updated world info for {}", world_id);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("🛑 Shutting down world publisher for {}", world_id);
                
                // Attempt to clean up by publishing empty messages with retain flag
                // This is optional but helps clean up retained messages
                if let Err(e) = client.publish(&info_topic, QoS::AtLeastOnce, true, "").await {
                    warn!("Failed to clean up retained world info: {}", e);
                }
                
                if let Err(e) = client.publish(&data_topic, QoS::AtLeastOnce, true, "").await {
                    warn!("Failed to clean up retained world data: {}", e);
                }
                
                break;
            }
        }
    }
}

// Generate sample blocks for the world
fn generate_sample_blocks(count: usize) -> Vec<BlockData> {
    let mut blocks = Vec::with_capacity(count);
    let mut rng = rand::thread_rng();
    
    // Block types to randomly choose from
    let block_types = ["Grass", "Stone", "Dirt", "Water"];
    
    for _ in 0..count {
        // Generate random positions between -40 and 40
        let x = rng.gen_range(-40..41);
        let y = rng.gen_range(0..5); // Height between 0 and 4
        let z = rng.gen_range(-40..41);
        
        // Randomly select a block type
        let block_type = block_types[rng.gen_range(0..block_types.len())].to_string();
        
        blocks.push(BlockData { x, y, z, block_type });
    }
    
    blocks
}

async fn run_device_client(
    device_id: String,
    device_type: String,
    device_state: Arc<RwLock<DeviceState>>,
    client: AsyncClient,
    mut eventloop: rumqttc::EventLoop,
) -> Result<(), Box<dyn std::error::Error>> {
    // Subscribe to device topics
    let light_topic = format!("home/{}/light", device_id);
    let position_topic = format!("home/{}/position/set", device_id);

    info!("Attempting to connect to MQTT broker...");

    // Handle connection and subscriptions
    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                info!("Connected to MQTT broker successfully");

                // Subscribe to topics
                if let Err(e) = client.subscribe(&light_topic, QoS::AtLeastOnce).await {
                    error!("Failed to subscribe to light topic {}: {}", light_topic, e);
                } else {
                    info!("Subscribed to light topic: {}", light_topic);
                }

                if let Err(e) = client.subscribe(&position_topic, QoS::AtLeastOnce).await {
                    error!(
                        "Failed to subscribe to position topic {}: {}",
                        position_topic, e
                    );
                } else {
                    info!("Subscribed to position topic: {}", position_topic);
                }

                // Send device announcement
                let announcement = {
                    let state = device_state.read().await;
                    DeviceAnnouncement {
                        device_id: device_id.clone(),
                        device_type: device_type.clone(),
                        state: "online".to_string(),
                        location: DeviceLocation {
                            x: state.properties.x,
                            y: state.properties.y,
                            z: state.properties.z,
                        },
                    }
                };

                if let Ok(payload) = serde_json::to_string(&announcement) {
                    if let Err(e) = client
                        .publish("devices/announce", QoS::AtLeastOnce, false, payload)
                        .await
                    {
                        error!("Failed to send device announcement: {}", e);
                    } else {
                        info!(
                            "Device announcement sent successfully: {} at position ({}, {}, {})",
                            device_id,
                            announcement.location.x,
                            announcement.location.y,
                            announcement.location.z
                        );
                    }
                }
            }
            Ok(Event::Incoming(Incoming::Publish(p))) => {
                if let Ok(payload_str) = String::from_utf8(p.payload.to_vec()) {
                    info!(
                        "Received MQTT message on topic '{}': {}",
                        p.topic, payload_str
                    );

                    // Handle light control messages
                    if p.topic.ends_with("/light") {
                        let mut state = device_state.write().await;
                        match payload_str.as_str() {
                            "ON" => {
                                state.light_state = true;
                                info!("💡 Light turned ON (device: {})", device_id);
                            }
                            "OFF" => {
                                state.light_state = false;
                                info!("🔹 Light turned OFF (device: {})", device_id);
                            }
                            cmd => {
                                warn!("Unknown light command '{}' for device {}", cmd, device_id);
                            }
                        }
                    }
                    // Handle position update messages
                    else if p.topic.ends_with("/position/set") {
                        if let Ok(position_update) =
                            serde_json::from_str::<PositionUpdate>(&payload_str)
                        {
                            let mut state = device_state.write().await;
                            state.properties.x = position_update.x;
                            state.properties.y = position_update.y;
                            state.properties.z = position_update.z;

                            info!(
                                "📍 Position updated for device {}: x={}, y={}, z={}",
                                device_id, position_update.x, position_update.y, position_update.z
                            );
                        } else {
                            error!("Failed to parse position update: {}", payload_str);
                        }
                    }
                }
            }
            Ok(Event::Incoming(Incoming::SubAck(_))) => {
                info!("Subscription acknowledged");
            }
            Ok(_) => {
                // Handle other events silently
            }
            Err(e) => {
                error!("MQTT connection error: {:?}", e);
                info!("Reconnecting in 5 seconds...");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
