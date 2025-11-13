//! Gamepad configuration and button mapping support
//!
//! Provides customizable gamepad controls with support for different controller types
//! and user preferences.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::GameAction;

/// Configuration for gamepad input handling
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct GamepadConfig {
    /// Whether gamepad input is enabled
    pub enabled: bool,

    /// Sensitivity multiplier for analog sticks
    pub sensitivity: f32,

    /// Deadzone for analog sticks (0.0 to 1.0)
    pub deadzone: f32,

    /// Whether vibration/rumble is enabled
    pub vibration_enabled: bool,

    /// Button mappings for different gamepad types
    pub button_mappings: HashMap<GamepadButtonType, GameAction>,

    /// Axis mappings for analog inputs
    pub axis_mappings: HashMap<GamepadAxisType, GameAxisAction>,

    /// Which gamepad to use (if multiple are connected)
    pub preferred_gamepad: Option<Entity>,

    /// Vibration intensity for different feedback types
    pub vibration_intensity: VibrationIntensity,
}

/// Vibration intensity settings for different types of feedback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VibrationIntensity {
    /// Intensity for device interaction (0.0 to 1.0)
    pub device_interaction: f32,

    /// Intensity for building actions (0.0 to 1.0)
    pub building: f32,

    /// Intensity for UI interactions (0.0 to 1.0)
    pub ui_interaction: f32,

    /// Intensity for damage/collision (0.0 to 1.0)
    pub collision: f32,
}

/// Gamepad button types using vendor-neutral naming
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GamepadButtonType {
    /// Face buttons
    South, // Usually A on Xbox, Cross on PlayStation
    East,  // Usually B on Xbox, Circle on PlayStation
    West,  // Usually X on Xbox, Square on PlayStation
    North, // Usually Y on Xbox, Triangle on PlayStation

    /// Shoulder buttons
    LeftTrigger,
    RightTrigger2,

    /// Trigger buttons
    LeftTrigger2,
    RightTrigger,

    /// D-pad directions
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,

    /// Center buttons
    Start,
    Select,

    /// Stick buttons (pressing the analog sticks)
    LeftThumb,
    RightThumb,
}

/// Gamepad axis types for analog inputs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GamepadAxisType {
    /// Left stick horizontal
    LeftStickX,
    /// Left stick vertical
    LeftStickY,
    /// Right stick horizontal
    RightStickX,
    /// Right stick vertical
    RightStickY,
}

/// Actions that can be performed with analog axes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameAxisAction {
    /// Camera/look controls
    LookHorizontal,
    LookVertical,

    /// Movement controls
    MoveHorizontal,
    MoveVertical,

    /// Zoom controls
    Zoom,

    /// Scrolling/menus
    Scroll,
}

impl Default for GamepadConfig {
    fn default() -> Self {
        let mut button_mappings = HashMap::new();
        let mut axis_mappings = HashMap::new();

        // Default Xbox-style button mapping
        button_mappings.insert(GamepadButtonType::South, GameAction::PlaceBlock);
        button_mappings.insert(GamepadButtonType::East, GameAction::RemoveBlock);
        button_mappings.insert(GamepadButtonType::West, GameAction::InteractDevice);
        button_mappings.insert(GamepadButtonType::North, GameAction::Jump);
        button_mappings.insert(GamepadButtonType::LeftTrigger, GameAction::PreviousSlot);
        button_mappings.insert(GamepadButtonType::RightTrigger2, GameAction::NextSlot);
        button_mappings.insert(GamepadButtonType::LeftTrigger2, GameAction::Run); // Use LT as run button
        button_mappings.insert(GamepadButtonType::RightTrigger, GameAction::RemoveBlock);
        button_mappings.insert(GamepadButtonType::Start, GameAction::OpenMenu);
        button_mappings.insert(GamepadButtonType::Select, GameAction::Console);
        // D-pad primary functions: movement
        button_mappings.insert(GamepadButtonType::DPadUp, GameAction::MoveForward);
        button_mappings.insert(GamepadButtonType::DPadDown, GameAction::MoveBackward);
        button_mappings.insert(GamepadButtonType::DPadLeft, GameAction::MoveLeft);
        button_mappings.insert(GamepadButtonType::DPadRight, GameAction::MoveRight);
        button_mappings.insert(GamepadButtonType::LeftThumb, GameAction::ToggleMinimap);

        // Default axis mapping
        axis_mappings.insert(GamepadAxisType::LeftStickX, GameAxisAction::MoveHorizontal);
        axis_mappings.insert(GamepadAxisType::LeftStickY, GameAxisAction::MoveVertical);
        axis_mappings.insert(GamepadAxisType::RightStickX, GameAxisAction::LookHorizontal);
        axis_mappings.insert(GamepadAxisType::RightStickY, GameAxisAction::LookVertical);

        Self {
            enabled: true,
            sensitivity: 1.0,
            deadzone: 0.1,
            vibration_enabled: true,
            button_mappings,
            axis_mappings,
            preferred_gamepad: None,
            vibration_intensity: VibrationIntensity::default(),
        }
    }
}

