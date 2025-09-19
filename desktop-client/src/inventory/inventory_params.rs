// Parameter bundles for inventory systems
// This reduces system parameter counts for Bevy compliance

use bevy::ecs::system::SystemParam;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;

#[cfg(feature = "console")]
use crate::console::ConsoleManager;

use crate::inventory::PlayerInventory;
// Desktop-specific imports
#[cfg(not(target_arch = "wasm32"))]
use crate::ui::main_menu::GameState;

// WASM-specific imports
#[cfg(target_arch = "wasm32")]
use crate::ui::GameState;

/// Parameter bundle for inventory input handling
#[derive(SystemParam)]
pub struct InventoryInputParams<'w, 's> {
    pub keyboard_input: Res<'w, ButtonInput<KeyCode>>,
    pub inventory: ResMut<'w, PlayerInventory>,
    pub accumulated_mouse_scroll: Res<'w, AccumulatedMouseScroll>,
    pub game_state: Res<'w, State<GameState>>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Parameter bundle for console-aware inventory input (when console feature enabled)
#[cfg(feature = "console")]
#[derive(SystemParam)]
pub struct ConsoleAwareInventoryInputParams<'w, 's> {
    pub base_params: InventoryInputParams<'w, 's>,
    pub console_manager: Option<Res<'w, ConsoleManager>>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{MinimalPlugins, app::App};

    #[test]
    fn test_inventory_input_params_compiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<PlayerInventory>();
        app.init_resource::<AccumulatedMouseScroll>();

        fn test_system(_params: InventoryInputParams) {}

        app.add_systems(Update, test_system);
    }
}
