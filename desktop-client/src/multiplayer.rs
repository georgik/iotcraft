use bevy::prelude::*;
use log::info;
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

pub struct MultiplayerPlugin;

impl Plugin for MultiplayerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WorldId::default())
            .add_systems(Startup, start_pose_subscriber)
            .add_systems(Update, (publish_local_pose, apply_remote_poses));
    }
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as u64
}

fn start_pose_subscriber(mut commands: Commands, mqtt: Res<MqttConfig>, world: Res<WorldId>) {
    let (tx, rx) = mpsc::channel::<PoseMessage>();
    commands.insert_resource(PoseRx(Mutex::new(rx)));

    let topic = format!("iotcraft/worlds/{}/players/+/pose", world.0);
    let host = mqtt.host.clone();
    let port = mqtt.port;

    thread::spawn(move || {
        let mut opts = MqttOptions::new("desktop-poses-sub", &host, port);
        opts.set_keep_alive(Duration::from_secs(5));
        let (client, mut conn) = Client::new(opts, 10);
        client.subscribe(&topic, QoS::AtMostOnce).ok();
        info!("Subscribed to poses: {}", topic);

        for n in conn.iter() {
            match n {
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    if let Ok(s) = String::from_utf8(p.payload.to_vec()) {
                        if let Ok(msg) = serde_json::from_str::<PoseMessage>(&s) {
                            // Push into channel; local filter is done in system
                            let _ = tx.send(msg);
                        }
                    }
                }
                Ok(Event::Incoming(Incoming::ConnAck(_))) => info!("Pose subscriber connected"),
                Ok(_) => {}
                Err(e) => {
                    bevy::log::error!("Pose subscriber error: {}", e);
                    break;
                }
            }
        }
    });
}

fn publish_local_pose(
    profile: Res<PlayerProfile>,
    world_id: Res<WorldId>,
    mqtt: Res<MqttConfig>,
    camera_q: Query<(&Transform,), With<Camera>>, // use camera transform as player pose
) {
    // Send at ~10 Hz: simple timer using system run frequency guard
    static mut LAST_SEND: u128 = 0;
    let now_ms = now_ts() as u128;
    let should_send = unsafe {
        if now_ms.saturating_sub(LAST_SEND) >= 100 {
            LAST_SEND = now_ms;
            true
        } else {
            false
        }
    };
    if !should_send {
        return;
    }

    let Ok((transform,)) = camera_q.single() else {
        return;
    };
    let (yaw, pitch) = {
        // Extract yaw/pitch from rotation: this is approximate for FPS cam
        let forward = transform.forward();
        let yaw = forward.x.atan2(forward.z);
        let pitch = forward.y.asin();
        (yaw, pitch)
    };

    let msg = PoseMessage {
        player_id: profile.player_id.clone(),
        player_name: profile.player_name.clone(),
        pos: [
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
        ],
        yaw,
        pitch,
        ts: now_ts(),
    };

    let payload = match serde_json::to_string(&msg) {
        Ok(s) => s,
        Err(_) => return,
    };
    let topic = format!(
        "iotcraft/worlds/{}/players/{}/pose",
        world_id.0, profile.player_id
    );

    // Fire-and-forget publish on a short thread so we don't block
    let host = mqtt.host.clone();
    let port = mqtt.port;
    thread::spawn(move || {
        let mut opts = MqttOptions::new("desktop-poses-pub", &host, port);
        opts.set_keep_alive(Duration::from_secs(5));
        let (client, mut conn) = Client::new(opts, 10);
        let _ = client.publish(&topic, QoS::AtMostOnce, false, payload.as_bytes());
        // drive until publish out to avoid drop before send
        for n in conn.iter() {
            if matches!(n, Ok(Event::Outgoing(Outgoing::Publish(_)))) {
                break;
            }
        }
    });
}

fn apply_remote_poses(
    profile: Res<PlayerProfile>,
    pose_rx: Res<PoseRx>,
    mut commands: Commands,
    mut q: Query<(&mut Transform, &RemotePlayer)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Pull all messages
    let Ok(rx) = pose_rx.0.lock() else {
        return;
    };
    while let Ok(msg) = rx.try_recv() {
        if msg.player_id == profile.player_id {
            continue;
        } // ignore our own
        // try update existing
        let mut updated = false;
        for (mut transform, rp) in q.iter_mut() {
            if rp.player_id == msg.player_id {
                transform.translation = Vec3::new(msg.pos[0], msg.pos[1], msg.pos[2]);
                updated = true;
                break;
            }
        }
        if !updated {
            // spawn simple cube
            let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
            let material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.6, 1.0),
                ..Default::default()
            });
            commands.spawn((
                Mesh3d(cube),
                MeshMaterial3d(material),
                Transform::from_translation(Vec3::new(msg.pos[0], msg.pos[1], msg.pos[2])),
                RemotePlayer {
                    player_id: msg.player_id.clone(),
                },
            ));
        }
    }
}
