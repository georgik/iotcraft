use bevy::prelude::*;

pub mod font_resource;
pub mod font_systems;

pub use font_resource::*;
pub use font_systems::*;

pub struct FontPlugin;

impl Plugin for FontPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, setup_fonts);
    }
}
