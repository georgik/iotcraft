use bevy::prelude::*;
use log::{error, info, warn};
use rumqttc::{Client, Event, Incoming, MqttOptions, QoS};
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::shared_world::*;
use crate::config::MqttConfig;
use crate::world::*;

/// Resource for managing world discovery
#[derive(Resource)]
pub struct WorldDiscovery {
    pub discovery_tx: std::sync::Mutex<Option<mpsc::Sender<DiscoveryMessage>>>,
    pub world_rx: std::sync::Mutex<Option<mpsc::Receiver<DiscoveryResponse>>>,
}

impl Default for WorldDiscovery {
    fn default() -> Self {
        Self {
            discovery_tx: std::sync::Mutex::new(None),
            world_rx: std::sync::Mutex::new(None),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DiscoveryMessage {
    RefreshWorlds,
    RequestWorldData { world_id: String },
}

#[derive(Debug, Clone)]
pub enum DiscoveryResponse {
    WorldListUpdated {
        worlds: HashMap<String, SharedWorldInfo>,
    },
    WorldDataReceived {
        world_id: String,
        world_data: WorldSaveData,
    },
    WorldChangeReceived {
        change: WorldChange,
    },
}

pub struct WorldDiscoveryPlugin;

impl Plugin for WorldDiscoveryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldDiscovery>()
            .add_systems(Startup, initialize_world_discovery)
            .add_systems(
                Update,
                (
                    handle_discovery_requests,
                    process_discovery_responses,
                    auto_refresh_worlds,
                ),
            );
    }
}

fn initialize_world_discovery(
    _commands: Commands,
    mqtt_config: Res<MqttConfig>,
    world_discovery: ResMut<WorldDiscovery>,
) {
    let (discovery_tx, discovery_rx) = mpsc::channel::<DiscoveryMessage>();
    let (response_tx, response_rx) = mpsc::channel::<DiscoveryResponse>();

    // Store channels in the resource
    *world_discovery.discovery_tx.lock().unwrap() = Some(discovery_tx);
    *world_discovery.world_rx.lock().unwrap() = Some(response_rx);

    let mqtt_host = mqtt_config.host.clone();
    let mqtt_port = mqtt_config.port;

    // Spawn discovery thread
    thread::spawn(move || {
        info!("Starting world discovery thread...");

        loop {
            let mut opts = MqttOptions::new("iotcraft-world-discovery", &mqtt_host, mqtt_port);
            opts.set_keep_alive(Duration::from_secs(30));
            opts.set_clean_session(false);

            let (client, mut conn) = Client::new(opts, 10);
            info!(
                "World discovery connecting to {}:{}...",
                mqtt_host, mqtt_port
            );

            let mut connected = false;
            let mut subscribed = false;
            let mut world_cache: HashMap<String, SharedWorldInfo> = HashMap::new();

            // Wait for connection and subscribe
            for event in conn.iter() {
                match event {
                    Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                        info!("World discovery connected successfully!");
                        connected = true;

                        // Subscribe to world discovery topics
                        if let Err(e) = client.subscribe("iotcraft/worlds/+/info", QoS::AtLeastOnce)
                        {
                            error!("Failed to subscribe to world info: {}", e);
                            break;
                        }
                        if let Err(e) = client.subscribe("iotcraft/worlds/+/data", QoS::AtLeastOnce)
                        {
                            error!("Failed to subscribe to world data: {}", e);
                            break;
                        }
                        if let Err(e) =
                            client.subscribe("iotcraft/worlds/+/changes", QoS::AtLeastOnce)
                        {
                            error!("Failed to subscribe to world changes: {}", e);
                            break;
                        }

                        info!("Subscribed to world discovery topics");
                        subscribed = true;
                        break;
                    }
                    Ok(Event::Incoming(Incoming::Publish(p))) => {
                        if !subscribed {
                            continue;
                        }

                        handle_discovery_message(&p, &mut world_cache, &response_tx);
                    }
                    Ok(_) => {}
                    Err(e) => {
                        error!("World discovery connection error: {:?}", e);
                        break;
                    }
                }
            }

            if !connected {
                error!("Failed to establish world discovery connection");
                thread::sleep(Duration::from_secs(5));
                continue;
            }

            // Handle discovery and connection events
            loop {
                // Handle connection events (non-blocking)
                match conn.try_recv() {
                    Ok(Ok(Event::Incoming(Incoming::Publish(p)))) => {
                        handle_discovery_message(&p, &mut world_cache, &response_tx);
                    }
                    Ok(Ok(_)) => {}
                    Ok(Err(e)) => {
                        error!("World discovery connection error: {:?}", e);
                        break;
                    }
                    Err(rumqttc::TryRecvError::Empty) => {}
                    Err(rumqttc::TryRecvError::Disconnected) => {
                        error!("World discovery connection lost");
                        break;
                    }
                }

                // Handle discovery requests (non-blocking)
                match discovery_rx.try_recv() {
                    Ok(DiscoveryMessage::RefreshWorlds) => {
                        // Send current world cache
                        let _ = response_tx.send(DiscoveryResponse::WorldListUpdated {
                            worlds: world_cache.clone(),
                        });
                    }
                    Ok(DiscoveryMessage::RequestWorldData { world_id }) => {
                        // Request specific world data (this would trigger a separate request)
                        info!("Requesting world data for: {}", world_id);
                        // In a real implementation, we might need to request this specifically
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                    Err(mpsc::TryRecvError::Disconnected) => {
                        warn!("World discovery channel disconnected");
                        break;
                    }
                }

                thread::sleep(Duration::from_millis(10));
            }

            error!("World discovery disconnected, reconnecting in 5 seconds...");
            thread::sleep(Duration::from_secs(5));
        }
    });

    info!("World discovery initialized");
}

fn handle_discovery_message(
    publish: &rumqttc::Publish,
    world_cache: &mut HashMap<String, SharedWorldInfo>,
    response_tx: &mpsc::Sender<DiscoveryResponse>,
) {
    let topic_parts: Vec<&str> = publish.topic.split('/').collect();

    if topic_parts.len() < 4 {
        return;
    }

    let world_id = topic_parts[2];
    let message_type = topic_parts[3];

    match message_type {
        "info" => {
            if publish.payload.is_empty() {
                // Empty message means world was unpublished
                world_cache.remove(world_id);
                info!("World {} was unpublished", world_id);
            } else {
                // Parse world info
                match String::from_utf8(publish.payload.to_vec()) {
                    Ok(payload_str) => {
                        match serde_json::from_str::<SharedWorldInfo>(&payload_str) {
                            Ok(world_info) => {
                                info!(
                                    "Discovered world: {} by {}",
                                    world_info.world_name, world_info.host_name
                                );
                                world_cache.insert(world_id.to_string(), world_info);
                            }
                            Err(e) => {
                                error!("Failed to parse world info: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to decode world info payload: {}", e);
                    }
                }
            }

            // Notify about updated world list
            let _ = response_tx.send(DiscoveryResponse::WorldListUpdated {
                worlds: world_cache.clone(),
            });
        }
        "data" => {
            if !publish.payload.is_empty() {
                match String::from_utf8(publish.payload.to_vec()) {
                    Ok(payload_str) => match serde_json::from_str::<WorldSaveData>(&payload_str) {
                        Ok(world_data) => {
                            info!("Received world data for: {}", world_id);
                            let _ = response_tx.send(DiscoveryResponse::WorldDataReceived {
                                world_id: world_id.to_string(),
                                world_data,
                            });
                        }
                        Err(e) => {
                            error!("Failed to parse world data: {}", e);
                        }
                    },
                    Err(e) => {
                        error!("Failed to decode world data payload: {}", e);
                    }
                }
            }
        }
        "changes" => match String::from_utf8(publish.payload.to_vec()) {
            Ok(payload_str) => match serde_json::from_str::<WorldChange>(&payload_str) {
                Ok(change) => {
                    let _ = response_tx.send(DiscoveryResponse::WorldChangeReceived { change });
                }
                Err(e) => {
                    error!("Failed to parse world change: {}", e);
                }
            },
            Err(e) => {
                error!("Failed to decode world change payload: {}", e);
            }
        },
        _ => {
            // Unknown message type
        }
    }
}

fn handle_discovery_requests(
    mut refresh_events: EventReader<RefreshOnlineWorldsEvent>,
    world_discovery: Res<WorldDiscovery>,
) {
    for _event in refresh_events.read() {
        if let Some(tx) = world_discovery.discovery_tx.lock().unwrap().as_ref() {
            let _ = tx.send(DiscoveryMessage::RefreshWorlds);
        }
    }
}

fn process_discovery_responses(
    world_discovery: Res<WorldDiscovery>,
    mut online_worlds: ResMut<OnlineWorlds>,
    mut commands: Commands,
    mut voxel_world: ResMut<crate::environment::VoxelWorld>,
    existing_blocks_query: Query<Entity, With<crate::environment::VoxelBlock>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut inventory: ResMut<crate::inventory::PlayerInventory>,
    camera_query: Query<Entity, With<crate::camera_controllers::CameraController>>,
    multiplayer_mode: Res<MultiplayerMode>,
) {
    if let Some(rx) = world_discovery.world_rx.lock().unwrap().as_ref() {
        while let Ok(response) = rx.try_recv() {
            match response {
                DiscoveryResponse::WorldListUpdated { worlds } => {
                    online_worlds.worlds = worlds;
                    online_worlds.last_updated = Some(std::time::Instant::now());
                    info!(
                        "Updated online worlds list: {} worlds available",
                        online_worlds.worlds.len()
                    );
                }
                DiscoveryResponse::WorldDataReceived {
                    world_id,
                    world_data,
                } => {
                    // Only load world data if we're joining this specific world
                    if let MultiplayerMode::JoinedWorld {
                        world_id: joined_id,
                        ..
                    } = &*multiplayer_mode
                    {
                        if *joined_id == world_id {
                            info!("Loading shared world data for: {}", world_id);
                            load_shared_world_data(
                                world_data,
                                &mut commands,
                                &mut voxel_world,
                                &existing_blocks_query,
                                &mut meshes,
                                &mut materials,
                                &asset_server,
                                &mut inventory,
                                &camera_query,
                            );
                        }
                    }
                }
                DiscoveryResponse::WorldChangeReceived { change } => {
                    // Apply world changes if we're in the same world
                    match &*multiplayer_mode {
                        MultiplayerMode::JoinedWorld { world_id, .. }
                        | MultiplayerMode::HostingWorld { world_id, .. } => {
                            if *world_id == change.world_id {
                                apply_world_change(
                                    change,
                                    &mut commands,
                                    &mut voxel_world,
                                    &mut meshes,
                                    &mut materials,
                                    &asset_server,
                                );
                            }
                        }
                        MultiplayerMode::SinglePlayer => {}
                    }
                }
            }
        }
    }
}

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
