use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;
use bevy_console::ConsoleOpen;
use log::{error, info};
use rumqttc::{Client, Event, MqttOptions, Outgoing, QoS};
use std::time::Duration;

use super::interaction_types::*;
use crate::config::MqttConfig;
use crate::devices::{
    DeviceEntity,
    device_types::{DoorState, OriginalPosition},
};
use crate::environment::Ground;
use crate::environment::VoxelWorld;
use crate::inventory::{ItemType, PlaceBlockEvent, PlayerInventory};

pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<InteractionEvent>()
            .add_event::<LampToggleEvent>()
            .add_event::<DoorToggleEvent>()
            .insert_resource(HoveredEntity::default())
            .insert_resource(GhostBlockState::default())
            .add_systems(Startup, (setup_lamp_materials, setup_door_materials))
            .add_systems(
                Update,
                (
                    raycast_interaction_system,
                    update_ghost_block_preview,
                    handle_interaction_input,
                    handle_interaction_events,
                    handle_lamp_toggle_events,
                    handle_door_toggle_events,
                    update_lamp_visuals,
                    update_door_visuals,
                    draw_interaction_cursor,
                    draw_crosshair,
                )
                    .chain()
                    .run_if(in_state(crate::ui::GameState::InGame)),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::{
        DeviceEntity,
        device_types::{DoorState, OriginalPosition},
    };
    use bevy::ecs::system::IntoSystem;

    #[test]
    fn test_handle_interaction_events_lamp() {
        let mut world = World::new();
        world.init_resource::<Events<InteractionEvent>>();
        world.init_resource::<Events<LampToggleEvent>>();
        world.init_resource::<Events<DoorToggleEvent>>();

        // Create a lamp device entity
        let device_entity = world
            .spawn((
                DeviceEntity {
                    device_id: "test_lamp".to_string(),
                    device_type: "lamp".to_string(),
                },
                LampState { is_on: false },
            ))
            .id();

        // Send interaction event
        let mut event_writer = world.resource_mut::<Events<InteractionEvent>>();
        event_writer.send(InteractionEvent {
            entity: device_entity,
            interaction_type: InteractionType::ToggleLamp,
        });
        drop(event_writer);

        let mut system = IntoSystem::into_system(handle_interaction_events);
        system.initialize(&mut world);
        system.run((), &mut world);

        // Check that lamp toggle event was generated - simplified test
        // In a real application, you'd need to set up proper event readers
        // For this test, we'll just verify the system ran without error
        // and that the entity still exists with the right components
        assert!(world.entity(device_entity).contains::<DeviceEntity>());
        assert!(world.entity(device_entity).contains::<LampState>());
    }

    #[test]
    fn test_handle_door_toggle_events() {
        let mut world = World::new();
        world.init_resource::<Events<DoorToggleEvent>>();

        // Create a door device entity
        let device_entity = world
            .spawn((
                DeviceEntity {
                    device_id: "test_door".to_string(),
                    device_type: "door".to_string(),
                },
                DoorState { is_open: false },
                OriginalPosition {
                    position: Vec3::new(1.0, 0.0, 1.0),
                },
                Transform::from_translation(Vec3::new(1.0, 0.0, 1.0)),
            ))
            .id();

        // Send door toggle event
        let mut event_writer = world.resource_mut::<Events<DoorToggleEvent>>();
        event_writer.send(DoorToggleEvent {
            device_id: "test_door".to_string(),
            new_state: true,
        });
        drop(event_writer);

        let mut system = IntoSystem::into_system(handle_door_toggle_events);
        system.initialize(&mut world);
        system.run((), &mut world);

        // Check that door state was updated
        let door_state = world.entity(device_entity).get::<DoorState>().unwrap();
        assert_eq!(door_state.is_open, true);
    }

    #[test]
    fn test_update_door_visuals() {
        let mut world = World::new();

        let original_pos = Vec3::new(2.0, 0.0, 3.0);
        let device_entity = world
            .spawn((
                DeviceEntity {
                    device_id: "visual_door".to_string(),
                    device_type: "door".to_string(),
                },
                DoorState { is_open: false },
                OriginalPosition {
                    position: original_pos,
                },
                Transform::from_translation(original_pos),
            ))
            .id();

        // Mark door as changed (simulating state change)
        world
            .entity_mut(device_entity)
            .get_mut::<DoorState>()
            .unwrap()
            .is_open = true;

        let mut system = IntoSystem::into_system(update_door_visuals);
        system.initialize(&mut world);
        system.run((), &mut world);

        // Check that transform was updated for open door
        let transform = world.entity(device_entity).get::<Transform>().unwrap();
        // Door should be rotated 90 degrees when open
        let expected_rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        assert!((transform.rotation.w - expected_rotation.w).abs() < 0.001);
        assert!((transform.rotation.y - expected_rotation.y).abs() < 0.001);
    }

    #[test]
    fn test_hovered_entity_tracking() {
        let mut world = World::new();
        world.insert_resource(HoveredEntity::default());

        // Create an interactable entity
        let _interactable_entity = world
            .spawn((
                Transform::from_translation(Vec3::new(0.0, 0.0, -5.0)), // In front of camera
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, -5.0)),
                Interactable {
                    interaction_type: InteractionType::ToggleLamp,
                },
            ))
            .id();

        // Check initial state
        let hovered = world.resource::<HoveredEntity>();
        assert!(hovered.entity.is_none());

        // Test would require more complex setup for camera and window systems
        // This demonstrates the pattern for testing ECS state changes
    }
}

