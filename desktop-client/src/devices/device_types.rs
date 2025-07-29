use bevy::prelude::*;
use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::mpsc::Receiver;

#[derive(Resource)]
pub struct DevicesTracker {
    pub spawned_devices: HashSet<String>,
}

#[derive(Resource)]
pub struct DeviceAnnouncementReceiver(pub Mutex<Receiver<String>>);

#[derive(Component)]
pub struct DeviceEntity {
    #[allow(dead_code)]
    pub device_id: String,
    #[allow(dead_code)]
    pub device_type: String,
}

/// Component to track door state
#[derive(Component, Debug, Clone)]
pub struct DoorState {
    pub is_open: bool,
    pub device_id: String,
}
