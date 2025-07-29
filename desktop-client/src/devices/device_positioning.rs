use bevy::prelude::*;
use bevy_console::ConsoleOpen;
use log::{error, info};
use rumqttc::{Client, Event, MqttOptions, Outgoing, QoS};
use serde_json::json;
use std::time::Duration;
use super::device_types::*;
use crate::config::MqttConfig;

/// Component to mark a device as being dragged
#[derive(Component)]
pub struct BeingDragged {
    pub offset: Vec3, // Offset from cursor position to object center
}

/// Drag mode enum to specify which plane to drag on
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragMode {
    XY, // Drag on XY plane (horizontal)
    XZ, // Drag on XZ plane (horizontal ground level)
    YZ, // Drag on YZ plane (vertical wall)
}

impl Default for DragMode {
    fn default() -> Self {
        DragMode::XZ // Default to ground plane movement
    }
}

/// Resource to track drag state
#[derive(Resource, Default)]
pub struct DragState {
    pub dragging_entity: Option<Entity>,
    pub drag_mode: DragMode,
    pub drag_plane_origin: Vec3, // Origin point for the drag plane
}

/// Events for position updates
#[derive(Event)]
pub struct DevicePositionUpdateEvent {
    pub device_id: String,
    pub new_position: Vec3,
}

/// Component to mark the device info UI panel
#[derive(Component)]
struct DeviceInfoPanel;

/// Component for device info text elements
#[derive(Component)]
struct DeviceInfoText;

pub struct DevicePositioningPlugin;

impl Plugin for DevicePositioningPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DragState::default())
            .add_event::<DevicePositionUpdateEvent>()
        .add_systems(
            Update,
            (
                handle_device_drag_input,
                handle_device_dragging,
                handle_position_update_events,
                draw_drag_gizmos,
                update_device_info_ui,
            )
                .chain(),
        )
        .add_systems(Startup, setup_device_info_ui);
    }
}

fn setup_device_info_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(50.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                ..Default::default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.7)),
            Visibility::Hidden, // Initially hidden
            DeviceInfoPanel,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Device Info: "),
                TextFont {
                    font_size: 20.0,
                    ..Default::default()
                },
                TextColor(Color::WHITE),
                DeviceInfoText,
            ));
        });
}

fn update_device_info_ui(
    drag_state: Res<DragState>,
    mut text_query: Query<&mut Text, With<DeviceInfoText>>,
    mut panel_query: Query<&mut Visibility, (With<DeviceInfoPanel>, Without<DeviceInfoText>)>,
    device_query: Query<(&DeviceEntity, &Transform), With<BeingDragged>>,
) {
    if let Some(entity) = drag_state.dragging_entity {
        // Show the panel and update text when dragging
        if let Ok(mut visibility) = panel_query.single_mut() {
            *visibility = Visibility::Visible;
        }
        
        if let Ok((device, transform)) = device_query.get(entity) {
            if let Ok(mut text) = text_query.single_mut() {
                **text = format!(
                    "Device ID: {} | Position: ({:.2}, {:.2}, {:.2}) | Mode: {:?}",
                    device.device_id,
                    transform.translation.x,
                    transform.translation.y,
                    transform.translation.z,
                    drag_state.drag_mode
                );
            }
        }
    } else {
        // Hide the panel when not dragging
        if let Ok(mut visibility) = panel_query.single_mut() {
            *visibility = Visibility::Hidden;
        }
    }
}

