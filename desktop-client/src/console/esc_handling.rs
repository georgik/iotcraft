#[cfg(feature = "console")]
use crate::console::ConsoleManager;
#[cfg(feature = "console")]
use bevy::prelude::*;

#[cfg(all(feature = "console", not(target_arch = "wasm32")))]
use crate::ui::GameState;

#[cfg(all(feature = "console", not(target_arch = "wasm32")))]
pub fn handle_esc_key(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    console_manager: Option<ResMut<ConsoleManager>>,
    mut game_state: ResMut<NextState<GameState>>,
    windows: Query<&mut Window>,
) {
    // Only close console with ESC when it's currently open
    if keyboard_input.just_pressed(KeyCode::Escape) {
        if let Some(mut console_manager) = console_manager {
            if console_manager.console.is_visible() {
                console_manager.console.toggle_visibility();
                game_state.set(GameState::InGame);

                // Re-enable cursor grab when leaving console
                // In Bevy 0.17, cursor options are managed separately from Window
                if let Ok(_window) = windows.single() {
                    // This would need to be handled by querying CursorOptions component in a separate system
                    // For now, leave this as a comment since the console feature gate handles this elsewhere
                    info!("Console closed - cursor management handled by game state system");
                }
            }
        }
    }
}

#[cfg(all(feature = "console", target_arch = "wasm32"))]
pub fn handle_esc_key(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    console_manager: Option<ResMut<ConsoleManager>>,
    windows: Query<&mut Window>,
) {
    // Only close console with ESC when it's currently open
    if keyboard_input.just_pressed(KeyCode::Escape) {
        if let Some(mut console_manager) = console_manager {
            if console_manager.console.is_visible() {
                console_manager.console.toggle_visibility();

                // Re-enable cursor grab when leaving console
                if let Ok(_window) = windows.single() {
                    info!("Console closed - cursor management handled by game state system");
                }
            }
        }
    }
}
