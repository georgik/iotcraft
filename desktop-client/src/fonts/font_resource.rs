use bevy::prelude::*;

#[derive(Resource)]
pub struct Fonts {
    pub regular: Handle<Font>,
    pub bold: Handle<Font>,
}

impl Fonts {
    pub fn new(assets: &AssetServer) -> Self {
        Self {
            regular: assets.load("fonts/NotoSans-Regular.ttf"),
            bold: assets.load("fonts/NotoSans-Bold.ttf"),
        }
    }
}

impl FromWorld for Fonts {
    fn from_world(world: &mut World) -> Self {
        let assets = world.get_resource::<AssetServer>().unwrap();
        Fonts::new(assets)
    }
}
