use bevy::prelude::*;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::world::WorldSaveData;

/// Represents a shared world in the multiplayer system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SharedWorldInfo {
    pub world_id: String,
    pub world_name: String,
    pub description: String,
    pub host_player: String,
    pub host_name: String,
    pub created_at: String,
    pub last_updated: String,
    pub player_count: u32,
    pub max_players: u32,
    pub is_public: bool,
    pub version: String,
}

/// Complete shared world data including the voxel data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedWorldData {
    pub info: SharedWorldInfo,
    pub world_data: WorldSaveData,
}

/// Represents changes made to a shared world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldChange {
    pub change_id: String,
    pub world_id: String,
    pub player_id: String,
    pub player_name: String,
    pub timestamp: u64,
    pub change_type: WorldChangeType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorldChangeType {
    BlockPlaced {
        x: i32,
        y: i32,
        z: i32,
        block_type: crate::environment::BlockType,
    },
    BlockRemoved {
        x: i32,
        y: i32,
        z: i32,
    },
    PlayerJoined {
        player_id: String,
        player_name: String,
    },
    PlayerLeft {
        player_id: String,
        player_name: String,
    },
}

/// Resource to track the current multiplayer mode
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

/// Resource to track available online worlds
#[derive(Resource, Debug, Default)]
pub struct OnlineWorlds {
    pub worlds: HashMap<String, SharedWorldInfo>,
    pub world_data_cache: HashMap<String, crate::world::WorldSaveData>,
    pub last_updated: Option<std::time::Instant>,
}

/// Player position information for multiplayer status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerPosition {
    pub player_id: String,
    pub player_name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub last_updated: String,
}

/// Resource to track player positions in multiplayer
#[derive(Resource, Debug, Default)]
pub struct MultiplayerPlayerPositions {
    pub positions: HashMap<String, PlayerPosition>,
}

/// Events for multiplayer world management
#[derive(Event, BufferedEvent)]
pub struct PublishWorldEvent {
    pub world_name: String,
    pub max_players: u32,
    pub is_public: bool,
}

#[derive(Event, BufferedEvent)]
pub struct UnpublishWorldEvent {
    pub world_id: String,
}

#[derive(Event, BufferedEvent)]
pub struct JoinSharedWorldEvent {
    pub world_id: String,
}

#[derive(Event, BufferedEvent)]
pub struct LeaveSharedWorldEvent;

#[derive(Event, BufferedEvent)]
pub struct WorldChangeEvent {
    pub change: WorldChange,
}

#[derive(Event, BufferedEvent)]
pub struct RefreshOnlineWorldsEvent;

#[derive(Event, BufferedEvent)]
pub struct PlayerMoveEvent {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Event, BufferedEvent)]
pub struct WorldStateReceivedEvent {
    pub world_id: String,
    pub world_data: WorldSaveData,
}

#[derive(Event, BufferedEvent)]
pub struct BlockChangeEvent {
    pub world_id: String,
    pub player_id: String,
    pub player_name: String,
    pub change_type: BlockChangeType,
}

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

/// Plugin for shared world functionality
pub struct SharedWorldPlugin;

impl Plugin for SharedWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MultiplayerMode>()
            .init_resource::<OnlineWorlds>()
            .init_resource::<MultiplayerPlayerPositions>()
            .add_event::<PublishWorldEvent>()
            .add_event::<UnpublishWorldEvent>()
            .add_event::<JoinSharedWorldEvent>()
            .add_event::<LeaveSharedWorldEvent>()
            .add_event::<WorldChangeEvent>()
            .add_event::<RefreshOnlineWorldsEvent>()
            .add_event::<WorldStateReceivedEvent>()
            .add_event::<BlockChangeEvent>()
            .add_event::<PlayerMoveEvent>()
            .add_systems(
                Update,
                (
                    handle_publish_world_events,
                    handle_unpublish_world_events,
                    handle_join_shared_world_events,
                    handle_leave_shared_world_events,
                    handle_world_change_events,
                    handle_refresh_online_worlds_events,
                    handle_block_change_events,
                    handle_world_state_received_events,
                    auto_enable_multiplayer_when_mqtt_available,
                    auto_transition_to_game_on_multiplayer_changes,
                    track_local_player_position,
                    handle_player_move_events,
                ),
            );
    }
}

