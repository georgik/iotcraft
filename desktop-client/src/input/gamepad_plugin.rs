//! Gamepad plugin providing comprehensive gamepad support for IoTCraft
//!
//! This plugin handles gamepad connections, processes input events, and integrates
//! with the existing input systems.

use bevy::prelude::*;
use std::time::Duration;

use super::GameAction;
use super::gamepad::{
    GamepadAxisState, GamepadButtonState, GamepadInputEvent, GamepadManager, apply_deadzone,
    get_axis_value_raw, is_button_just_pressed, is_button_just_released,
};
use super::gamepad_config::GameAxisAction;

/// Plugin that adds comprehensive gamepad support to the game
pub struct GamepadInputPlugin;

impl Plugin for GamepadInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GamepadManager>()
            .init_resource::<GamepadButtonState>()
            .init_resource::<GamepadAxisState>()
            .add_event::<GamepadInputEvent>()
            .add_systems(Startup, initialize_gamepad_manager)
            .add_systems(
                Update,
                (
                    handle_gamepad_connections,
                    update_gamepad_button_states,
                    process_gamepad_input,
                    update_button_duration,
                    update_axis_state,
                    send_gamepad_events,
                )
                    .chain(),
            );
    }
}

/// Initialize the gamepad manager resource
fn initialize_gamepad_manager(
    mut gamepad_manager: ResMut<GamepadManager>,
    gamepads: Query<(Entity, &Name), With<bevy::input::gamepad::Gamepad>>,
) {
    info!("Initializing gamepad manager...");

    for (entity, name) in &gamepads {
        gamepad_manager.add_gamepad(entity, name.to_string());
        info!("Found connected gamepad: {}", name);
    }

    if gamepad_manager.connected_gamepads.is_empty() {
        info!(
            "No gamepads connected. Gamepad support will be available when controllers are connected."
        );
    }
}

/// Handle gamepad connection and disconnection events
fn handle_gamepad_connections(
    mut gamepad_manager: ResMut<GamepadManager>,
    mut connection_events: EventReader<bevy::input::gamepad::GamepadConnectionEvent>,
    gamepads: Query<&Name, With<bevy::input::gamepad::Gamepad>>,
) {
    for event in connection_events.read() {
        match &event.connection {
            bevy::input::gamepad::GamepadConnection::Connected {
                name,
                vendor_id: _,
                product_id: _,
            } => {
                if let Ok(gamepad_name) = gamepads.get(event.gamepad) {
                    gamepad_manager.add_gamepad(event.gamepad, gamepad_name.to_string());
                    info!("Gamepad connected: {}", gamepad_name);
                } else {
                    gamepad_manager.add_gamepad(event.gamepad, name.clone());
                    info!("Gamepad connected: {}", name);
                }
            }
            bevy::input::gamepad::GamepadConnection::Disconnected => {
                gamepad_manager.remove_gamepad(event.gamepad);
                info!("Gamepad disconnected");
            }
        }
    }
}

/// Update gamepad button states - track which buttons are currently held down
fn update_gamepad_button_states(
    mut button_state: ResMut<GamepadButtonState>,
    gamepads: Query<&bevy::input::gamepad::Gamepad>,
) {
    // Clear the held buttons set and rebuild it from current gamepad states
    button_state.held_buttons.clear();

    // Add any currently pressed buttons to the held set
    for gamepad in &gamepads {
        use bevy::input::gamepad::GamepadButton;

        // Check all standard gamepad buttons
        let all_buttons = [
            GamepadButton::South,
            GamepadButton::East,
            GamepadButton::West,
            GamepadButton::North,
            GamepadButton::LeftTrigger,
            GamepadButton::LeftTrigger2,
            GamepadButton::RightTrigger,
            GamepadButton::RightTrigger2,
            GamepadButton::LeftThumb,
            GamepadButton::RightThumb,
            GamepadButton::Select,
            GamepadButton::Start,
            GamepadButton::DPadUp,
            GamepadButton::DPadDown,
            GamepadButton::DPadLeft,
            GamepadButton::DPadRight,
            GamepadButton::Mode,
            GamepadButton::C,
            GamepadButton::Z,
        ];

        for button in all_buttons {
            if gamepad.pressed(button) {
                button_state.held_buttons.insert(button);
            }
        }
    }
}