/// System to handle picking devices for dragging with gizmo axis selection
fn handle_device_drag_input(
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    device_query: Query<
        (Entity, &GlobalTransform, &DeviceEntity),
        (With<DeviceEntity>, Without<BeingDragged>),
    >,
    mut commands: Commands,
    mut drag_state: ResMut<DragState>,
    console_open: Res<ConsoleOpen>,
) {
    // Don't interact when console is open
    if console_open.open {
        return;
    }

    // Handle keyboard shortcuts for switching drag modes
    if keyboard_input.just_pressed(KeyCode::KeyX) {
        drag_state.drag_mode = DragMode::YZ; // X-axis constrained, move in YZ plane
        info!("Drag mode: YZ plane (X-axis locked)");
    } else if keyboard_input.just_pressed(KeyCode::KeyY) {
        drag_state.drag_mode = DragMode::XZ; // Y-axis constrained, move in XZ plane
        info!("Drag mode: XZ plane (Y-axis locked)");
    } else if keyboard_input.just_pressed(KeyCode::KeyZ) {
        drag_state.drag_mode = DragMode::XY; // Z-axis constrained, move in XY plane
        info!("Drag mode: XY plane (Z-axis locked)");
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
            // Set drag plane origin to the device position
            drag_state.drag_plane_origin = device_position;

            // Calculate intersection based on current drag mode
            let plane_intersection = match drag_state.drag_mode {
                DragMode::XY => {
                    // XY plane at device's Z position
                    let t = (device_position.z - ray.origin.z) / ray.direction.z;
                    ray.origin + ray.direction * t
                }
                DragMode::XZ => {
                    // XZ plane at device's Y position
                    let t = (device_position.y - ray.origin.y) / ray.direction.y;
                    ray.origin + ray.direction * t
                }
                DragMode::YZ => {
                    // YZ plane at device's X position
                    let t = (device_position.x - ray.origin.x) / ray.direction.x;
                    ray.origin + ray.direction * t
                }
            };

            let offset = device_position - plane_intersection;

            commands.entity(entity).insert(BeingDragged { offset });
            drag_state.dragging_entity = Some(entity);
            info!(
                "Started dragging device at entity {:?} in {:?} mode",
                entity, drag_state.drag_mode
            );
        }
    }

    // Handle releasing drag
    if mouse_input.just_released(MouseButton::Right) || keyboard_input.just_pressed(KeyCode::Escape)
    {
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
        // Calculate intersection based on current drag mode
        let intersection = match drag_state.drag_mode {
            DragMode::XY => {
                // XY plane at drag origin's Z position
                let t = (drag_state.drag_plane_origin.z - ray.origin.z) / ray.direction.z;
                ray.origin + ray.direction * t
            }
            DragMode::XZ => {
                // XZ plane at drag origin's Y position
                let t = (drag_state.drag_plane_origin.y - ray.origin.y) / ray.direction.y;
                ray.origin + ray.direction * t
            }
            DragMode::YZ => {
                // YZ plane at drag origin's X position
                let t = (drag_state.drag_plane_origin.x - ray.origin.x) / ray.direction.x;
                ray.origin + ray.direction * t
            }
        };

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

/// System to draw axis gizmos and drag mode indicators
fn draw_drag_gizmos(
    mut gizmos: Gizmos,
    device_query: Query<&GlobalTransform, With<DeviceEntity>>,
    dragged_query: Query<&GlobalTransform, (With<DeviceEntity>, With<BeingDragged>)>,
    drag_state: Res<DragState>,
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

    // Draw bright outline and axis gizmos for devices being dragged
    for transform in dragged_query.iter() {
        let position = transform.translation();

        // Draw bright outline
        gizmos.cuboid(
            Transform::from_translation(position),
            Color::srgb(1.0, 1.0, 0.0), // Bright yellow
        );

        // Draw axis gizmos based on current drag mode
        let axis_length = 1.5;

        match drag_state.drag_mode {
            DragMode::XY => {
                // Highlight X and Y axes, dim Z axis
                gizmos.line(
                    position,
                    position + Vec3::X * axis_length,
                    Color::srgb(1.0, 0.0, 0.0),
                ); // Red X
                gizmos.line(
                    position,
                    position + Vec3::Y * axis_length,
                    Color::srgb(0.0, 1.0, 0.0),
                ); // Green Y
                gizmos.line(
                    position,
                    position + Vec3::Z * axis_length,
                    Color::srgba(0.0, 0.0, 1.0, 0.3),
                ); // Dim Blue Z
            }
            DragMode::XZ => {
                // Highlight X and Z axes, dim Y axis
                gizmos.line(
                    position,
                    position + Vec3::X * axis_length,
                    Color::srgb(1.0, 0.0, 0.0),
                ); // Red X
                gizmos.line(
                    position,
                    position + Vec3::Y * axis_length,
                    Color::srgba(0.0, 1.0, 0.0, 0.3),
                ); // Dim Green Y
                gizmos.line(
                    position,
                    position + Vec3::Z * axis_length,
                    Color::srgb(0.0, 0.0, 1.0),
                ); // Blue Z
            }
            DragMode::YZ => {
                // Highlight Y and Z axes, dim X axis
                gizmos.line(
                    position,
                    position + Vec3::X * axis_length,
                    Color::srgba(1.0, 0.0, 0.0, 0.3),
                ); // Dim Red X
                gizmos.line(
                    position,
                    position + Vec3::Y * axis_length,
                    Color::srgb(0.0, 1.0, 0.0),
                ); // Green Y
                gizmos.line(
                    position,
                    position + Vec3::Z * axis_length,
                    Color::srgb(0.0, 0.0, 1.0),
                ); // Blue Z
            }
        }

        // Draw axis labels
        // Note: Text rendering in gizmos is limited, but we can use simple indicators
        let label_offset = axis_length + 0.3;
        gizmos.sphere(
            Isometry3d::from_translation(position + Vec3::X * label_offset),
            0.1,
            Color::srgb(1.0, 0.0, 0.0),
        );
        gizmos.sphere(
            Isometry3d::from_translation(position + Vec3::Y * label_offset),
            0.1,
            Color::srgb(0.0, 1.0, 0.0),
        );
        gizmos.sphere(
            Isometry3d::from_translation(position + Vec3::Z * label_offset),
            0.1,
            Color::srgb(0.0, 0.0, 1.0),
        );
    }
}

/// Visual feedback for draggable devices (keeping for backward compatibility)
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
