#[cfg(feature = "console")]
use crate::{console::ConsoleCommand, reply};
#[cfg(feature = "console")]
use bevy::prelude::*;
#[cfg(feature = "console")]
use log::{error, info};
#[cfg(feature = "console")]
use rumqttc::{Client, Event, MqttOptions, Outgoing, QoS};
#[cfg(feature = "console")]
use serde_json::json;
#[cfg(feature = "console")]
use std::time::Duration;

#[cfg(feature = "console")]
use super::console_types::*;
#[cfg(feature = "console")]
use crate::config::MqttConfig;
#[cfg(feature = "console")]
use crate::devices::{DeviceEntity, device_positioning::DevicePositionUpdateEvent};

#[cfg(feature = "console")]
pub fn handle_spawn_door_command(
    mut log: ConsoleCommand<SpawnDoorCommand>,
    mqtt_config: Res<MqttConfig>,
) {
    if let Some(Ok(SpawnDoorCommand { device_id, x, y, z })) = log.take() {
        info!("Console command: spawn_door {}", device_id);
        let payload = json!({
            "device_id": device_id,
            "device_type": "door",
            "state": "online",
            "location": { "x": x, "y": y, "z": z }
        })
        .to_string();

        // Create a temporary client for simulation
        let mut mqtt_options =
            MqttOptions::new("spawn-door-client", &mqtt_config.host, mqtt_config.port);
        mqtt_options.set_keep_alive(Duration::from_secs(5));
        let (client, mut connection) = Client::new(mqtt_options, 10);

        client
            .publish(
                "devices/announce",
                QoS::AtMostOnce,
                false,
                payload.as_bytes(),
            )
            .unwrap();

        // Drive the event loop to ensure the message is sent
        for notification in connection.iter() {
            if let Ok(Event::Outgoing(Outgoing::Publish(_))) = notification {
                break;
            }
        }

        reply!(log, "Spawn door command sent for device {}", device_id);
    }
}

#[cfg(feature = "console")]
pub fn handle_move_command(
    mut log: ConsoleCommand<MoveCommand>,
    mut device_query: Query<(&mut Transform, &DeviceEntity)>,
    mut position_events: EventWriter<DevicePositionUpdateEvent>,
) {
    if let Some(Ok(MoveCommand { device_id, x, y, z })) = log.take() {
        info!("Console command: move {} {} {} {}", device_id, x, y, z);

        let new_position = Vec3::new(x, y, z);
        let mut device_found = false;

        // Find and update the device position
        for (mut transform, device) in device_query.iter_mut() {
            if device.device_id == device_id {
                transform.translation = new_position;
                device_found = true;

                // Send position update event
                position_events.write(DevicePositionUpdateEvent {
                    device_id: device_id.clone(),
                    new_position,
                });

                break;
            }
        }

        if device_found {
            reply!(log, "Moved device {} to ({}, {}, {})", device_id, x, y, z);
            info!(
                "Device {} moved to ({}, {}, {}) via console",
                device_id, x, y, z
            );
        } else {
            reply!(log, "Device {} not found", device_id);
            info!("Device {} not found for move command", device_id);
        }
    }
}

#[cfg(feature = "console")]
pub fn handle_test_error_command(
    mut log: ConsoleCommand<TestErrorCommand>,
    mut error_resource: ResMut<crate::ui::error_indicator::ErrorResource>,
    time: Res<Time>,
) {
    if let Some(Ok(TestErrorCommand { message })) = log.take() {
        error!("Test error command: {}", message);

        // Trigger the error indicator
        error_resource.indicator_on = true;
        error_resource.last_error_time = time.elapsed_secs();
        error_resource.messages.push(message.clone());

        // The error message will appear in the UI indicator and in the console reply
        reply!(log, "ERROR: {}", message);
    }
}

#[cfg(feature = "console")]
pub fn handle_list_command(
    mut log: ConsoleCommand<ListCommand>,
    device_query: Query<(&Transform, &DeviceEntity)>,
) {
    if let Some(Ok(_list_command)) = log.take() {
        info!("Console command: list");

        let device_count = device_query.iter().count();

        if device_count == 0 {
            reply!(log, "No connected devices found.");
            info!("List command: no devices found");
            return;
        }

        reply!(log, "Connected devices ({}):", device_count);

        for (transform, device) in device_query.iter() {
            let pos = transform.translation;
            reply!(
                log,
                "- ID: {} | Type: {} | Position: ({:.2}, {:.2}, {:.2})",
                device.device_id,
                device.device_type,
                pos.x,
                pos.y,
                pos.z
            );
        }

        info!("Listed {} connected devices via console", device_count);
    }
}