/// Process gamepad input and map to game actions
fn process_gamepad_input(
    gamepad_manager: Res<GamepadManager>,
    gamepad_config: Res<super::gamepad_config::GamepadConfig>,
    gamepads: Query<&bevy::input::gamepad::Gamepad>,
    mut button_state: ResMut<GamepadButtonState>,
    mut axis_state: ResMut<GamepadAxisState>,
    mut input_events: EventWriter<GamepadInputEvent>,
    _time: Res<Time>,
) {
    if !gamepad_config.enabled {
        return;
    }

    let Some(active_gamepad) = gamepad_manager.get_active_gamepad() else {
        return;
    };

    let Ok(gamepad) = gamepads.get(active_gamepad) else {
        return;
    };

    // Update activity time when any input is detected
    let mut has_activity = false;

    // Process button inputs
    for (button_type, &action) in &gamepad_config.button_mappings {
        let bevy_button = button_type.to_bevy_button();

        if is_button_just_pressed(gamepad, bevy_button) {
            button_state
                .pressed_buttons
                .insert(bevy_button, Duration::ZERO);
            input_events.write(GamepadInputEvent::ButtonPressed {
                gamepad: active_gamepad,
                button: bevy_button,
                action,
            });
            has_activity = true;
        } else if is_button_just_released(gamepad, bevy_button) {
            button_state.pressed_buttons.remove(&bevy_button);
            input_events.write(GamepadInputEvent::ButtonReleased {
                gamepad: active_gamepad,
                button: bevy_button,
                action,
            });
        }
    }

    // Process axis inputs
    for (axis_type, &axis_action) in &gamepad_config.axis_mappings {
        let bevy_axis = axis_type.to_bevy_axis();

        if let Some(raw_value) = get_axis_value_raw(gamepad, bevy_axis) {
            let processed_value = apply_deadzone(
                raw_value * gamepad_config.sensitivity,
                gamepad_config.deadzone,
            );

            // Only send events for significant axis movement
            let threshold = 0.1;
            if processed_value.abs() > threshold {
                let current_value = axis_state.axis_values.get(&bevy_axis).unwrap_or(&0.0);
                if (current_value - processed_value).abs() > 0.01 {
                    input_events.write(GamepadInputEvent::AxisMoved {
                        gamepad: active_gamepad,
                        axis: bevy_axis,
                        value: processed_value,
                        action: axis_action,
                    });
                }
            }

            axis_state.axis_values.insert(bevy_axis, processed_value);

            if processed_value.abs() > threshold {
                has_activity = true;
            }
        }
    }

    if has_activity {
        // Update activity time would be managed by the gamepad manager
        // This could be used for auto-pause functionality
    }
}

/// Update the duration held for pressed buttons
fn update_button_state(mut button_state: ResMut<GamepadButtonState>, time: Res<Time>) {
    let delta = time.delta();
    for duration in button_state.pressed_buttons.values_mut() {
        *duration += delta;
    }
}

/// Update axis state (clean up old values, etc.)
fn update_axis_state(mut axis_state: ResMut<GamepadAxisState>) {
    // Remove axis values that are very close to zero
    axis_state
        .axis_values
        .retain(|_, &mut value| value.abs() > 0.01);
}

/// Send gamepad events to the main game systems
fn send_gamepad_events(
    mut gamepad_events: EventReader<GamepadInputEvent>,
    // Add additional parameters here for handling the events
    // For example: mut player_movement: EventWriter<PlayerMovementEvent>,
) {
    for event in gamepad_events.read() {
        match event {
            GamepadInputEvent::ButtonPressed { action, .. } => {
                info!("Gamepad button pressed: {:?}", action);
                // Handle button press actions (individual press events)
            }
            GamepadInputEvent::ButtonReleased { action, .. } => {
                info!("Gamepad button released: {:?}", action);
                // Handle button release actions
            }
            GamepadInputEvent::AxisMoved { action, value, .. } => {
                // Handle axis movement (continuous state tracking)
            }
        }
    }
}

/// Update the duration held for pressed buttons
fn update_button_duration(time: Res<Time>, mut button_state: ResMut<GamepadButtonState>) {
    let delta = time.delta();
    for duration in button_state.pressed_buttons.values_mut() {
        *duration += delta;
    }
}
