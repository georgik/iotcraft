use bevy::prelude::*;

pub mod inventory_systems;
pub mod inventory_types;

pub use inventory_systems::*;
pub use inventory_types::*;

/// Plugin for the inventory system
pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<GiveItemEvent>()
            .add_event::<PlaceBlockEvent>()
            .add_event::<BreakBlockEvent>()
            .insert_resource(PlayerInventory::new())
            .add_systems(
                Update,
                (give_item_system, place_block_system, break_block_system),
            );
    }
}
