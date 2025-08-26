pub mod mqtt_utils;
pub mod shared_world;
pub mod world_discovery;
pub mod world_publisher;

#[cfg(test)]
mod world_tests;

// pub use mqtt_utils::*;
pub use shared_world::*;
pub use world_discovery::*;
pub use world_publisher::*;

// Original multiplayer functionality
use bevy::prelude::*;
use log::{error, info};
use rumqttc::{Client, Event, Incoming, MqttOptions, Outgoing, QoS};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::config::MqttConfig;
use crate::profile::PlayerProfile;

#[derive(Resource, Debug, Clone)]
pub struct WorldId(pub String);

impl Default for WorldId {
    fn default() -> Self {
        Self("default".to_string())
    }
}

#[derive(Component)]
pub struct RemotePlayer {
    // RemotePlayer component marker - no fields currently needed
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PoseMessage {
    player_id: String,
    player_name: String,
    pos: [f32; 3],
    yaw: f32,
    pitch: f32,
    ts: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DisconnectMessage {
    player_id: String,
    ts: u64,
}

#[derive(Resource)]
struct PoseRx(pub Mutex<mpsc::Receiver<PoseMessage>>);

#[derive(Resource)]
struct PoseTx(pub Mutex<mpsc::Sender<PoseMessage>>);

#[derive(Resource, Default)]
pub struct MultiplayerConnectionStatus {
    pub connection_available: bool,
}

impl MultiplayerConnectionStatus {
    pub fn is_multiplayer_enabled(&self) -> bool {
        self.connection_available
    }
}

pub struct MultiplayerPlugin;

impl Plugin for MultiplayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WorldId::default())
            .insert_resource(InitialPoseSent::default())
            .insert_resource(MultiplayerConnectionStatus::default())
            .add_systems(Startup, start_multiplayer_connections)
            .add_systems(Update, (publish_local_pose, apply_remote_poses))
            .add_systems(Update, update_position_timer);
    }
}

#[derive(Resource)]
struct PositionTimer {
    timer: Timer,
    last_position: Option<Vec3>,
}

impl Default for PositionTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.1, TimerMode::Repeating), // 10 Hz
            last_position: None,
        }
    }
}

#[derive(Resource, Default)]
struct InitialPoseSent;

// Remove the field since it's not being used - this is just a marker resource

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as u64
}

