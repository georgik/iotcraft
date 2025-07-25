use bevy::prelude::*;
use bevy_console::ConsoleOpen;
use log::{error, info};
use rumqttc::{Client, Event, MqttOptions, Outgoing, QoS};
use serde_json::json;
use std::time::Duration;

use super::device_types::*;
use crate::config::MqttConfig;
use crate::environment::Ground;

/// Component to mark a device as being dragged
#[derive(Component)]
pub struct BeingDragged {
    pub offset: Vec3, // Offset from cursor position to object center
}

/// Resource to track drag state
#[derive(Resource, Default)]
pub struct DragState {
    pub dragging_entity: Option<Entity>,
    pub drag_plane_y: f32, // Y level to constrain dragging to
}

/// Events for position updates
#[derive(Event)]
pub struct DevicePositionUpdateEvent {
    pub device_id: String,
    pub new_position: Vec3,
}

pub struct DevicePositioningPlugin;

impl Plugin for DevicePositioningPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DragState {
            dragging_entity: None,
            drag_plane_y: 0.5,
        })
        .add_event::<DevicePositionUpdateEvent>()
        .add_systems(
            Update,
            (
                handle_device_drag_input,
                handle_device_dragging,
                handle_position_update_events,
            )
                .chain(),
        );
    }
}

/// System to handle picking devices for dragging
fn handle_device_drag_input(
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    device_query: Query<
        (Entity, &GlobalTransform, &DeviceEntity),
        (With<DeviceEntity>, Without<BeingDragged>),
    >,
    dragged_query: Query<Entity, With<BeingDragged>>,
    mut commands: Commands,
    mut drag_state: ResMut<DragState>,
    console_open: Res<ConsoleOpen>,
) {
    // Don't interact when console is open
    if console_open.open {
        return;
    }

    // Handle right-click to start dragging (left-click is for lamp interaction)
    if mouse_input.just_pressed(MouseButton::Right) {
        let Ok(window) = windows.single() else {
            return;
        };
        let (camera, camera_transform) = *camera_query;
        let Some(cursor_position) = window.cursor_position() else {
            return;
        };

        let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
            return;
        };

        let mut closest_device = None;
        let mut closest_distance = f32::MAX;

        // Find the closest device to the cursor
        for (entity, transform, _device) in device_query.iter() {
            let device_position = transform.translation();

            // Simple sphere collision detection
            let sphere_radius = 0.7;
            let to_sphere = device_position - ray.origin;
            let projection_length = to_sphere.dot(*ray.direction);

            if projection_length < 0.0 {
                continue;
            }

            let closest_point = ray.origin + ray.direction * projection_length;
            let distance_to_sphere = (closest_point - device_position).length();

            if distance_to_sphere <= sphere_radius && projection_length < closest_distance {
                closest_distance = projection_length;
                closest_device = Some((entity, device_position));
            }
        }

        // Start dragging the closest device
        if let Some((entity, device_position)) = closest_device {
            // Calculate offset from cursor ray intersection with drag plane to device center
            let drag_plane_intersection = ray.origin
                + ray.direction * ((drag_state.drag_plane_y - ray.origin.y) / ray.direction.y);
            let offset = device_position - drag_plane_intersection;

            commands.entity(entity).insert(BeingDragged { offset });
            drag_state.dragging_entity = Some(entity);
            info!("Started dragging device at entity {:?}", entity);
        }
    }

    // Handle releasing drag - position updates will be sent by handle_device_dragging system
    if mouse_input.just_released(MouseButton::Right) || keyboard_input.just_pressed(KeyCode::Escape)
    {
        // The cleanup will be handled by handle_device_dragging after sending position updates
        info!("Stopped dragging devices");
    }
}

