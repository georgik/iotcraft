use bevy::prelude::*;

#[derive(Component)]
pub struct Ground;

#[derive(Component)]
pub struct LogoCube;

#[derive(Component)]
pub struct Thermometer;

#[derive(Resource)]
pub struct ThermometerMaterial(pub Handle<StandardMaterial>);
