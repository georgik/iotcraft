use bevy::prelude::*;
use bevy::pbr::MeshMaterial3d;
use std::collections::HashSet;
use serde_json;
use log::info;

use super::device_types::*;
use crate::console::BlinkCube;

pub struct DevicePlugin;

impl Plugin for DevicePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, listen_for_device_announcements);
    }
}

pub fn listen_for_device_announcements(
    mut commands: Commands,
    device_receiver: Res<DeviceAnnouncementReceiver>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut tracker: ResMut<DevicesTracker>,
    asset_server: Res<AssetServer>,
) {
    if let Ok(rx) = device_receiver.0.lock() {
        if let Ok(device_json) = rx.try_recv() {
            // Parse the JSON device announcement
            if let Ok(device_data) = serde_json::from_str::<serde_json::Value>(&device_json) {
                if let (Some(device_id), Some(device_type), Some(location)) = (
                    device_data["device_id"].as_str(),
                    device_data["device_type"].as_str(),
                    device_data["location"].as_object()
                ) {
                    if !tracker.spawned_devices.contains(device_id) {
                        tracker.spawned_devices.insert(device_id.to_string());
                        
                        // Extract location coordinates
                        let x = location["x"].as_f64().unwrap_or(0.0) as f32;
                        let y = location["y"].as_f64().unwrap_or(0.5) as f32;
                        let z = location["z"].as_f64().unwrap_or(0.0) as f32;
                        
                        // Choose material based on device type
                        let material = match device_type {
                            "lamp" => {
                                let lamp_texture: Handle<Image> = asset_server.load("textures/lamp.png");
                                materials.add(StandardMaterial {
                                    base_color_texture: Some(lamp_texture),
                                    base_color: Color::srgb(0.2, 0.2, 0.2),
                                    ..default()
                                })
                            },
                            "sensor" => materials.add(StandardMaterial {
                                base_color: Color::srgb(0.2, 0.8, 1.0),
                                ..default()
                            }),
                            _ => materials.add(StandardMaterial {
                                base_color: Color::srgb(0.5, 0.5, 0.5),
                                ..default()
                            }),
                        };
                        
                        // Spawn the device entity
                        let mut entity_commands = commands.spawn((
                            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                            MeshMaterial3d(material),
                            Transform::from_translation(Vec3::new(x, y, z)),
                            DeviceEntity {
                                device_id: device_id.to_string(),
                                device_type: device_type.to_string(),
                            },
                        ));
                        
                        // Add BlinkCube component for lamp devices so they can blink
                        if device_type == "lamp" {
                            entity_commands.insert(BlinkCube);
                        }
                        
                        info!("Spawned device: {} of type {} at ({}, {}, {})", device_id, device_type, x, y, z);
                    }
                }
            }
        }
    }
}
