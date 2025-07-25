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
    // Future interaction types can be added here
    // ToggleSwitch,
    // ReadSensor,
}

/// Component for lamp devices that can be toggled
#[derive(Component)]
pub struct LampState {
    pub is_on: bool,
    pub device_id: String,
}

/// Resource to track the currently hovered interactable entity
#[derive(Resource, Default)]
pub struct HoveredEntity {
    pub entity: Option<Entity>,
}

/// Event sent when a player interacts with a block
#[derive(Event)]
pub struct InteractionEvent {
    pub entity: Entity,
    pub interaction_type: InteractionType,
}

/// Event sent when a lamp state should be changed
#[derive(Event)]
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

/// Visual indicator for the player's "hand" or interaction cursor
#[derive(Component)]
pub struct PlayerHand;

/// Material handles for lamp states
#[derive(Resource)]
pub struct LampMaterials {
    pub lamp_off: Handle<StandardMaterial>,
    pub lamp_on: Handle<StandardMaterial>,
    pub hovered: Handle<StandardMaterial>,
}
