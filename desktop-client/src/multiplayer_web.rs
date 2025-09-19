//! Web-compatible multiplayer module with shared types and MQTT-based synchronization
//! This module provides block change synchronization and multiplayer features for WASM

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::sync::mpsc;

/// Block change event for multiplayer synchronization
#[derive(Event, BufferedEvent, Debug, Clone, Serialize, Deserialize)]
pub struct BlockChangeEvent {
    pub world_id: String,
    pub player_id: String,
    pub player_name: String,
    pub change_type: BlockChangeType,
}

/// Types of block changes that can be synchronized
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockChangeType {
    Placed {
        x: i32,
        y: i32,
        z: i32,
        block_type: crate::environment::BlockType,
    },
    Removed {
        x: i32,
        y: i32,
        z: i32,
    },
}

/// Multiplayer mode tracking
#[derive(Resource, Debug, Clone, PartialEq)]
pub enum MultiplayerMode {
    /// Playing in single player mode - changes only local
    SinglePlayer,
    /// Hosting a world that others can join
    HostingWorld {
        world_id: String,
        is_published: bool,
    },
    /// Joined someone else's world
    JoinedWorld {
        world_id: String,
        host_player: String,
    },
}

impl Default for MultiplayerMode {
    fn default() -> Self {
        Self::SinglePlayer
    }
}

/// Resource to track multiplayer connection status
#[derive(Resource, Default)]
pub struct MultiplayerConnectionStatus {
    pub connection_available: bool,
}

impl MultiplayerConnectionStatus {
    pub fn is_multiplayer_enabled(&self) -> bool {
        self.connection_available
    }
}

/// Channel for sending block changes to MQTT publisher
#[derive(Resource)]
pub struct BlockChangeSender(pub Mutex<mpsc::Sender<BlockChangeEvent>>);

/// Channel for receiving block changes from MQTT subscriber
#[derive(Resource)]
pub struct BlockChangeReceiver(pub Mutex<mpsc::Receiver<BlockChangeEvent>>);

/// Shared world ID resource
#[derive(Resource, Debug, Clone)]
pub struct WorldId(pub String);

impl Default for WorldId {
    fn default() -> Self {
        Self("default".to_string())
    }
}

/// Component to mark remote player entities
#[derive(Component)]
pub struct RemotePlayer;

/// Pose message for player position synchronization (re-exported from mqtt::web)
pub use crate::mqtt::web::{PoseMessage, PoseReceiver, PoseSender};

/// Web multiplayer plugin that provides block synchronization
pub struct WebMultiplayerPlugin;

impl Plugin for WebMultiplayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MultiplayerMode>()
            .init_resource::<MultiplayerConnectionStatus>()
            .init_resource::<WorldId>()
            .add_event::<BlockChangeEvent>()
            .add_systems(Startup, setup_block_change_connections)
            .add_systems(
                Update,
                (handle_outgoing_block_changes, handle_incoming_block_changes),
            );
    }
}

/// Set up MQTT connections for block change synchronization
fn setup_block_change_connections(
    mut commands: Commands,
    _mqtt_config: Res<crate::config::MqttConfig>,
    _world_id: Res<WorldId>,
    _profile: Res<crate::profile::PlayerProfile>,
) {
    let (outgoing_tx, _outgoing_rx) = mpsc::channel::<BlockChangeEvent>();
    let (_incoming_tx, incoming_rx) = mpsc::channel::<BlockChangeEvent>();

    commands.insert_resource(BlockChangeSender(Mutex::new(outgoing_tx)));
    commands.insert_resource(BlockChangeReceiver(Mutex::new(incoming_rx)));

    // Enable multiplayer connection
    commands.insert_resource(MultiplayerConnectionStatus {
        connection_available: true,
    });

    info!("🌐 Block change multiplayer setup complete (simplified for initial web version)");

    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(
        &"🌐 Block change multiplayer setup complete (simplified for initial web version)".into(),
    );
}

