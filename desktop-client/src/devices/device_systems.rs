use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;
use log::info;
use serde_json;

use super::device_types::*;
use crate::console::BlinkCube;
use crate::interaction::{Interactable, InteractionType};

pub struct DevicePlugin;

impl Plugin for DevicePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DevicesTracker {
            spawned_devices: std::collections::HashSet::new(),
        })
        .add_systems(Update, listen_for_device_announcements);
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
                    device_data["location"].as_object(),
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
                                let lamp_texture: Handle<Image> =
                                    asset_server.load("textures/lamp.webp");
                                materials.add(StandardMaterial {
                                    base_color_texture: Some(lamp_texture),
                                    base_color: Color::srgb(0.2, 0.2, 0.2),
                                    ..default()
                                })
                            }
                            "door" => {
                                let door_texture: Handle<Image> =
                                    asset_server.load("textures/door.webp");
                                materials.add(StandardMaterial {
                                    base_color_texture: Some(door_texture),
                                    base_color: Color::srgb(0.8, 0.6, 0.4), // Wood-like brown when closed
                                    ..default()
                                })
                            }
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
                            if device_type == "door" {
                                Mesh3d(meshes.add(Cuboid::new(0.2, 2.0, 1.0)))
                            } else {
                                Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0)))
                            },
                            MeshMaterial3d(material),
                            Transform::from_translation(Vec3::new(x, y, z)),
                            DeviceEntity {
                                device_id: device_id.to_string(),
                                device_type: device_type.to_string(),
                            },
                        ));

                        if device_type == "lamp" {
                            entity_commands.insert(BlinkCube);
                            entity_commands.insert(Interactable {
                                interaction_type: InteractionType::ToggleLamp,
                            });
                            entity_commands.insert(crate::interaction::LampState {
                                is_on: false,
                                device_id: device_id.to_string(),
                            });
                        }

                        if device_type == "door" {
                            entity_commands.insert(Interactable {
                                interaction_type: InteractionType::ToggleDoor,
                            });
                            entity_commands.insert(crate::devices::device_types::DoorState {
                                is_open: false,
                                device_id: device_id.to_string(),
                            });
                            entity_commands.insert(
                                crate::devices::device_types::OriginalPosition {
                                    position: Vec3::new(x, y, z),
                                },
                            );
                        }

                        info!(
                            "Spawned device: {} of type {} at ({}, {}, {})",
                            device_id, device_type, x, y, z
                        );
                    }
                }
            }
        }
    }
}
