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
    pub player_id: String,
}

#[derive(Component)]
pub struct RemoteNameTag;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct PoseMessage {
    player_id: String,
    player_name: String,
    pos: [f32; 3],
    yaw: f32,
    pitch: f32,
    ts: u64,
}

#[derive(Resource)]
struct PoseRx(pub Mutex<mpsc::Receiver<PoseMessage>>);

#[derive(Resource)]
struct PoseTx(pub Mutex<mpsc::Sender<PoseMessage>>);

pub struct MultiplayerPlugin;

impl Plugin for MultiplayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WorldId::default())
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

    let subscribe_topic = format!("iotcraft/worlds/{}/players/+/pose", world.0);
    let host = mqtt.host.clone();
    let port = mqtt.port;
    let client_id = format!("desktop-{}", profile.player_id);

    // Subscriber thread - persistent connection for receiving poses
    let sub_host = host.clone();
    let sub_client_id = format!("{}-sub", client_id);
    thread::spawn(move || {
        info!("Starting multiplayer pose subscriber...");
        loop {
            let mut opts = MqttOptions::new(&sub_client_id, &sub_host, port);
            opts.set_keep_alive(Duration::from_secs(30));
            opts.set_clean_session(false);

            let (client, mut conn) = Client::new(opts, 10);
            if let Err(e) = client.subscribe(&subscribe_topic, QoS::AtLeastOnce) {
                error!("Failed to subscribe to poses: {}", e);
                thread::sleep(Duration::from_secs(5));
                continue;
            }

            info!("Subscribed to poses: {}", subscribe_topic);

            // Handle connection events
            for notification in conn.iter() {
                match notification {
                    Ok(Event::Incoming(Incoming::Publish(p))) => {
                        if p.topic.contains("/pose") {
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
                    Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                        info!("Pose subscriber connected successfully");
                    }
                    Ok(_) => {} // Other events we don't care about
                    Err(e) => {
                        error!("Pose subscriber connection error: {:?}", e);
                        break; // Reconnect
                    }
                }
            }

            error!("Pose subscriber disconnected, reconnecting in 5 seconds...");
            thread::sleep(Duration::from_secs(5));
        }
    });

    // Publisher thread - simple approach using the existing pattern
    let pub_host = host.clone();
    let pub_client_id = format!("{}-pub", client_id);
    let publish_topic_template = format!("iotcraft/worlds/{}/players", world.0);

    thread::spawn(move || {
        info!("Starting multiplayer pose publisher...");

        // Simple publisher - create new connection for each message like the original approach
        // but with proper error handling and reconnection logic
        while let Ok(msg) = outgoing_rx.recv() {
            let topic = format!("{}/{}/pose", publish_topic_template, msg.player_id);
            if let Ok(payload) = serde_json::to_string(&msg) {
                // Use same pattern as existing console commands for consistency
                let host_clone = pub_host.clone();
                let client_id_clone = pub_client_id.clone();

                // Short-lived connection for publishing (like existing code)
                let mut opts = MqttOptions::new(&client_id_clone, &host_clone, port);
                opts.set_keep_alive(Duration::from_secs(5));
                let (client, mut conn) = Client::new(opts, 10);

                if let Err(e) = client.publish(&topic, QoS::AtMostOnce, false, payload.as_bytes()) {
                    error!("Failed to publish pose: {}", e);
                    continue;
                }

                // Drive connection until publish is sent
                for notification in conn.iter() {
                    if let Ok(Event::Outgoing(Outgoing::Publish(_))) = notification {
                        break;
                    }
                    if let Err(_) = notification {
                        break;
                    }
                }
            }
        }

        error!("Multiplayer publisher thread exited");
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
) {
    if !timer.timer.just_finished() {
        return;
    }

    let Ok(transform) = camera_q.single() else {
        return;
    };

    let current_position = transform.translation;

    // Only send if position changed significantly (reduce network traffic)
    let should_send = match timer.last_position {
        Some(last_pos) => current_position.distance(last_pos) > 0.01, // 1cm threshold
        None => true,                                                 // First time
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
        if let Err(_) = tx.send(msg) {
            // Channel disconnected or other error
            error!("Failed to send pose message to publisher thread");
        }
    }
}

fn apply_remote_poses(
    profile: Res<PlayerProfile>,
    pose_rx: Res<PoseRx>,
    mut commands: Commands,
    mut remote_players: Query<(&mut Transform, &RemotePlayer)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(rx) = pose_rx.0.lock() else {
        return;
    };

    // Process all available messages
    while let Ok(msg) = rx.try_recv() {
        // Ignore our own messages
        if msg.player_id == profile.player_id {
            continue;
        }

        // Try to update existing remote player
        let mut updated = false;
        for (mut transform, remote_player) in remote_players.iter_mut() {
            if remote_player.player_id == msg.player_id {
                transform.translation = Vec3::new(msg.pos[0], msg.pos[1], msg.pos[2]);
                // TODO: Also update rotation based on yaw/pitch
                updated = true;
                break;
            }
        }

        // Spawn new remote player if not found
        if !updated {
            let cube = meshes.add(Cuboid::new(0.8, 1.8, 0.8)); // Player-sized cube
            let material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.8, 0.2), // Green for remote players
                ..Default::default()
            });

            commands.spawn((
                Mesh3d(cube),
                MeshMaterial3d(material),
                Transform::from_translation(Vec3::new(msg.pos[0], msg.pos[1], msg.pos[2])),
                RemotePlayer {
                    player_id: msg.player_id.clone(),
                },
                Name::new(format!("RemotePlayer-{}", msg.player_name)),
            ));

            info!(
                "New remote player joined: {} ({})",
                msg.player_name, msg.player_id
            );
        }
    }
}