/// System to handle outgoing block changes (send to MQTT)
fn handle_outgoing_block_changes(
    mut block_change_events: EventReader<BlockChangeEvent>,
    block_change_sender: Option<Res<BlockChangeSender>>,
    connection_status: Res<MultiplayerConnectionStatus>,
) {
    if !connection_status.is_multiplayer_enabled() {
        return;
    }

    let Some(sender) = block_change_sender else {
        return;
    };

    let Ok(tx) = sender.0.lock() else {
        return;
    };

    for event in block_change_events.read() {
        if let Err(_) = tx.send(event.clone()) {
            error!("Failed to send block change to MQTT publisher");
        } else {
            info!("📡 Sent block change: {:?}", event.change_type);
        }
    }
}

/// System to handle incoming block changes (apply to world)
fn handle_incoming_block_changes(
    mut commands: Commands,
    block_change_receiver: Option<Res<BlockChangeReceiver>>,
    mut voxel_world: ResMut<crate::environment::VoxelWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    existing_blocks: Query<(Entity, &crate::environment::VoxelBlock)>,
    connection_status: Res<MultiplayerConnectionStatus>,
) {
    if !connection_status.is_multiplayer_enabled() {
        return;
    }

    let Some(receiver) = block_change_receiver else {
        return;
    };

    let Ok(rx) = receiver.0.lock() else {
        return;
    };

    while let Ok(block_change) = rx.try_recv() {
        info!(
            "📡 Received block change from {}: {:?}",
            block_change.player_name, block_change.change_type
        );

        match block_change.change_type {
            BlockChangeType::Placed {
                x,
                y,
                z,
                block_type,
            } => {
                let position = IVec3::new(x, y, z);

                // Add block to voxel world
                voxel_world.set_block(position, block_type);

                // Create visual representation
                let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

                let material = match block_type {
                    crate::environment::BlockType::Water => materials.add(StandardMaterial {
                        base_color: Color::srgba(0.0, 0.35, 0.9, 0.6),
                        alpha_mode: AlphaMode::Blend,
                        ..default()
                    }),
                    _ => {
                        let texture_path = match block_type {
                            crate::environment::BlockType::Grass => "textures/grass.webp",
                            crate::environment::BlockType::Dirt => "textures/dirt.webp",
                            crate::environment::BlockType::Stone => "textures/stone.webp",
                            crate::environment::BlockType::QuartzBlock => {
                                "textures/quartz_block.webp"
                            }
                            crate::environment::BlockType::GlassPane => "textures/glass_pane.webp",
                            crate::environment::BlockType::CyanTerracotta => {
                                "textures/cyan_terracotta.webp"
                            }
                            _ => "textures/dirt.webp", // fallback
                        };
                        let texture = asset_server.load(texture_path);
                        materials.add(StandardMaterial {
                            base_color_texture: Some(texture),
                            ..default()
                        })
                    }
                };

                commands.spawn((
                    Mesh3d(cube_mesh),
                    MeshMaterial3d(material),
                    Transform::from_translation(Vec3::new(x as f32, y as f32, z as f32)),
                    crate::environment::VoxelBlock { position },
                    Name::new(format!("RemoteBlock-{}-{}-{}", x, y, z)),
                ));

                info!(
                    "✅ Placed remote block {:?} at ({}, {}, {})",
                    block_type, x, y, z
                );

                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(
                    &format!(
                        "✅ Placed remote block {:?} at ({}, {}, {})",
                        block_type, x, y, z
                    )
                    .into(),
                );
            }
            BlockChangeType::Removed { x, y, z } => {
                let position = IVec3::new(x, y, z);

                // Remove from voxel world
                voxel_world.remove_block(&position);

                // Remove visual representation
                for (entity, block) in existing_blocks.iter() {
                    if block.position == position {
                        commands.entity(entity).despawn();
                        break;
                    }
                }

                info!("🗑️ Removed remote block at ({}, {}, {})", x, y, z);

                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(
                    &format!("🗑️ Removed remote block at ({}, {}, {})", x, y, z).into(),
                );
            }
        }
    }
}
