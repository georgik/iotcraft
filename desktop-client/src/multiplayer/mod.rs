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
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::sync::mpsc;
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
pub struct PoseMessage {
    pub player_id: String,
    pub player_name: String,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub ts: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DisconnectMessage {
    player_id: String,
    ts: u64,
}

#[derive(Resource)]
pub struct PoseRx(pub Mutex<mpsc::Receiver<PoseMessage>>);

#[derive(Resource)]
pub struct PoseTx(pub Mutex<mpsc::Sender<PoseMessage>>);

// Note: MQTT status management now handled by Core MQTT Service

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

// Initialize multiplayer - now using Core MQTT Service for connections
fn start_multiplayer_connections(
    mut commands: Commands,
    _mqtt: Res<MqttConfig>,
    _world: Res<WorldId>,
    _profile: Res<PlayerProfile>,
) {
    // Note: PoseRx and PoseTx are now provided by CoreMqttServicePlugin
    // We just need to set up the multiplayer status and timer

    commands.insert_resource(PositionTimer::default());

    // Start with multiplayer enabled - Core MQTT Service will handle availability
    commands.insert_resource(MultiplayerConnectionStatus {
        connection_available: true, // Assume available, Core MQTT Service will manage this
    });
    info!("✅ Multiplayer initialized - using Core MQTT Service for pose communication");
}

fn update_position_timer(mut timer: ResMut<PositionTimer>, time: Res<Time>) {
    timer.timer.tick(time.delta());
}

// Note: MQTT connection status is now managed by Core MQTT Service

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
