// Console-aware inventory input handling
// This module provides inventory input handling with optional console integration

use crate::inventory::InventoryInputParams;
use bevy::prelude::*;

#[cfg(feature = "console")]
use crate::inventory::ConsoleAwareInventoryInputParams;

/// System to handle inventory input (console feature enabled version)
#[cfg(feature = "console")]
pub fn handle_inventory_input_bundled(mut params: ConsoleAwareInventoryInputParams) {
    // Don't handle input when console is open or in any menu state
    let console_open = params
        .console_manager
        .as_ref()
        .map(|manager| manager.console.is_visible())
        .unwrap_or(false);

    #[cfg(not(target_arch = "wasm32"))]
    let in_game_state =
        *params.base_params.game_state.get() == crate::ui::main_menu::GameState::InGame;
    #[cfg(target_arch = "wasm32")]
    let in_game_state = *params.base_params.game_state.get() == crate::ui::GameState::InGame;

    if console_open || !in_game_state {
        return;
    }

    handle_inventory_input_core(&mut params.base_params);
}

/// System to handle inventory input (console feature disabled version)
#[cfg(not(feature = "console"))]
pub fn handle_inventory_input_bundled(mut params: InventoryInputParams) {
    // Don't handle input when in any menu state (console not available to check)
    #[cfg(not(target_arch = "wasm32"))]
    let in_game_state = *params.game_state.get() == crate::ui::main_menu::GameState::InGame;
    #[cfg(target_arch = "wasm32")]
    let in_game_state = *params.game_state.get() == crate::ui::GameState::InGame;

    if !in_game_state {
        return;
    }

    handle_inventory_input_core(&mut params);
}

/// Core inventory input handling logic (shared between console and non-console versions)
fn handle_inventory_input_core(params: &mut InventoryInputParams) {
    // Handle mouse wheel for inventory slot switching
    if params.accumulated_mouse_scroll.delta.y != 0.0 {
        let current_slot = params.inventory.selected_slot;
        let new_slot = if params.accumulated_mouse_scroll.delta.y > 0.0 {
            // Scroll up - previous slot (wraps around)
            if current_slot == 0 {
                8
            } else {
                current_slot - 1
            }
        } else {
            // Scroll down - next slot (wraps around)
            if current_slot == 8 {
                0
            } else {
                current_slot + 1
            }
        };

        if new_slot != current_slot {
            params.inventory.select_slot(new_slot);
            info!("Selected inventory slot {}", new_slot + 1);
        }
    }

    // Handle number keys 1-9 for slot selection
    let key_mappings = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
    ];

    for (key, slot) in key_mappings {
        if params.keyboard_input.just_pressed(key) {
            params.inventory.select_slot(slot);
            info!("Selected inventory slot {}", slot + 1);
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::PlayerInventory;
    use bevy::{MinimalPlugins, app::App};

    #[test]
    fn test_handle_inventory_input_bundled_compiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<PlayerInventory>();

        #[cfg(feature = "console")]
        app.add_systems(Update, handle_inventory_input_bundled);
        #[cfg(not(feature = "console"))]
        app.add_systems(Update, handle_inventory_input_bundled);

        // Compilation test only
    }
}
