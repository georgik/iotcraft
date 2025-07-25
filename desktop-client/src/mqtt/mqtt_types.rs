use bevy::prelude::*;
use std::sync::Mutex;
use std::sync::mpsc::Receiver;

#[derive(Resource)]
pub struct TemperatureResource {
    pub value: Option<f32>,
}

impl Default for TemperatureResource {
    fn default() -> Self {
        Self { value: None }
    }
}

#[derive(Resource)]
pub struct TemperatureReceiver(pub Mutex<Receiver<f32>>);
