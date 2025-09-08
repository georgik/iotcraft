// Parameter bundles for inventory systems
// This reduces system parameter counts for Bevy compliance

use bevy::ecs::system::SystemParam;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;

#[cfg(feature = "console")]
use crate::console::ConsoleManager;

use crate::environment::{VoxelBlock, VoxelWorld};
use crate::inventory::{BreakBlockEvent, GiveItemEvent, PlaceBlockEvent, PlayerInventory};
use crate::profile::PlayerProfile;
// Desktop-specific imports
#[cfg(not(target_arch = "wasm32"))]
use crate::ui::main_menu::GameState;

// WASM-specific imports
#[cfg(target_arch = "wasm32")]
use crate::ui::GameState;

/// Parameter bundle for core inventory operations
#[derive(SystemParam)]
pub struct CoreInventoryParams<'w, 's> {
    pub inventory: ResMut<'w, PlayerInventory>,
    pub voxel_world: ResMut<'w, VoxelWorld>,
    pub commands: Commands<'w, 's>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

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

/// Parameter bundle for block placement operations
#[derive(SystemParam)]
pub struct BlockPlacementParams<'w, 's> {
    pub inventory: ResMut<'w, PlayerInventory>,
    pub voxel_world: ResMut<'w, VoxelWorld>,
    pub place_events: EventReader<'w, 's, PlaceBlockEvent>,
    pub commands: Commands<'w, 's>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Parameter bundle for block visual creation (meshes and materials)
#[derive(SystemParam)]
pub struct BlockVisualsParams<'w, 's> {
    pub meshes: ResMut<'w, Assets<Mesh>>,
    pub materials: ResMut<'w, Assets<StandardMaterial>>,
    pub asset_server: Res<'w, AssetServer>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Parameter bundle for item giving events
#[derive(SystemParam)]
pub struct ItemGivingParams<'w, 's> {
    pub inventory: ResMut<'w, PlayerInventory>,
    pub give_events: EventReader<'w, 's, GiveItemEvent>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Parameter bundle for block breaking operations
#[derive(SystemParam)]
pub struct BlockBreakingParams<'w, 's> {
    pub break_events: EventReader<'w, 's, BreakBlockEvent>,
    pub voxel_world: ResMut<'w, VoxelWorld>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Parameter bundle for multiplayer block synchronization (desktop only)
#[cfg(not(target_arch = "wasm32"))]
#[derive(SystemParam)]
pub struct MultiplayerBlockSyncParams<'w, 's> {
    pub place_events: EventReader<'w, 's, PlaceBlockEvent>,
    pub block_change_events: EventWriter<'w, crate::multiplayer::BlockChangeEvent>,
    pub multiplayer_mode: Res<'w, crate::multiplayer::MultiplayerMode>,
    pub player_profile: Res<'w, PlayerProfile>,
    pub inventory: Res<'w, PlayerInventory>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Parameter bundle for multiplayer block synchronization (WASM only)
#[cfg(target_arch = "wasm32")]
#[derive(SystemParam)]
pub struct MultiplayerBlockSyncParams<'w, 's> {
    pub place_events: EventReader<'w, 's, PlaceBlockEvent>,
    pub block_change_events: EventWriter<'w, crate::multiplayer_web::BlockChangeEvent>,
    pub multiplayer_mode: Res<'w, crate::multiplayer_web::MultiplayerMode>,
    pub player_profile: Res<'w, PlayerProfile>,
    pub inventory: Res<'w, PlayerInventory>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Comprehensive parameter bundle for complete block placement workflow
#[derive(SystemParam)]
pub struct ComprehensiveBlockPlacementParams<'w, 's> {
    pub placement: BlockPlacementParams<'w, 's>,
    pub visuals: BlockVisualsParams<'w, 's>,
    pub sync: MultiplayerBlockSyncParams<'w, 's>,
}

/// Parameter bundle for entity management in inventory systems
#[derive(SystemParam)]
pub struct InventoryEntityParams<'w, 's> {
    pub voxel_block_query: Query<'w, 's, Entity, With<VoxelBlock>>,
    pub commands: Commands<'w, 's>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{MinimalPlugins, app::App};

    #[test]
    fn test_core_inventory_params_compiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<PlayerInventory>();
        app.init_resource::<VoxelWorld>();

        fn test_system(_params: CoreInventoryParams) {}

        app.add_systems(Update, test_system);
    }

    #[test]
    fn test_inventory_input_params_compiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<PlayerInventory>();
        app.init_resource::<AccumulatedMouseScroll>();

        fn test_system(_params: InventoryInputParams) {}

        app.add_systems(Update, test_system);
    }

    #[test]
    fn test_block_placement_params_compiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<PlayerInventory>();
        app.init_resource::<VoxelWorld>();
        app.init_resource::<Events<PlaceBlockEvent>>();

        fn test_system(_params: BlockPlacementParams) {}

        app.add_systems(Update, test_system);
    }

    #[test]
    fn test_item_giving_params_compiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<PlayerInventory>();
        app.init_resource::<Events<GiveItemEvent>>();

        fn test_system(_params: ItemGivingParams) {}

        app.add_systems(Update, test_system);
    }

    #[test]
    fn test_block_breaking_params_compiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<VoxelWorld>();
        app.init_resource::<Events<BreakBlockEvent>>();

        fn test_system(_params: BlockBreakingParams) {}

        app.add_systems(Update, test_system);
    }
}
