use bevy::prelude::*;

use crate::console::bevy_ui_console::BevyUiConsole;
use crate::console::console_trait::{Console, ConsoleConfig, ConsoleManager};

#[cfg(not(target_arch = "wasm32"))]
use crate::ui::GameState;

// Import different console implementations based on features

#[cfg(feature = "console-bevy")]
use crate::console::bevy_console_adapter::BevyConsoleAdapter;

/// Plugin that provides console functionality with swappable backends
pub struct ConsolePlugin;

impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut App) {
        // Initialize the console config and state
        app.init_resource::<ConsoleConfig>();

        // Choose console implementation based on available features
        let console_impl: Box<dyn Console> = {
            #[cfg(feature = "console-bevy")]
            {
                info!("Using legacy Bevy console implementation");
                Box::new(BevyConsoleAdapter::new())
            }

            #[cfg(not(feature = "console-bevy"))]
            {
                info!("Using Bevy UI console implementation");
                Box::new(BevyUiConsole::new())
            }
        };

        // Create and insert the console manager
        let mut console_manager = ConsoleManager::new(console_impl);
        console_manager.console.initialize(app);
        app.insert_resource(console_manager);

        // Add console message events
        app.add_event::<ConsoleMessageEvent>();

        // Add console systems - fix system scheduling
        app.add_systems(Update, handle_console_toggle)
            .add_systems(Update, update_console)
            .add_systems(Update, handle_console_messages);

        info!("Console plugin initialized");
    }
}

/// System to handle console toggle key
#[cfg(not(target_arch = "wasm32"))]
fn handle_console_toggle(
    mut console_manager: ResMut<ConsoleManager>,
    config: Res<ConsoleConfig>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<NextState<GameState>>,
    current_state: Res<State<GameState>>,
) {
    // Handle F12 key (default toggle) - still works as full toggle
    if keyboard_input.just_pressed(config.toggle_key) {
        console_manager.toggle();

        // Update game state based on console visibility
        if console_manager.console.is_visible() {
            game_state.set(GameState::ConsoleOpen);
        } else {
            game_state.set(GameState::InGame);
        }
    }

    // Handle T key for opening console only (not closing)
    if keyboard_input.just_pressed(KeyCode::KeyT) {
        if !console_manager.console.is_visible() && *current_state.get() == GameState::InGame {
            // Console is closed and we're in game, open it
            console_manager.console.toggle_visibility();
            game_state.set(GameState::ConsoleOpen);

            // Set flag to ignore next T key input to prevent immediate character input
            if let Some(bevy_ui_console) = console_manager
                .console
                .as_any_mut()
                .downcast_mut::<crate::console::bevy_ui_console::BevyUiConsole>(
            ) {
                bevy_ui_console.ignore_next_t_key = true;
            }
        }
    }

    // Handle ESC key for closing console when it's open
    if keyboard_input.just_pressed(KeyCode::Escape) {
        if console_manager.console.is_visible() && *current_state.get() == GameState::ConsoleOpen {
            // Console is open, close it
            console_manager.console.toggle_visibility();
            game_state.set(GameState::InGame);
        }
    }
}

/// Simplified console toggle for WASM (no game state management)
#[cfg(target_arch = "wasm32")]
fn handle_console_toggle(
    mut console_manager: ResMut<ConsoleManager>,
    config: Res<ConsoleConfig>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // Handle F12 key (default toggle)
    if keyboard_input.just_pressed(config.toggle_key) {
        console_manager.toggle();
    }

    // Handle T key for opening console only (not closing)
    if keyboard_input.just_pressed(KeyCode::KeyT) {
        if !console_manager.console.is_visible() {
            console_manager.console.toggle_visibility();

            // Set flag to ignore next T key input to prevent immediate character input
            if let Some(bevy_ui_console) = console_manager
                .console
                .as_any_mut()
                .downcast_mut::<crate::console::bevy_ui_console::BevyUiConsole>(
            ) {
                bevy_ui_console.ignore_next_t_key = true;
            }
        }
    }

    // Handle ESC key for closing console when it's open
    if keyboard_input.just_pressed(KeyCode::Escape) {
        if console_manager.console.is_visible() {
            console_manager.console.toggle_visibility();
        }
    }
}

/// System to update the console each frame
fn update_console(world: &mut World) {
    // We need direct world access to call console.update(&mut world)
    // Get the console manager, call update, then put it back
    if let Some(mut console_manager) = world.remove_resource::<ConsoleManager>() {
        console_manager.console.update(world);
        world.insert_resource(console_manager);
    }
}

/// Event for sending messages to the console from other systems
#[derive(Event, BufferedEvent, Clone)]
pub struct ConsoleMessageEvent {
    pub message: String,
}

/// System to handle console message events
pub fn handle_console_messages(
    mut console_manager: ResMut<ConsoleManager>,
    mut message_events: EventReader<ConsoleMessageEvent>,
) {
    for event in message_events.read() {
        console_manager.add_message(&event.message);
    }
}