impl Default for VibrationIntensity {
    fn default() -> Self {
        Self {
            device_interaction: 0.7,
            building: 0.5,
            ui_interaction: 0.3,
            collision: 1.0,
        }
    }
}

impl GamepadButtonType {
    /// Convert from Bevy's GamepadButton to our button type
    pub fn from_bevy_button(button: bevy::input::gamepad::GamepadButton) -> Option<Self> {
        use bevy::input::gamepad::GamepadButton;

        match button {
            GamepadButton::South => Some(Self::South),
            GamepadButton::East => Some(Self::East),
            GamepadButton::West => Some(Self::West),
            GamepadButton::North => Some(Self::North),
            GamepadButton::LeftTrigger => Some(Self::LeftTrigger),
            GamepadButton::RightTrigger2 => Some(Self::RightTrigger2),
            GamepadButton::LeftTrigger2 => Some(Self::LeftTrigger2),
            GamepadButton::RightTrigger => Some(Self::RightTrigger),
            GamepadButton::DPadUp => Some(Self::DPadUp),
            GamepadButton::DPadDown => Some(Self::DPadDown),
            GamepadButton::DPadLeft => Some(Self::DPadLeft),
            GamepadButton::DPadRight => Some(Self::DPadRight),
            GamepadButton::Start => Some(Self::Start),
            GamepadButton::Select => Some(Self::Select),
            GamepadButton::LeftThumb => Some(Self::LeftThumb),
            GamepadButton::RightThumb => Some(Self::RightThumb),
            _ => None,
        }
    }

    /// Convert to Bevy's GamepadButton
    pub fn to_bevy_button(self) -> bevy::input::gamepad::GamepadButton {
        use bevy::input::gamepad::GamepadButton;

        match self {
            Self::South => GamepadButton::South,
            Self::East => GamepadButton::East,
            Self::West => GamepadButton::West,
            Self::North => GamepadButton::North,
            Self::LeftTrigger => GamepadButton::LeftTrigger,
            Self::RightTrigger2 => GamepadButton::RightTrigger2,
            Self::LeftTrigger2 => GamepadButton::LeftTrigger2,
            Self::RightTrigger => GamepadButton::RightTrigger,
            Self::DPadUp => GamepadButton::DPadUp,
            Self::DPadDown => GamepadButton::DPadDown,
            Self::DPadLeft => GamepadButton::DPadLeft,
            Self::DPadRight => GamepadButton::DPadRight,
            Self::Start => GamepadButton::Start,
            Self::Select => GamepadButton::Select,
            Self::LeftThumb => GamepadButton::LeftThumb,
            Self::RightThumb => GamepadButton::RightThumb,
        }
    }
}

impl GamepadAxisType {
    /// Convert from Bevy's GamepadAxis to our axis type
    pub fn from_bevy_axis(axis: bevy::input::gamepad::GamepadAxis) -> Option<Self> {
        use bevy::input::gamepad::GamepadAxis;

        match axis {
            GamepadAxis::LeftStickX => Some(Self::LeftStickX),
            GamepadAxis::LeftStickY => Some(Self::LeftStickY),
            GamepadAxis::RightStickX => Some(Self::RightStickX),
            GamepadAxis::RightStickY => Some(Self::RightStickY),
            _ => None,
        }
    }

    /// Convert to Bevy's GamepadAxis
    pub fn to_bevy_axis(self) -> bevy::input::gamepad::GamepadAxis {
        use bevy::input::gamepad::GamepadAxis;

        match self {
            Self::LeftStickX => GamepadAxis::LeftStickX,
            Self::LeftStickY => GamepadAxis::LeftStickY,
            Self::RightStickX => GamepadAxis::RightStickX,
            Self::RightStickY => GamepadAxis::RightStickY,
        }
    }
}
