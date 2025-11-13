//! Gamepad input management and handling systems
//!
//! Provides high-level gamepad input processing with configuration support,
//! connection management, and integration with the game's input systems.

use bevy::prelude::*;
use std::time::Duration;

use super::GameAction;
use super::gamepad_config::GameAxisAction;

/// Tracks connected gamepads and their states
#[derive(Resource, Debug)]
pub struct GamepadManager {
    /// Currently active gamepad entity
    pub active_gamepad: Option<Entity>,
    /// All connected gamepads
    pub connected_gamepads: Vec<Entity>,
    /// Names of connected gamepads (for UI display)
    pub gamepad_names: Vec<String>,
    /// Time since last gamepad activity (for auto-pause)
    pub last_activity_time: Option<Duration>,
}

/// Current state of gamepad buttons that are currently pressed
#[derive(Resource, Debug, Default)]
pub struct GamepadButtonState {
    /// Currently pressed buttons and their held duration
    pub pressed_buttons: std::collections::HashMap<bevy::input::gamepad::GamepadButton, Duration>,
    /// Currently held buttons for continuous input
    pub held_buttons: std::collections::HashSet<bevy::input::gamepad::GamepadButton>,
}

/// Current state of gamepad axes
#[derive(Resource, Debug, Default)]
pub struct GamepadAxisState {
    /// Current values of gamepad axes
    pub axis_values: std::collections::HashMap<bevy::input::gamepad::GamepadAxis, f32>,
}

/// Gamepad input events that other systems can process
#[derive(Event, Debug, Clone, PartialEq, BufferedEvent)]
pub enum GamepadInputEvent {
    /// A gamepad button was just pressed
    ButtonPressed {
        gamepad: Entity,
        button: bevy::input::gamepad::GamepadButton,
        action: GameAction,
    },
    /// A gamepad button was just released
    ButtonReleased {
        gamepad: Entity,
        button: bevy::input::gamepad::GamepadButton,
        action: GameAction,
    },
    /// A gamepad axis moved
    AxisMoved {
        gamepad: Entity,
        axis: bevy::input::gamepad::GamepadAxis,
        value: f32,
        action: GameAxisAction,
    },
}

impl Default for GamepadManager {
    fn default() -> Self {
        Self {
            active_gamepad: None,
            connected_gamepads: Vec::new(),
            gamepad_names: Vec::new(),
            last_activity_time: None,
        }
    }
}

impl GamepadManager {
    /// Get the currently active gamepad, or the first available one
    pub fn get_active_gamepad(&self) -> Option<Entity> {
        self.active_gamepad
            .or_else(|| self.connected_gamepads.first().copied())
    }

    /// Set the active gamepad
    pub fn set_active_gamepad(&mut self, gamepad: Entity) {
        if self.connected_gamepads.contains(&gamepad) {
            self.active_gamepad = Some(gamepad);
        }
    }

    /// Add a newly connected gamepad
    pub fn add_gamepad(&mut self, gamepad: Entity, name: String) {
        if !self.connected_gamepads.contains(&gamepad) {
            self.connected_gamepads.push(gamepad);
            self.gamepad_names.push(name);

            // If this is the first gamepad, make it active
            if self.active_gamepad.is_none() {
                self.active_gamepad = Some(gamepad);
            }
        }
    }

    /// Remove a disconnected gamepad
    pub fn remove_gamepad(&mut self, gamepad: Entity) {
        if let Some(index) = self.connected_gamepads.iter().position(|&g| g == gamepad) {
            self.connected_gamepads.remove(index);
            self.gamepad_names.remove(index);

            // If this was the active gamepad, select a new one
            if self.active_gamepad == Some(gamepad) {
                self.active_gamepad = self.connected_gamepads.first().copied();
            }
        }
    }

    /// Update activity timestamp
    pub fn update_activity(&mut self, time: Duration) {
        self.last_activity_time = Some(time);
    }
}

/// Apply deadzone to raw axis values
pub fn apply_deadzone(value: f32, deadzone: f32) -> f32 {
    if value.abs() < deadzone {
        0.0
    } else {
        // Normalize the value after removing deadzone
        let sign = value.signum();
        let normalized = (value.abs() - deadzone) / (1.0 - deadzone);
        sign * normalized.clamp(0.0, 1.0)
    }
}

/// Map raw axis value to action direction
pub fn map_axis_to_direction(value: f32, deadzone: f32) -> f32 {
    apply_deadzone(value, deadzone)
}