fn handle_publish_world_events(
    mut publish_events: EventReader<PublishWorldEvent>,
    current_world: Option<Res<crate::world::CurrentWorld>>,
    mut multiplayer_mode: ResMut<MultiplayerMode>,
) {
    for event in publish_events.read() {
        info!("Publishing world: {}", event.world_name);

        if let Some(current_world) = &current_world {
            // Generate a unique world ID
            let world_id = format!("{}_{}", current_world.name, chrono::Utc::now().timestamp());

            *multiplayer_mode = MultiplayerMode::HostingWorld {
                world_id: world_id.clone(),
                is_published: event.is_public,
            };

            info!(
                "World {} is now being hosted with ID: {} (public: {})",
                event.world_name, world_id, event.is_public
            );
            info!("Multiplayer mode changed to: {:?}", *multiplayer_mode);
        } else {
            warn!("Cannot publish world - no current world loaded");
        }
    }
}

fn handle_unpublish_world_events(
    mut unpublish_events: EventReader<UnpublishWorldEvent>,
    mut multiplayer_mode: ResMut<MultiplayerMode>,
) {
    for _event in unpublish_events.read() {
        info!("Unpublishing world");
        *multiplayer_mode = MultiplayerMode::SinglePlayer;
    }
}

fn handle_join_shared_world_events(
    mut join_events: EventReader<JoinSharedWorldEvent>,
    mut multiplayer_mode: ResMut<MultiplayerMode>,
    online_worlds: Res<OnlineWorlds>,
    mut world_state_events: EventWriter<WorldStateReceivedEvent>,
) {
    for event in join_events.read() {
        info!("Attempting to join shared world: {}", event.world_id);

        if let Some(world_info) = online_worlds.worlds.get(&event.world_id) {
            *multiplayer_mode = MultiplayerMode::JoinedWorld {
                world_id: event.world_id.clone(),
                host_player: world_info.host_player.clone(),
            };

            info!(
                "Joined world {} hosted by {}",
                world_info.world_name, world_info.host_name
            );

            // Check if we have cached world data and load it
            if let Some(world_data) = online_worlds.world_data_cache.get(&event.world_id) {
                info!(
                    "Found cached world data for: {}, triggering load",
                    event.world_id
                );
                world_state_events.write(WorldStateReceivedEvent {
                    world_id: event.world_id.clone(),
                    world_data: world_data.clone(),
                });
            } else {
                info!(
                    "No cached world data found for: {}, waiting for MQTT data",
                    event.world_id
                );
            }
        } else {
            error!("World {} not found in online worlds", event.world_id);
        }
    }
}

fn handle_leave_shared_world_events(
    mut leave_events: EventReader<LeaveSharedWorldEvent>,
    mut multiplayer_mode: ResMut<MultiplayerMode>,
) {
    for _event in leave_events.read() {
        info!("Leaving shared world");
        *multiplayer_mode = MultiplayerMode::SinglePlayer;
    }
}

fn handle_world_change_events(
    mut change_events: EventReader<WorldChangeEvent>,
    multiplayer_mode: Res<MultiplayerMode>,
) {
    for event in change_events.read() {
        match &*multiplayer_mode {
            MultiplayerMode::HostingWorld { .. } | MultiplayerMode::JoinedWorld { .. } => {
                // In multiplayer mode, broadcast changes
                info!("Broadcasting world change: {:?}", event.change.change_type);
                // TODO: Implement MQTT broadcasting
            }
            MultiplayerMode::SinglePlayer => {
                // In single player mode, changes are local only
            }
        }
    }
}

fn handle_refresh_online_worlds_events(
    mut refresh_events: EventReader<RefreshOnlineWorldsEvent>,
    mut online_worlds: ResMut<OnlineWorlds>,
) {
    for _event in refresh_events.read() {
        info!("Refreshing online worlds list");
        online_worlds.last_updated = Some(std::time::Instant::now());
        // TODO: Implement MQTT-based world discovery
    }
}

