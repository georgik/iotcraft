use bevy::prelude::*;

#[derive(Resource)]
pub struct Fonts {
    pub regular: Handle<Font>,
    #[allow(dead_code)]
    pub bold: Handle<Font>,
}

impl Fonts {
    pub fn new(assets: &AssetServer) -> Self {
        // Use Noto Sans which has better Unicode support than Bevy's default font
        // This includes support for Czech diacritical marks and extended Latin characters
        Self {
            regular: assets.load("fonts/NotoSans-Regular.ttf"),
            bold: assets.load("fonts/NotoSans-Regular.ttf"), // Use regular for bold too for now
        }
    }
}

impl FromWorld for Fonts {
    fn from_world(world: &mut World) -> Self {
        let assets = world.get_resource::<AssetServer>().unwrap();
        Fonts::new(assets)
    }
}
