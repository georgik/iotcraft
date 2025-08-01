use crate::fonts::Fonts;
use bevy::prelude::*;

pub fn setup_fonts(mut commands: Commands, asset_server: Res<AssetServer>) {
    info!("Setting up font resources");

    let fonts = Fonts::new(&asset_server);
    commands.insert_resource(fonts);

    info!("Font resources loaded successfully");
}