fn handle_block_change_events(
    mut block_change_events: EventReader<BlockChangeEvent>,
    world_publisher: Res<crate::multiplayer::world_publisher::WorldPublisher>,
    multiplayer_mode: Res<MultiplayerMode>,
) {
    use crate::multiplayer::world_publisher::PublishMessage;

    for event in block_change_events.read() {
        info!(
            "🎯 Received BlockChangeEvent for world {} by {}: {:?}",
            event.world_id, event.player_name, event.change_type
        );
        info!("🌍 Current multiplayer mode: {:?}", &*multiplayer_mode);

        match &*multiplayer_mode {
            MultiplayerMode::HostingWorld { world_id, .. }
            | MultiplayerMode::JoinedWorld { world_id, .. } => {
                info!("✅ In multiplayer mode with world_id: {}", world_id);
                if event.world_id == *world_id {
                    info!(
                        "🚀 World IDs match! Publishing block change for world {}: {:?} by {}",
                        world_id, event.change_type, event.player_name
                    );

                    let publish_tx_available = world_publisher.publish_tx.lock().unwrap().is_some();
                    info!("📡 World publisher TX available: {}", publish_tx_available);

                    if let Some(tx) = world_publisher.publish_tx.lock().unwrap().as_ref() {
                        info!("📤 Sending block change to MQTT publisher thread...");

                        let send_result = tx.send(PublishMessage::PublishBlockChange {
                            world_id: event.world_id.clone(),
                            player_id: event.player_id.clone(),
                            player_name: event.player_name.clone(),
                            change_type: event.change_type.clone(),
                        });

                        match send_result {
                            Ok(()) => {
                                info!("✅ Successfully sent block change to MQTT publisher!");
                            }
                            Err(e) => {
                                error!("❌ Failed to send block change publish message: {}", e);
                            }
                        }
                    } else {
                        error!(
                            "❌ World publisher TX channel is None - MQTT publisher not initialized!"
                        );
                    }
                } else {
                    warn!(
                        "⚠️  World ID mismatch: event world {} != current world {}",
                        event.world_id, world_id
                    );
                }
            }
            MultiplayerMode::SinglePlayer => {
                info!("🚫 In SinglePlayer mode, skipping MQTT publishing for block change");
            }
        }
    }
}