/// System to handle updating device positions while being dragged
fn handle_device_dragging(
    camera_query: Single<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    mut dragged_query: Query<(&mut Transform, &BeingDragged, &DeviceEntity)>,
    mut position_update_events: EventWriter<DevicePositionUpdateEvent>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut drag_state: ResMut<DragState>,
    mut commands: Commands,
    console_open: Res<ConsoleOpen>,
) {
    // Don't interact when console is open
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

    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    for (mut transform, being_dragged, device) in dragged_query.iter_mut() {
        // Calculate intersection with the drag plane
        let t = (drag_state.drag_plane_y - ray.origin.y) / ray.direction.y;
        let intersection = ray.origin + ray.direction * t;

        // Apply the offset to get the new device position
        let new_position = intersection + being_dragged.offset;
        transform.translation = new_position;

        // Send position update when drag is released
        if mouse_input.just_released(MouseButton::Right) {
            position_update_events.write(DevicePositionUpdateEvent {
                device_id: device.device_id.clone(),
                new_position,
            });
        }
    }

    // Clean up dragging state after processing all dragged entities
    if mouse_input.just_released(MouseButton::Right) || keyboard_input.just_pressed(KeyCode::Escape)
    {
        // Remove BeingDragged component from the dragged entity
        if let Some(entity) = drag_state.dragging_entity {
            commands.entity(entity).remove::<BeingDragged>();
        }
        // Reset drag state
        drag_state.dragging_entity = None;
    }
}

/// System to handle position update events and send MQTT messages
fn handle_position_update_events(
    mut position_events: EventReader<DevicePositionUpdateEvent>,
    mqtt_config: Res<MqttConfig>,
) {
    for event in position_events.read() {
        info!(
            "Updating position for device {} to {:?}",
            event.device_id, event.new_position
        );

        // Send MQTT position update in a separate thread
        let device_id = event.device_id.clone();
        let position = event.new_position;

        let mqtt_host = mqtt_config.host.clone();
        let mqtt_port = mqtt_config.port;

        std::thread::spawn(move || {
            info!("MQTT: Sending position update for device {}", device_id);
            let mut opts = MqttOptions::new("position-updater", &mqtt_host, mqtt_port);
            opts.set_keep_alive(Duration::from_secs(5));
            let (client, mut connection) = Client::new(opts, 10);

            let topic = format!("home/{}/position/set", device_id);
            let payload = json!({
                "x": position.x,
                "y": position.y,
                "z": position.z
            })
            .to_string();

            info!(
                "MQTT: Publishing position update '{}' to topic '{}'",
                payload, topic
            );
            match client.publish(&topic, QoS::AtMostOnce, false, payload.as_bytes()) {
                Ok(_) => {
                    // Drive until publish is sent
                    for notification in connection.iter() {
                        if let Ok(Event::Outgoing(Outgoing::Publish(_))) = notification {
                            info!(
                                "MQTT: Position update sent successfully for device {}",
                                device_id
                            );
                            break;
                        }
                    }
                }
                Err(e) => error!("MQTT: Failed to publish position update: {}", e),
            }
        });
    }
}

/// Visual feedback for draggable devices
pub fn draw_drag_feedback(
    mut gizmos: Gizmos,
    device_query: Query<&GlobalTransform, With<DeviceEntity>>,
    dragged_query: Query<&GlobalTransform, (With<DeviceEntity>, With<BeingDragged>)>,
    console_open: Res<ConsoleOpen>,
) {
    if console_open.open {
        return;
    }

    // Draw subtle outline for all devices to show they're draggable
    for transform in device_query.iter() {
        let position = transform.translation();
        gizmos.cuboid(
            Transform::from_translation(position),
            Color::srgba(0.5, 0.5, 1.0, 0.3), // Semi-transparent blue
        );
    }

    // Draw bright outline for devices being dragged
    for transform in dragged_query.iter() {
        let position = transform.translation();
        gizmos.cuboid(
            Transform::from_translation(position),
            Color::srgb(1.0, 1.0, 0.0), // Bright yellow
        );
    }
}