// Initialize both subscriber and publisher connections
fn start_multiplayer_connections(
    mut commands: Commands,
    mqtt: Res<MqttConfig>,
    world: Res<WorldId>,
    profile: Res<PlayerProfile>,
) {
    let (pose_tx, pose_rx) = mpsc::channel::<PoseMessage>();
    let (outgoing_tx, outgoing_rx) = mpsc::channel::<PoseMessage>();

    commands.insert_resource(PoseRx(Mutex::new(pose_rx)));
    commands.insert_resource(PoseTx(Mutex::new(outgoing_tx)));
    commands.insert_resource(PositionTimer::default());

    // Enable multiplayer since MQTT is available (we know this because device announcements work)
    commands.insert_resource(MultiplayerConnectionStatus {
        connection_available: true,
    });
    info!("Multiplayer enabled - MQTT broker is available");

    let subscribe_topic = format!("iotcraft/worlds/{}/players/+/pose", world.0);
    let host = mqtt.host.clone();
    let port = mqtt.port;
    let client_id = format!("desktop-{}", profile.player_id);

    // Subscriber thread - persistent connection for receiving poses
    let sub_host = host.clone();
    let sub_client_id = format!("{}-sub", client_id);
    thread::spawn(move || {
        info!("Starting multiplayer pose subscriber...");

        // Try to connect once; if it fails, disable multiplayer
        let mut opts = MqttOptions::new(&sub_client_id, &sub_host, port);
        opts.set_keep_alive(Duration::from_secs(30));
        opts.set_clean_session(true);

        let (client, mut conn) = Client::new(opts, 10);

        // Test connection with a short timeout
        let mut initial_connection_success = false;
        let mut connection_attempts = 0;

        info!("Attempting initial MQTT connection for multiplayer...");

        for notification in conn.iter() {
            match notification {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    info!("Pose subscriber connected successfully - multiplayer enabled");
                    initial_connection_success = true;

                    // Subscribe to the topic
                    if let Err(e) = client.subscribe(&subscribe_topic, QoS::AtLeastOnce) {
                        error!("Failed to subscribe to poses: {}", e);
                        break;
                    } else {
                        info!("Subscribed to poses: {}", subscribe_topic);
                    }
                    break; // Proceed to main loop
                }
                Err(e) => {
                    error!("Initial MQTT connection failed: {:?}", e);
                    connection_attempts += 1;
                    if connection_attempts > 2 {
                        // Quick initial retry limit
                        break;
                    }
                }
                Ok(_) => {}
            }
        }

        if !initial_connection_success {
            info!("MQTT connection not available - multiplayer disabled");
            return; // Exit thread - multiplayer is disabled
        }

        // If we got here, initial connection worked - continue with normal multiplayer operation
        loop {
            let mut opts = MqttOptions::new(&sub_client_id, &sub_host, port);
            opts.set_keep_alive(Duration::from_secs(30));
            opts.set_clean_session(true);

            let (client, mut conn) = Client::new(opts, 10);
            let mut subscribed = false;

            for notification in conn.iter() {
                match notification {
                    Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                        if let Err(e) = client.subscribe(&subscribe_topic, QoS::AtLeastOnce) {
                            error!("Failed to subscribe to poses: {}", e);
                            break;
                        } else {
                            subscribed = true;
                        }
                    }
                    Ok(Event::Incoming(Incoming::Publish(p))) => {
                        if subscribed && p.topic.contains("/pose") {
                            if let Ok(s) = String::from_utf8(p.payload.to_vec()) {
                                if let Ok(msg) = serde_json::from_str::<PoseMessage>(&s) {
                                    if let Err(_) = pose_tx.send(msg) {
                                        error!("Failed to send pose message to game thread");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Ok(Event::Incoming(Incoming::SubAck(_))) => {
                        info!("Pose subscription acknowledged by broker");
                    }
                    Ok(_) => {}
                    Err(e) => {
                        error!("Pose subscriber connection error: {:?}", e);
                        break;
                    }
                }
            }

            // Reconnect after delay
            info!("Pose subscriber disconnected, reconnecting in 5 seconds...");
            thread::sleep(Duration::from_secs(5));
        }
    });

    // Publisher thread - persistent connection for publishing poses
    let pub_host = host.clone();
    let pub_client_id = format!("{}-pub", client_id);
    let publish_topic_template = format!("iotcraft/worlds/{}/players", world.0);
    let _disconnect_topic = format!(
        "{}/{}/disconnect",
        publish_topic_template, profile.player_id
    );
    let _disconnect_payload = serde_json::to_string(&DisconnectMessage {
        player_id: profile.player_id.clone(),
        ts: now_ts(),
    })
    .unwrap_or_else(|_| "{}".to_string());

    thread::spawn(move || {
        info!("Starting multiplayer pose publisher...");

        // Test initial connection
        let mut opts = MqttOptions::new(&pub_client_id, &pub_host, port);
        opts.set_keep_alive(Duration::from_secs(30));
        opts.set_clean_session(true);

        let (_client, mut conn) = Client::new(opts, 10);

        let mut initial_connection_success = false;
        let mut connection_attempts = 0;

        // Try initial connection
        for event in conn.iter() {
            match event {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    info!("Pose publisher connected successfully - multiplayer enabled");
                    initial_connection_success = true;
                    break;
                }
                Err(e) => {
                    error!("Initial publisher connection failed: {:?}", e);
                    connection_attempts += 1;
                    if connection_attempts > 2 {
                        break;
                    }
                }
                Ok(_) => {}
            }
        }

        if !initial_connection_success {
            info!("MQTT connection not available - multiplayer publisher disabled");
            return; // Exit thread - multiplayer is disabled
        }

        // Continue with normal multiplayer publishing
        loop {
            let mut opts = MqttOptions::new(&pub_client_id, &pub_host, port);
            opts.set_keep_alive(Duration::from_secs(30));
            opts.set_clean_session(true);

            let (client, mut conn) = Client::new(opts, 10);
            let mut connected = false;
            let reconnect = false;

            // Wait for connection
            let mut connection_established = false;
            for event in conn.iter() {
                match event {
                    Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                        connected = true;
                        connection_established = true;
                        break;
                    }
                    Err(e) => {
                        error!("Pose publisher connection error: {:?}", e);
                        break;
                    }
                    Ok(_) => {}
                }
            }

            if !connection_established {
                error!("Failed to establish publisher connection");
                thread::sleep(Duration::from_secs(5));
                continue;
            }

            // Now handle messages and additional events in non-blocking mode
            loop {
                // Handle additional connection events (non-blocking)
                match conn.try_recv() {
                    Ok(Ok(Event::Outgoing(Outgoing::Publish(_)))) => {
                        // Message sent successfully (keep quiet)
                    }
                    Ok(Ok(_other)) => {
                        // Other events we don't need to log
                    }
                    Ok(Err(e)) => {
                        error!("Pose publisher connection error: {:?}", e);
                        break;
                    }
                    Err(rumqttc::TryRecvError::Empty) => {
                        // No connection events right now, that's fine
                    }
                    Err(rumqttc::TryRecvError::Disconnected) => {
                        error!("Pose publisher connection lost");
                        break;
                    }
                }

                // Check for messages to publish (non-blocking)
                match outgoing_rx.try_recv() {
                    Ok(msg) => {
                        if connected {
                            let topic =
                                format!("{}/{}/pose", publish_topic_template, msg.player_id);
                            if let Ok(payload) = serde_json::to_string(&msg) {
                                if let Err(e) = client.publish(
                                    &topic,
                                    QoS::AtMostOnce,
                                    false,
                                    payload.as_bytes(),
                                ) {
                                    error!("Failed to publish pose: {}", e);
                                }
                            }
                        }
                    }
                    Err(_) => {
                        // No messages to publish right now, that's fine
                    }
                }

                if reconnect {
                    break;
                }

                // Small sleep to avoid busy waiting
                thread::sleep(Duration::from_millis(10));
            }

            error!("Pose publisher disconnected, reconnecting in 5 seconds...");
            thread::sleep(Duration::from_secs(5));
        }
    });

    info!("Multiplayer connections initialized");
}

fn update_position_timer(mut timer: ResMut<PositionTimer>, time: Res<Time>) {
    timer.timer.tick(time.delta());
}

fn publish_local_pose(
    profile: Res<PlayerProfile>,
    mut timer: ResMut<PositionTimer>,
    pose_tx: Res<PoseTx>,
    camera_q: Query<&Transform, With<Camera>>,
    connection_status: Res<MultiplayerConnectionStatus>,
) {
    if !timer.timer.just_finished() {
        return;
    }

    // Don't publish poses if multiplayer is disabled
    if !connection_status.is_multiplayer_enabled() {
        return;
    }

    let Ok(transform) = camera_q.single() else {
        return;
    };

    let current_position = transform.translation;

    // Always send initial pose, then only if position changed significantly
    let should_send = match timer.last_position {
        Some(last_pos) => current_position.distance(last_pos) > 0.01, // 1cm threshold
        None => true,                                                 // Always send first time
    };

    if !should_send {
        return;
    }

    timer.last_position = Some(current_position);

    // Extract yaw/pitch from rotation
    let forward = transform.forward();
    let yaw = forward.x.atan2(forward.z);
    let pitch = forward.y.asin();

    let msg = PoseMessage {
        player_id: profile.player_id.clone(),
        player_name: profile.player_name.clone(),
        pos: [current_position.x, current_position.y, current_position.z],
        yaw,
        pitch,
        ts: now_ts(),
    };

    // Send to publisher thread
    if let Ok(tx) = pose_tx.0.lock() {
        if let Err(_) = tx.send(msg.clone()) {
            // Channel disconnected or other error
            error!("Failed to send pose message to publisher thread");
        }
    }
}

fn apply_remote_poses(
    profile: Res<PlayerProfile>,
    pose_rx: Res<PoseRx>,
    mut commands: Commands,
    mut remote_players: Query<
        (&mut Transform, &crate::player_avatar::PlayerAvatar),
        With<RemotePlayer>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    connection_status: Res<MultiplayerConnectionStatus>,
) {
    // Don't process remote poses if multiplayer is disabled
    if !connection_status.is_multiplayer_enabled() {
        return;
    }

    let Ok(rx) = pose_rx.0.lock() else {
        return;
    };

    // Process all available messages
    while let Ok(msg) = rx.try_recv() {
        // Ignore our own messages
        if msg.player_id == profile.player_id {
            continue;
        }

        // Try to update existing remote player avatar
        let mut updated = false;
        for (mut transform, player_avatar) in remote_players.iter_mut() {
            if player_avatar.player_id == msg.player_id {
                transform.translation = Vec3::new(msg.pos[0], msg.pos[1], msg.pos[2]);
                transform.rotation = Quat::from_rotation_y(msg.yaw);
                updated = true;
                break;
            }
        }

        // Spawn new remote player avatar if not found
        if !updated {
            let position = Vec3::new(msg.pos[0], msg.pos[1], msg.pos[2]);
            let avatar_entity = crate::player_avatar::spawn_player_avatar(
                &mut commands,
                &mut meshes,
                &mut materials,
                position,
                msg.player_id.clone(),
                msg.player_name.clone(),
            );

            // Add RemotePlayer component to the spawned avatar
            commands.entity(avatar_entity).insert(RemotePlayer {});

            info!(
                "New remote player joined: {} ({}) with voxel avatar",
                msg.player_name, msg.player_id
            );
        }
    }
}
