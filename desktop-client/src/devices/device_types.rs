use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::mpsc::Receiver;

/// Device types available in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceType {
    Lamp,
    Door,
    Sensor,
}

impl DeviceType {
    /// Convert from string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "lamp" => Some(DeviceType::Lamp),
            "door" => Some(DeviceType::Door),
            "sensor" => Some(DeviceType::Sensor),
            _ => None,
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceType::Lamp => "lamp",
            DeviceType::Door => "door",
            DeviceType::Sensor => "sensor",
        }
    }

    /// Get the mesh dimensions for this device type (width, height, depth)
    pub fn mesh_dimensions(&self) -> (f32, f32, f32) {
        match self {
            DeviceType::Lamp => (1.0, 1.0, 1.0),
            DeviceType::Door => (0.2, 2.0, 1.0),
            DeviceType::Sensor => (1.0, 1.0, 1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_type_from_str() {
        assert_eq!(DeviceType::from_str("lamp"), Some(DeviceType::Lamp));
        assert_eq!(DeviceType::from_str("door"), Some(DeviceType::Door));
        assert_eq!(DeviceType::from_str("sensor"), Some(DeviceType::Sensor));
        assert_eq!(DeviceType::from_str("unknown"), None);
    }

    #[test]
    fn test_device_type_as_str() {
        assert_eq!(DeviceType::Lamp.as_str(), "lamp");
        assert_eq!(DeviceType::Door.as_str(), "door");
        assert_eq!(DeviceType::Sensor.as_str(), "sensor");
    }

    #[test]
    fn test_device_type_mesh_dimensions() {
        assert_eq!(DeviceType::Lamp.mesh_dimensions(), (1.0, 1.0, 1.0));
        assert_eq!(DeviceType::Door.mesh_dimensions(), (0.2, 2.0, 1.0));
        assert_eq!(DeviceType::Sensor.mesh_dimensions(), (1.0, 1.0, 1.0));
    }
}

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
#[derive(Component, Clone, Debug)]
pub struct DoorState {
    pub is_open: bool,
}

#[derive(Component, Clone, Debug)]
pub struct OriginalPosition {
    pub position: Vec3,
}