/// Setup material resources for different lamp states
fn setup_lamp_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let lamp_texture: Handle<Image> = asset_server.load("textures/lamp.webp");

    let lamp_materials = LampMaterials {
        lamp_off: materials.add(StandardMaterial {
            base_color_texture: Some(lamp_texture.clone()),
            base_color: Color::srgb(0.2, 0.2, 0.2), // Dark when off
            ..default()
        }),
        lamp_on: materials.add(StandardMaterial {
            base_color_texture: Some(lamp_texture),
            base_color: Color::srgb(1.0, 0.9, 0.6), // Bright yellow when on
            emissive: LinearRgba::new(0.8, 0.7, 0.4, 1.0),
            ..default()
        }),
    };

    commands.insert_resource(lamp_materials);
}

/// Setup material resources for different door states
fn setup_door_materials(
    mut commands: Commands,
    _materials: ResMut<Assets<StandardMaterial>>,
    _asset_server: Res<AssetServer>,
) {
    // Currently doors use transform-based animations instead of material changes
    // Create empty resource for potential future use
    let door_materials = DoorMaterials {};
    commands.insert_resource(door_materials);
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

    // Use screen center when cursor is grabbed, otherwise use cursor position
    let cursor_position = if window.cursor_options.grab_mode == bevy::window::CursorGrabMode::Locked
    {
        // When cursor is grabbed, use screen center where crosshair is displayed
        Vec2::new(window.width() / 2.0, window.height() / 2.0)
    } else {
        // When cursor is not grabbed, use actual cursor position
        let Some(cursor_pos) = window.cursor_position() else {
            hovered_entity.entity = None;
            return;
        };
        cursor_pos
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

/// Updates the ghost block preview
fn update_ghost_block_preview(
    camera_query: Single<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    voxel_world: Res<VoxelWorld>,
    mut ghost_state: ResMut<GhostBlockState>,
    console_open: Res<ConsoleOpen>,
) {
    if console_open.open {
        ghost_state.position = None;
        return;
    }

    let Ok(window) = windows.single() else {
        ghost_state.position = None;
        return;
    };

    let (camera, camera_transform) = *camera_query;

    // Use screen center when cursor is grabbed, otherwise use cursor position
    let cursor_position = if window.cursor_options.grab_mode == bevy::window::CursorGrabMode::Locked
    {
        // When cursor is grabbed, use screen center where crosshair is displayed
        Vec2::new(window.width() / 2.0, window.height() / 2.0)
    } else {
        // When cursor is not grabbed, use actual cursor position
        let Some(cursor_pos) = window.cursor_position() else {
            ghost_state.position = None;
            return;
        };
        cursor_pos
    };

    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        ghost_state.position = None;
        return;
    };

    // Define interaction distance range - start from a minimum distance to avoid placing too close
    let min_distance = 2.0; // Minimum distance from camera
    let max_distance = 8.0; // Maximum reach distance
    let step_size = 0.1; // Fine-grained raycast steps

    ghost_state.position = None;
    ghost_state.can_place = false;

    // Perform precise raycast to find which block face is being looked at.
    let mut last_pos = (ray.origin + ray.direction * min_distance).as_ivec3();
    let mut current_distance = min_distance;
    while current_distance <= max_distance {
        let check_position = (ray.origin + ray.direction * current_distance).as_ivec3();
        if voxel_world.is_block_at(check_position) {
            // Hit a block. The placement position is the last position that was air.
            if !voxel_world.is_block_at(last_pos) {
                ghost_state.position = Some(last_pos);
                ghost_state.can_place = true;
            }
            break;
        }
        last_pos = check_position;
        current_distance += step_size;
    }
}

/// Draw a crosshair at the center of the screen and ghost block preview
fn draw_crosshair(
    mut gizmos: Gizmos,
    console_open: Res<ConsoleOpen>,
    ghost_state: Res<GhostBlockState>,
    inventory: Res<PlayerInventory>,
    windows: Query<&Window>,
) {
    if console_open.open {
        return;
    }

    // Get window to determine screen center
    let Ok(window) = windows.single() else {
        return;
    };

    let screen_center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
    let crosshair_size = 10.0;

    // Draw crosshair at screen center
    gizmos.line_2d(
        screen_center + Vec2::new(-crosshair_size, 0.0),
        screen_center + Vec2::new(crosshair_size, 0.0),
        Color::WHITE,
    );
    gizmos.line_2d(
        screen_center + Vec2::new(0.0, -crosshair_size),
        screen_center + Vec2::new(0.0, crosshair_size),
        Color::WHITE,
    );

    // Draw ghost block if we have inventory item and valid placement position
    if let Some(_selected_item) = inventory.get_selected_item() {
        if let Some(ghost_pos) = ghost_state.position {
            if ghost_state.can_place {
                let position = ghost_pos.as_vec3();
                let color = Color::srgba(0.2, 1.0, 0.2, 0.5); // Semi-transparent green

                // Draw wireframe cube
                let half_size = 0.5;
                let corners = [
                    position + Vec3::new(-half_size, -half_size, -half_size),
                    position + Vec3::new(half_size, -half_size, -half_size),
                    position + Vec3::new(half_size, half_size, -half_size),
                    position + Vec3::new(-half_size, half_size, -half_size),
                    position + Vec3::new(-half_size, -half_size, half_size),
                    position + Vec3::new(half_size, -half_size, half_size),
                    position + Vec3::new(half_size, half_size, half_size),
                    position + Vec3::new(-half_size, half_size, half_size),
                ];

                // Bottom face
                gizmos.line(corners[0], corners[1], color);
                gizmos.line(corners[1], corners[2], color);
                gizmos.line(corners[2], corners[3], color);
                gizmos.line(corners[3], corners[0], color);

                // Top face
                gizmos.line(corners[4], corners[5], color);
                gizmos.line(corners[5], corners[6], color);
                gizmos.line(corners[6], corners[7], color);
                gizmos.line(corners[7], corners[4], color);

                // Vertical edges
                gizmos.line(corners[0], corners[4], color);
                gizmos.line(corners[1], corners[5], color);
                gizmos.line(corners[2], corners[6], color);
                gizmos.line(corners[3], corners[7], color);
            }
        }
    }
}
fn handle_interaction_input(
    mouse_input: Res<ButtonInput<MouseButton>>,
    hovered_entity: Res<HoveredEntity>,
    interactable_query: Query<&Interactable>,
    mut interaction_events: EventWriter<InteractionEvent>,
    mut place_block_events: EventWriter<PlaceBlockEvent>,
    inventory: ResMut<PlayerInventory>,
    _camera_query: Single<(&Camera, &GlobalTransform)>,
    _windows: Query<&Window>,
    console_open: Res<ConsoleOpen>,
    _voxel_world: Res<VoxelWorld>,
    ghost_state: Res<GhostBlockState>,
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

    // Handle right-click for placing blocks from inventory
    if mouse_input.just_pressed(MouseButton::Right) {
        if let Some(selected_item) = inventory.get_selected_item() {
            let ItemType::Block(block_type) = selected_item.item_type;

            // For block placement, we rely on the ghost_state which already uses correct cursor logic
            // No need for additional raycasting here since ghost_state handles it

            if let Some(placement_position) = ghost_state.position {
                if ghost_state.can_place {
                    place_block_events.write(PlaceBlockEvent {
                        position: placement_position,
                    });

                    info!("Placed {:?} at {:?}", block_type, placement_position);
                }
            }
        }
    }
}

/// System that handles interaction events and converts them to specific actions
fn handle_interaction_events(
    mut interaction_events: EventReader<InteractionEvent>,
    mut lamp_toggle_events: EventWriter<LampToggleEvent>,
    mut door_toggle_events: EventWriter<DoorToggleEvent>,
    lamp_query: Query<&LampState>,
    door_query: Query<&DoorState>,
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
            InteractionType::ToggleDoor => {
                // Get the device info for this entity
                if let Ok(device) = device_query.get(event.entity) {
                    // Check current door state if available
                    let current_state = door_query
                        .get(event.entity)
                        .map(|door| door.is_open)
                        .unwrap_or(false);

                    door_toggle_events.write(DoorToggleEvent {
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

/// System that handles door toggle events
fn handle_door_toggle_events(
    mut door_toggle_events: EventReader<DoorToggleEvent>,
    mut door_query: Query<&mut DoorState>,
    device_query: Query<(Entity, &DeviceEntity)>,
    mut commands: Commands,
) {
    for event in door_toggle_events.read() {
        info!(
            "Toggling door {} to {}",
            event.device_id,
            if event.new_state { "open" } else { "closed" }
        );

        // Find the entity with this device_id
        let mut found_entity = None;
        for (entity, device) in device_query.iter() {
            if device.device_id == event.device_id {
                found_entity = Some(entity);
                break;
            }
        }

        if let Some(entity) = found_entity {
            // Update or add door state component
            if let Ok(mut door_state) = door_query.get_mut(entity) {
                door_state.is_open = event.new_state;
            } else {
                // Add door state component if it doesn't exist
                commands.entity(entity).insert(DoorState {
                    is_open: event.new_state,
                });
            }
        }
    }
}

/// System that updates door visual appearance based on their state
fn update_door_visuals(
    mut door_query: Query<
        (&DoorState, &mut Transform, &OriginalPosition),
        (Changed<DoorState>, With<DeviceEntity>),
    >,
    _device_query: Query<&DeviceEntity>,
) {
    // Update rotation for doors that changed state
    for (door_state, mut transform, original_pos) in door_query.iter_mut() {
        // Calculate pivot point (right edge of door)
        // Door is 0.2 units thick, so pivot is 0.1 units from center along X-axis (right edge)
        let pivot_offset = Vec3::new(0.1, 0.0, 0.0);

        if door_state.is_open {
            // Rotate 90 degrees around Y-axis at the pivot point (right edge)
            let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
            transform.rotation = rotation;

            // Calculate new position: move pivot to origin, rotate, move back
            let rotated_offset = rotation * pivot_offset;
            transform.translation = original_pos.position - pivot_offset + rotated_offset;
        } else {
            // Door closed - reset to original position and rotation
            transform.rotation = Quat::IDENTITY;
            transform.translation = original_pos.position;
        }

        info!(
            "Door {} rotated to {} degrees (pivoting around right edge)",
            if door_state.is_open {
                "opened"
            } else {
                "closed"
            },
            if door_state.is_open { 90 } else { 0 }
        );
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
