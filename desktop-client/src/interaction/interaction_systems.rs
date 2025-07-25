use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;
use bevy_console::ConsoleOpen;
use log::{error, info};
use rumqttc::{Client, Event, MqttOptions, Outgoing, QoS};
use std::time::Duration;

use super::interaction_types::*;
use crate::config::MqttConfig;
use crate::devices::DeviceEntity;
use crate::environment::Ground;

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<InteractionEvent>()
            .add_event::<LampToggleEvent>()
            .insert_resource(HoveredEntity::default())
            .add_systems(Startup, setup_lamp_materials)
            .add_systems(
                Update,
                (
                    raycast_interaction_system,
                    handle_interaction_input,
                    handle_interaction_events,
                    handle_lamp_toggle_events,
                    update_lamp_visuals,
                    draw_interaction_cursor,
                )
                    .chain(),
            );
    }
}

/// Setup material resources for different lamp states
fn setup_lamp_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let lamp_texture: Handle<Image> = asset_server.load("textures/lamp.png");

    let lamp_materials = LampMaterials {
        lamp_off: materials.add(StandardMaterial {
            base_color_texture: Some(lamp_texture.clone()),
            base_color: Color::srgb(0.2, 0.2, 0.2), // Dark when off
            ..default()
        }),
        lamp_on: materials.add(StandardMaterial {
            base_color_texture: Some(lamp_texture.clone()),
            base_color: Color::srgb(1.0, 0.9, 0.6), // Bright yellow when on
            emissive: LinearRgba::new(0.8, 0.7, 0.4, 1.0),
            ..default()
        }),
        hovered: materials.add(StandardMaterial {
            base_color_texture: Some(lamp_texture),
            base_color: Color::srgb(0.6, 0.8, 1.0), // Blue tint when hovered
            emissive: LinearRgba::new(0.1, 0.2, 0.3, 1.0),
            ..default()
        }),
    };

    commands.insert_resource(lamp_materials);
}

/// System that performs raycasting to find interactable objects under the cursor
fn raycast_interaction_system(
    camera_query: Single<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    interactable_query: Query<(Entity, &GlobalTransform), With<Interactable>>,
    mut hovered_entity: ResMut<HoveredEntity>,
    console_open: Res<ConsoleOpen>,
) {
    // Don't interact when console is open
    if console_open.open {
        hovered_entity.entity = None;
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };

    let (camera, camera_transform) = *camera_query;

    let Some(cursor_position) = window.cursor_position() else {
        hovered_entity.entity = None;
        return;
    };

    // Calculate a ray pointing from the camera into the world based on the cursor's position
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    let mut closest_entity = None;
    let mut closest_distance = f32::MAX;

    // Check all interactable entities
    for (entity, transform) in interactable_query.iter() {
        let entity_position = transform.translation();

        // Simple sphere collision detection (assuming 1x1x1 cubes)
        let sphere_radius = 0.7; // Slightly smaller than cube for better UX

        // Calculate closest point on ray to sphere center
        let to_sphere = entity_position - ray.origin;
        let projection_length = to_sphere.dot(*ray.direction);

        if projection_length < 0.0 {
            continue; // Behind the camera
        }

        let closest_point = ray.origin + ray.direction * projection_length;
        let distance_to_sphere = (closest_point - entity_position).length();

        if distance_to_sphere <= sphere_radius && projection_length < closest_distance {
            closest_distance = projection_length;
            closest_entity = Some(entity);
        }
    }

    hovered_entity.entity = closest_entity;
}

/// System that handles mouse click input for interactions
fn handle_interaction_input(
    mouse_input: Res<ButtonInput<MouseButton>>,
    hovered_entity: Res<HoveredEntity>,
    interactable_query: Query<&Interactable>,
    mut interaction_events: EventWriter<InteractionEvent>,
    console_open: Res<ConsoleOpen>,
) {
    // Don't interact when console is open
    if console_open.open {
        return;
    }

    if mouse_input.just_pressed(MouseButton::Left) {
        if let Some(entity) = hovered_entity.entity {
            if let Ok(interactable) = interactable_query.get(entity) {
                interaction_events.write(InteractionEvent {
                    entity,
                    interaction_type: interactable.interaction_type.clone(),
                });
                info!("Player interacted with entity {:?}", entity);
            }
        }
    }
}

/// System that handles interaction events and converts them to specific actions
fn handle_interaction_events(
    mut interaction_events: EventReader<InteractionEvent>,
    mut lamp_toggle_events: EventWriter<LampToggleEvent>,
    lamp_query: Query<&LampState>,
    device_query: Query<&DeviceEntity>,
) {
    for event in interaction_events.read() {
        match event.interaction_type {
            InteractionType::ToggleLamp => {
                // Get the device info for this entity
                if let Ok(device) = device_query.get(event.entity) {
                    // Check current lamp state if available
                    let current_state = lamp_query
                        .get(event.entity)
                        .map(|lamp| lamp.is_on)
                        .unwrap_or(false);

                    lamp_toggle_events.write(LampToggleEvent {
                        device_id: device.device_id.clone(),
                        new_state: !current_state,
                    });
                }
            }
        }
    }
}