fn handle_world_state_received_events(
    mut world_state_events: EventReader<WorldStateReceivedEvent>,
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
    for event in world_state_events.read() {
        // Only load world data if we're currently in the specified world
        if let MultiplayerMode::JoinedWorld {
            world_id: joined_id,
            ..
        } = &*multiplayer_mode
        {
            if *joined_id == event.world_id {
                info!(
                    "Loading shared world state for: {} ({} blocks)",
                    event.world_id,
                    event.world_data.blocks.len()
                );
                load_shared_world_data(
                    &event.world_data,
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
}

fn load_shared_world_data(
    world_data: &WorldSaveData,
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
    let cleared_entities = existing_blocks_query.iter().count();
    for entity in existing_blocks_query.iter() {
        commands.entity(entity).despawn();
    }
    info!(
        "Cleared {} existing block entities from scene",
        cleared_entities
    );
    voxel_world.blocks.clear();

    // Load blocks
    for block_data in &world_data.blocks {
        voxel_world.blocks.insert(
            IVec3::new(block_data.x, block_data.y, block_data.z),
            block_data.block_type,
        );
    }
    info!("Loaded {} blocks into VoxelWorld", voxel_world.blocks.len());

    // Spawn visual blocks
    let mut spawned_blocks = 0;
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
        spawned_blocks += 1;
    }
    info!("Spawned {} visual block entities", spawned_blocks);

    // Load inventory
    *inventory = world_data.inventory.clone();
    inventory.ensure_proper_size();
    // ResMut automatically marks resources as changed when mutated

    // Set player position if camera exists
    if let Ok(camera_entity) = camera_query.single() {
        commands.entity(camera_entity).insert(Transform {
            translation: world_data.player_position,
            rotation: world_data.player_rotation,
            ..default()
        });
        info!("Set player position to: {:?}", world_data.player_position);
    }

    info!("Successfully loaded shared world data");
}

/// System that automatically enables multiplayer mode when MQTT is available and a world is loaded
/// This only runs during InGame state to avoid interfering with menu navigation
fn auto_enable_multiplayer_when_mqtt_available(
    mut multiplayer_mode: ResMut<MultiplayerMode>,
    multiplayer_status: Res<crate::multiplayer::MultiplayerConnectionStatus>,
    current_world: Option<Res<crate::world::CurrentWorld>>,
    player_profile: Res<crate::profile::PlayerProfile>,
    game_state: Res<State<crate::ui::GameState>>,
) {
    // Only auto-enable if we're currently in SinglePlayer mode AND in InGame state
    // This prevents auto-enabling when user is navigating menus
    if let MultiplayerMode::SinglePlayer = &*multiplayer_mode {
        // Only auto-enable during InGame state to avoid interfering with menu navigation
        if *game_state.get() != crate::ui::GameState::InGame {
            return;
        }

        // Check if MQTT is available and we have a world loaded
        if multiplayer_status.connection_available && current_world.is_some() {
            let current_world = current_world.unwrap();

            // Generate a world ID based on the current world name and player ID
            let world_id = format!("{}_{}", current_world.name, player_profile.player_id);

            info!(
                "🚀 Auto-enabling multiplayer mode! MQTT available and world '{}' loaded. World ID: {}",
                current_world.name, world_id
            );

            *multiplayer_mode = MultiplayerMode::HostingWorld {
                world_id,
                is_published: false, // Auto-enabled as private by default
            };

            info!(
                "✅ Multiplayer mode automatically set to: {:?}",
                *multiplayer_mode
            );
        }
    }
}

/// System that automatically transitions to InGame state when multiplayer mode changes from MCP commands
/// This ensures that when publish_world or join_world is called via MCP, the game properly enters gameplay
/// BUT respects when a user explicitly chooses to stay in the main menu
fn auto_transition_to_game_on_multiplayer_changes(
    multiplayer_mode: Res<MultiplayerMode>,
    current_state: Res<State<crate::ui::GameState>>,
    mut next_state: ResMut<NextState<crate::ui::GameState>>,
) {
    // Only transition if we're currently in WorldSelection and multiplayer mode becomes active
    // DO NOT transition from MainMenu, as that interferes with "quit to main menu" functionality
    if matches!(*current_state.get(), crate::ui::GameState::WorldSelection) {
        match &*multiplayer_mode {
            MultiplayerMode::HostingWorld { .. } | MultiplayerMode::JoinedWorld { .. } => {
                info!(
                    "🎮 Multiplayer mode became active ({:?}) while in {:?} - transitioning to InGame",
                    &*multiplayer_mode,
                    *current_state.get()
                );
                next_state.set(crate::ui::GameState::InGame);
            }
            MultiplayerMode::SinglePlayer => {
                // Don't transition for SinglePlayer mode - this is normal when unpublishing
            }
        }
    }
    // When in MainMenu state, respect the user's choice to stay there even with active multiplayer
}

/// System that tracks the local player's position and updates the multiplayer position data
fn track_local_player_position(
    mut player_positions: ResMut<MultiplayerPlayerPositions>,
    player_profile: Res<crate::profile::PlayerProfile>,
    multiplayer_mode: Res<MultiplayerMode>,
    camera_query: Query<&Transform, (With<Camera>, Without<crate::player_avatar::PlayerAvatar>)>,
) {
    // Only track position in multiplayer modes
    match &*multiplayer_mode {
        MultiplayerMode::SinglePlayer => return,
        _ => {}
    }

    // Get camera/player position
    if let Ok(camera_transform) = camera_query.single() {
        let position = PlayerPosition {
            player_id: player_profile.player_id.clone(),
            player_name: player_profile.player_name.clone(),
            x: camera_transform.translation.x,
            y: camera_transform.translation.y,
            z: camera_transform.translation.z,
            last_updated: chrono::Utc::now().to_rfc3339(),
        };

        player_positions
            .positions
            .insert(player_profile.player_id.clone(), position);
    }
}

/// System that handles PlayerMoveEvent by updating camera position and player position tracking
fn handle_player_move_events(
    mut move_events: EventReader<PlayerMoveEvent>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
    mut player_positions: ResMut<MultiplayerPlayerPositions>,
    player_profile: Res<crate::profile::PlayerProfile>,
    multiplayer_mode: Res<MultiplayerMode>,
) {
    for event in move_events.read() {
        info!(
            "Handling player move event: ({}, {}, {})",
            event.x, event.y, event.z
        );

        // Try to move any camera (simplified query to avoid conflicts)
        for mut camera_transform in camera_query.iter_mut() {
            camera_transform.translation = Vec3::new(event.x, event.y, event.z);
            info!("Moved camera to: ({}, {}, {})", event.x, event.y, event.z);
            break; // Only move the first camera
        }

        // Update player position tracking immediately (for any multiplayer mode)
        match &*multiplayer_mode {
            MultiplayerMode::SinglePlayer => {} // Don't track in single player
            _ => {
                let position = PlayerPosition {
                    player_id: player_profile.player_id.clone(),
                    player_name: player_profile.player_name.clone(),
                    x: event.x,
                    y: event.y,
                    z: event.z,
                    last_updated: chrono::Utc::now().to_rfc3339(),
                };

                player_positions
                    .positions
                    .insert(player_profile.player_id.clone(), position);

                info!(
                    "Updated position tracking for player {} to ({}, {}, {})",
                    player_profile.player_id, event.x, event.y, event.z
                );
            }
        }
    }
}
