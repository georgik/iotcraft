use crate::ui::GameState;
use bevy::prelude::*;
use bevy_console::ConsoleOpen;

/// System to handle ESC key presses for game state transitions
pub fn handle_esc_key(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<NextState<GameState>>,
    current_state: Res<State<GameState>>,
    mut windows: Query<&mut Window>,
    mut console_open: ResMut<ConsoleOpen>,
) {
    if !keyboard_input.just_pressed(KeyCode::Escape) {
        return;
    }

    match current_state.get() {
        GameState::InGame => {
            // Console escape handler should not interfere with gameplay menu
            // The MainMenuPlugin now handles ESC in InGame state
            return;
        }
        GameState::ConsoleOpen => {
            // Close console and return to game
            console_open.open = false;
            game_state.set(GameState::InGame);

            // Re-grab cursor
            for mut window in &mut windows {
                window.cursor_options.grab_mode = bevy::window::CursorGrabMode::Locked;
                window.cursor_options.visible = false;
            }
        }
        _ => (),
    }
}