/// System that handles lamp toggle events and sends MQTT messages
fn handle_lamp_toggle_events(
    mut lamp_toggle_events: EventReader<LampToggleEvent>,
    mut lamp_query: Query<&mut LampState>,
    device_query: Query<(Entity, &DeviceEntity)>,
    mut commands: Commands,
    mqtt_config: Res<MqttConfig>,
) {
    for event in lamp_toggle_events.read() {
        info!("Toggling lamp {} to {}", event.device_id, event.new_state);

        // Find the entity with this device_id
        let mut found_entity = None;
        for (entity, device) in device_query.iter() {
            if device.device_id == event.device_id {
                found_entity = Some(entity);
                break;
            }
        }

        if let Some(entity) = found_entity {
            // Update or add lamp state component
            if let Ok(mut lamp_state) = lamp_query.get_mut(entity) {
                lamp_state.is_on = event.new_state;
            } else {
                // Add lamp state component if it doesn't exist
                commands.entity(entity).insert(LampState {
                    is_on: event.new_state,
                    device_id: event.device_id.clone(),
                });
            }

            // Send MQTT message in a separate thread to avoid blocking
            let device_id = event.device_id.clone();
            let new_state = event.new_state;
            let mqtt_host = mqtt_config.host.clone();
            let mqtt_port = mqtt_config.port;

            std::thread::spawn(move || {
                info!(
                    "MQTT: Connecting player interaction client to publish to device {}",
                    device_id
                );
                let mut mqtt_options =
                    MqttOptions::new("player-interaction", &mqtt_host, mqtt_port);
                mqtt_options.set_keep_alive(Duration::from_secs(5));

                let (client, mut connection) = Client::new(mqtt_options, 10);
                let payload = if new_state { "ON" } else { "OFF" };
                let topic = format!("home/{}/light", device_id);

                info!(
                    "MQTT: Publishing player interaction command '{}' to topic '{}'",
                    payload, topic
                );
                match client.publish(&topic, QoS::AtMostOnce, false, payload.as_bytes()) {
                    Ok(_) => {
                        // Drive the event loop to ensure the message is sent
                        for notification in connection.iter() {
                            if let Ok(Event::Outgoing(Outgoing::Publish(_))) = notification {
                                info!(
                                    "MQTT: Player interaction command sent successfully: {} to {}",
                                    payload, topic
                                );
                                break;
                            }
                        }
                    }
                    Err(e) => error!("MQTT: Failed to publish player interaction message: {}", e),
                }
            });
        }
    }
}

/// System that updates lamp visual appearance based on their state
fn update_lamp_visuals(
    mut lamp_query: Query<(&LampState, &mut MeshMaterial3d<StandardMaterial>), Changed<LampState>>,
    hovered_entity: Res<HoveredEntity>,
    lamp_materials: Res<LampMaterials>,
    interactable_query: Query<Entity, With<Interactable>>,
) {
    // Update materials for lamps that changed state
    for (lamp_state, mut material) in lamp_query.iter_mut() {
        let new_material = if lamp_state.is_on {
            lamp_materials.lamp_on.clone()
        } else {
            lamp_materials.lamp_off.clone()
        };

        material.0 = new_material;
    }

    // Update hovered entity appearance
    if let Some(hovered) = hovered_entity.entity {
        if interactable_query.contains(hovered) {
            // This would ideally apply a hover effect, but we need a more complex system
            // to overlay the hover material properly. For now, we'll handle this in draw_interaction_cursor
        }
    }
}

/// System that draws a visual cursor/crosshair when hovering over interactable objects
fn draw_interaction_cursor(
    camera_query: Single<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    mut gizmos: Gizmos,
    hovered_entity: Res<HoveredEntity>,
    interactable_query: Query<&GlobalTransform, With<Interactable>>,
    console_open: Res<ConsoleOpen>,
    ground_query: Query<&GlobalTransform, With<Ground>>,
) {
    if console_open.open {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };

    let (camera, camera_transform) = *camera_query;

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // If we're hovering over an interactable entity, draw a special cursor
    if let Some(entity) = hovered_entity.entity {
        if let Ok(transform) = interactable_query.get(entity) {
            let entity_position = transform.translation();

            // Draw a targeting reticle around the interactable object
            gizmos.sphere(
                Isometry3d::new(entity_position, Quat::IDENTITY),
                0.6,
                Color::srgb(0.2, 1.0, 0.2), // Green for interactable
            );

            // Draw crosshair lines
            let size = 0.3;
            gizmos.line(
                entity_position + Vec3::new(-size, 0.0, 0.0),
                entity_position + Vec3::new(size, 0.0, 0.0),
                Color::srgb(0.2, 1.0, 0.2),
            );
            gizmos.line(
                entity_position + Vec3::new(0.0, -size, 0.0),
                entity_position + Vec3::new(0.0, size, 0.0),
                Color::srgb(0.2, 1.0, 0.2),
            );
            gizmos.line(
                entity_position + Vec3::new(0.0, 0.0, -size),
                entity_position + Vec3::new(0.0, 0.0, size),
                Color::srgb(0.2, 1.0, 0.2),
            );

            return;
        }
    }

    // Default cursor behavior - show where the player is looking (existing behavior)
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    // Try to find ground intersection for default cursor
    if let Ok(ground_transform) = ground_query.single() {
        if let Some(distance) = ray.intersect_plane(
            ground_transform.translation(),
            InfinitePlane3d::new(ground_transform.up()),
        ) {
            let point = ray.get_point(distance);

            // Draw default cursor
            gizmos.circle(
                Isometry3d::new(
                    point + ground_transform.up() * 0.01,
                    Quat::from_rotation_arc(Vec3::Z, ground_transform.up().as_vec3()),
                ),
                0.2,
                Color::WHITE,
            );
        }
    }
}
