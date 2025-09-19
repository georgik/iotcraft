use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Component that marks an entity as interactable
#[derive(Component)]
pub struct Interactable {
    pub interaction_type: InteractionType,
}

/// Types of interactions available
#[derive(Debug, Clone)]
pub enum InteractionType {
    ToggleLamp,
    ToggleDoor,
}

/// Component for lamp devices that can be toggled
#[derive(Component)]
pub struct LampState {
    pub is_on: bool,
}

/// Event sent when a door state should be changed
#[derive(Event, BufferedEvent)]
pub struct DoorToggleEvent {
    pub device_id: String,
    pub new_state: bool, // true for open, false for closed
}

/// Resource tracking which entity is currently being hovered over
#[derive(Resource, Default)]
pub struct HoveredEntity {
    pub entity: Option<Entity>,
}

/// Resource tracking ghost block preview state
#[derive(Resource, Default)]
pub struct GhostBlockState {
    pub target_block_position: Option<IVec3>, // Position of existing block that would be broken (highlighted)
    pub placement_position: Option<IVec3>, // Position where new block would be placed (adjacent to target)
    pub can_place: bool,
}

/// Event sent when a player interacts with a block
#[derive(Event, BufferedEvent)]
pub struct InteractionEvent {
    pub entity: Entity,
    pub interaction_type: InteractionType,
}

/// Event sent when a lamp state should be changed
#[derive(Event, BufferedEvent)]
pub struct LampToggleEvent {
    pub device_id: String,
    pub new_state: bool,
}

/// Data structure for MQTT lamp control messages
#[derive(Serialize, Deserialize)]
pub struct LampControlMessage {
    pub device_id: String,
    pub state: String,  // "ON" or "OFF"
    pub source: String, // "player_interaction"
}

/// Material handles for lamp states
#[derive(Resource)]
pub struct LampMaterials {
    pub lamp_off: Handle<StandardMaterial>,
    pub lamp_on: Handle<StandardMaterial>,
}

/// Material handles for door states
#[derive(Resource)]
pub struct DoorMaterials {
    // Currently unused - doors use transform-based animations instead of material changes
    // Keeping the struct for potential future use
}
