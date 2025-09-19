use bevy::prelude::*;
use log::{info, warn};
use std::collections::HashMap;
use std::time::Duration;

use super::shared_world::*;
use crate::world::*;

/// Data structure to hold information about the last received message on a topic
#[derive(Debug, Clone)]
pub struct LastMessage {
    pub content: String,
}

impl std::fmt::Display for LastMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.content)
    }
}

/// Legacy resource kept for last message tracking only
/// Main world discovery now handled by Core MQTT Service
#[derive(Resource, Default)]
pub struct WorldDiscovery {
    pub last_messages: std::sync::Mutex<HashMap<String, LastMessage>>,
}

pub struct WorldDiscoveryPlugin;

impl Plugin for WorldDiscoveryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldDiscovery>().add_systems(
            Update,
            (
                process_core_mqtt_world_info,
                process_core_mqtt_world_data,
                auto_refresh_worlds,
            ),
        );
    }
}

// All legacy MQTT message handling has been removed
// The Core MQTT Service now handles all MQTT communication and routing

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

/// Process world info messages from Core MQTT Service
/// These arrive from the unified MQTT connection and need to be injected into world discovery
fn process_core_mqtt_world_info(
    world_discovery_rx: Option<Res<crate::mqtt::core_service::WorldDiscoveryReceiver>>,
    mut online_worlds: ResMut<OnlineWorlds>,
) {
    if let Some(receiver) = world_discovery_rx {
        if let Ok(rx) = receiver.0.lock() {
            while let Ok(world_info) = rx.try_recv() {
                info!(
                    "🌍 Processing world info from Core MQTT Service: {} ({})",
                    world_info.world_name, world_info.world_id
                );

                // Add to online worlds cache directly
                online_worlds
                    .worlds
                    .insert(world_info.world_id.clone(), world_info);
                online_worlds.last_updated = Some(std::time::Instant::now());

                info!(
                    "✅ Added world to cache. Total online worlds: {}",
                    online_worlds.worlds.len()
                );
            }
        } else {
            warn!("⚠️ Could not lock WorldDiscoveryReceiver mutex");
        }
    } else {
        // Add periodic debug logging to see if this system is running but resource missing
        use std::sync::atomic::{AtomicU32, Ordering};
        static DEBUG_COUNTER: AtomicU32 = AtomicU32::new(0);

        let count = DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
        if count % 300 == 0 {
            // Log every ~5 seconds (60fps * 5 = 300 frames)
            warn!(
                "⚠️ process_core_mqtt_world_info: WorldDiscoveryReceiver resource not found (check #{}: {})",
                count / 300,
                count
            );
        }
    }
}

/// Process world data messages from Core MQTT Service
/// These arrive from the unified MQTT connection for complete world synchronization
fn process_core_mqtt_world_data(
    world_data_rx: Option<Res<crate::mqtt::core_service::WorldDataReceiver>>,
    mut online_worlds: ResMut<OnlineWorlds>,
) {
    if let Some(receiver) = world_data_rx {
        if let Ok(rx) = receiver.0.lock() {
            while let Ok((world_id, world_data)) = rx.try_recv() {
                info!(
                    "🌍 Processing world data from Core MQTT Service: {} ({} blocks)",
                    world_id,
                    world_data.blocks.len()
                );

                // Cache the world data for joining
                online_worlds
                    .world_data_cache
                    .insert(world_id.clone(), world_data);

                info!(
                    "✅ Cached world data for {}. Total cached worlds: {}",
                    world_id,
                    online_worlds.world_data_cache.len()
                );
            }
        }
    }
}
