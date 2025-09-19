pub mod async_world_creation;
pub mod world_systems;
pub mod world_types;

pub use async_world_creation::*;
pub use world_systems::WorldPlugin as WorldSystemsPlugin;
pub use world_types::*;

use bevy::prelude::*;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WorldSystemsPlugin)
            .add_plugins(async_world_creation::AsyncWorldCreationPlugin);
    }
}
