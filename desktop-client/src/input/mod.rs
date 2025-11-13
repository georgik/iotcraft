//! Input handling module for IoTCraft desktop client
//!
//! This module provides comprehensive input support including:
//! - Keyboard and mouse input (existing)
//! - Gamepad input (new)
//! - Input configuration and remapping
//! - Cross-platform compatibility

pub mod gamepad;
pub mod gamepad_config;
pub mod gamepad_plugin;

// Re-export key types for convenience
pub use gamepad::*;
pub use gamepad_config::*;
pub use gamepad_plugin::*;

/// Actions that can be performed in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GameAction {
    // Movement
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Jump,
    Run,

    // Camera
    LookUp,
    LookDown,
    LookLeft,
    LookRight,
    ZoomIn,
    ZoomOut,

    // Building
    PlaceBlock,
    RemoveBlock,
    NextBlock,
    PreviousBlock,

    // Device interaction
    InteractDevice,
    ToggleDevice,

    // UI
    OpenMenu,
    CloseMenu,
    Console,
    ToggleMinimap,

    // System
    Screenshot,
    ToggleDebug,

    // Inventory
    SelectSlot1,
    SelectSlot2,
    SelectSlot3,
    SelectSlot4,
    SelectSlot5,
    SelectSlot6,
    SelectSlot7,
    SelectSlot8,
    SelectSlot9,
    NextSlot,
    PreviousSlot,
}

impl GameAction {
    /// Get the inventory slot number (1-9) for slot selection actions
    pub fn as_inventory_slot(self) -> Option<usize> {
        match self {
            GameAction::SelectSlot1 => Some(0),
            GameAction::SelectSlot2 => Some(1),
            GameAction::SelectSlot3 => Some(2),
            GameAction::SelectSlot4 => Some(3),
            GameAction::SelectSlot5 => Some(4),
            GameAction::SelectSlot6 => Some(5),
            GameAction::SelectSlot7 => Some(6),
            GameAction::SelectSlot8 => Some(7),
            GameAction::SelectSlot9 => Some(8),
            _ => None,
        }
    }
}
