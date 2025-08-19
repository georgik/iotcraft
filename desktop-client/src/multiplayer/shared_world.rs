use bevy::prelude::*;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::world::WorldSaveData;

/// Represents a shared world in the multiplayer system
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub last_updated: Option<std::time::Instant>,
}

/// Events for multiplayer world management
#[derive(Event)]
pub struct PublishWorldEvent {
    pub world_name: String,
    pub max_players: u32,
    pub is_public: bool,
}

#[derive(Event)]
pub struct UnpublishWorldEvent {
    pub world_id: String,
}

#[derive(Event)]
pub struct JoinSharedWorldEvent {
    pub world_id: String,
}

#[derive(Event)]
pub struct LeaveSharedWorldEvent;

#[derive(Event)]
pub struct WorldChangeEvent {
    pub change: WorldChange,
}

#[derive(Event)]
pub struct RefreshOnlineWorldsEvent;

/// New events for world state synchronization
#[derive(Event)]
pub struct PublishWorldStateEvent {
    pub world_id: String,
    pub force_full_snapshot: bool,
}

#[derive(Event)]
pub struct RequestWorldStateEvent {
    pub world_id: String,
}

#[derive(Event)]
pub struct WorldStateReceivedEvent {
    pub world_id: String,
    pub world_data: WorldSaveData,
}

#[derive(Event)]
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

#[derive(Event)]
pub struct InventoryChangeEvent {
    pub world_id: String,
    pub player_id: String,
    pub inventory: crate::inventory::PlayerInventory,
}

/// Plugin for shared world functionality
pub struct SharedWorldPlugin;

impl Plugin for SharedWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MultiplayerMode>()
            .init_resource::<OnlineWorlds>()
            .add_event::<PublishWorldEvent>()
            .add_event::<UnpublishWorldEvent>()
            .add_event::<JoinSharedWorldEvent>()
            .add_event::<LeaveSharedWorldEvent>()
            .add_event::<WorldChangeEvent>()
            .add_event::<RefreshOnlineWorldsEvent>()
            // New world state synchronization events
            .add_event::<PublishWorldStateEvent>()
            .add_event::<RequestWorldStateEvent>()
            .add_event::<WorldStateReceivedEvent>()
            .add_event::<BlockChangeEvent>()
            .add_event::<InventoryChangeEvent>()
            .add_systems(
                Update,
                (
                    handle_publish_world_events,
                    handle_unpublish_world_events,
                    handle_join_shared_world_events,
                    handle_leave_shared_world_events,
                    handle_world_change_events,
                    handle_refresh_online_worlds_events,
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
                "World {} is now being hosted with ID: {}",
                event.world_name, world_id
            );
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
