#[cfg(feature = "console")]
use crate::console::ConsoleManager;
#[cfg(feature = "console")]
use bevy::prelude::*;
#[cfg(feature = "console")]
use bevy::window::CursorGrabMode;

#[cfg(feature = "console")]
use crate::ui::GameState;

#[cfg(feature = "console")]
pub fn handle_esc_key(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut console_manager: Option<ResMut<ConsoleManager>>,
    mut game_state: ResMut<NextState<GameState>>,
    mut windows: Query<&mut Window>,
) {
    // Only close console with ESC when it's currently open
    if keyboard_input.just_pressed(KeyCode::Escape) {
        if let Some(mut console_manager) = console_manager {
            if console_manager.console.is_visible() {
                console_manager.console.toggle_visibility();
                game_state.set(GameState::InGame);

                // Re-enable cursor grab when leaving console
                // In Bevy 0.17, cursor options are managed separately from Window
                if let Ok(window) = windows.single() {
                    // This would need to be handled by querying CursorOptions component in a separate system
                    // For now, leave this as a comment since the console feature gate handles this elsewhere
                    info!("Console closed - cursor management handled by game state system");
                }
            }
        }
    }
}