/// Check if a gamepad button is currently pressed using Bevy's Gamepad component
pub fn is_button_pressed(
    gamepad: &bevy::input::gamepad::Gamepad,
    button: bevy::input::gamepad::GamepadButton,
) -> bool {
    gamepad.pressed(button)
}

/// Check if a gamepad button is currently held using our button state tracking
pub fn is_button_held(
    button_state: &GamepadButtonState,
    button: bevy::input::gamepad::GamepadButton,
) -> bool {
    button_state.held_buttons.contains(&button)
}

/// Check if a gamepad action is currently active (button is held)
pub fn is_gamepad_action_active(
    button_state: &GamepadButtonState,
    gamepad_config: &super::gamepad_config::GamepadConfig,
    action: GameAction,
) -> bool {
    // Find which gamepad button corresponds to this action
    for (button_type, mapped_action) in &gamepad_config.button_mappings {
        if *mapped_action == action {
            let bevy_button = button_type.to_bevy_button();
            if button_state.held_buttons.contains(&bevy_button) {
                return true;
            }
        }
    }
    false
}

/// Check if a gamepad button was just pressed this frame
pub fn is_button_just_pressed(
    gamepad: &bevy::input::gamepad::Gamepad,
    button: bevy::input::gamepad::GamepadButton,
) -> bool {
    gamepad.just_pressed(button)
}

/// Check if a gamepad button was just released this frame
pub fn is_button_just_released(
    gamepad: &bevy::input::gamepad::Gamepad,
    button: bevy::input::gamepad::GamepadButton,
) -> bool {
    gamepad.just_released(button)
}

/// Get the current value of a gamepad axis
pub fn get_axis_value(
    gamepad: &bevy::input::gamepad::Gamepad,
    axis: bevy::input::gamepad::GamepadAxis,
) -> Option<f32> {
    gamepad.get(axis)
}

/// Get the raw value of a gamepad axis (without deadzone processing)
pub fn get_axis_value_raw(
    gamepad: &bevy::input::gamepad::Gamepad,
    axis: bevy::input::gamepad::GamepadAxis,
) -> Option<f32> {
    gamepad.get(axis)
}

/// Trigger vibration on a gamepad
pub fn trigger_vibration(
    rumble_requests: &mut EventWriter<bevy::input::gamepad::GamepadRumbleRequest>,
    gamepad: Entity,
    intensity: f32,
    duration: Duration,
) {
    rumble_requests.write(bevy::input::gamepad::GamepadRumbleRequest::Add {
        gamepad,
        intensity: bevy::input::gamepad::GamepadRumbleIntensity::strong_motor(intensity),
        duration,
    });
}

/// Stop vibration on a gamepad
pub fn stop_vibration(
    rumble_requests: &mut EventWriter<bevy::input::gamepad::GamepadRumbleRequest>,
    gamepad: Entity,
) {
    rumble_requests.write(bevy::input::gamepad::GamepadRumbleRequest::Stop { gamepad });
}

/// Get a human-readable name for a gamepad button
pub fn get_button_name(button: bevy::input::gamepad::GamepadButton) -> &'static str {
    use bevy::input::gamepad::GamepadButton;

    match button {
        GamepadButton::South => "A",
        GamepadButton::East => "B",
        GamepadButton::West => "X",
        GamepadButton::North => "Y",
        GamepadButton::LeftTrigger => "LB",
        GamepadButton::RightTrigger2 => "RB",
        GamepadButton::LeftTrigger2 => "LT",
        GamepadButton::RightTrigger => "RT",
        GamepadButton::DPadUp => "D-Pad Up",
        GamepadButton::DPadDown => "D-Pad Down",
        GamepadButton::DPadLeft => "D-Pad Left",
        GamepadButton::DPadRight => "D-Pad Right",
        GamepadButton::Start => "Start",
        GamepadButton::Select => "Select",
        GamepadButton::LeftThumb => "Left Stick",
        GamepadButton::RightThumb => "Right Stick",
        _ => "Unknown",
    }
}

/// Get a human-readable name for a gamepad axis
pub fn get_axis_name(axis: bevy::input::gamepad::GamepadAxis) -> &'static str {
    use bevy::input::gamepad::GamepadAxis;

    match axis {
        GamepadAxis::LeftStickX => "Left Stick X",
        GamepadAxis::LeftStickY => "Left Stick Y",
        GamepadAxis::RightStickX => "Right Stick X",
        GamepadAxis::RightStickY => "Right Stick Y",
        _ => "Unknown",
    }
}
