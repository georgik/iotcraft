use bevy::prelude::*;
use bevy_console::{ConsoleCommand, reply};
use log::info;
use rumqttc::{Client, Event, MqttOptions, Outgoing, QoS};
use serde_json::json;
use std::fs;
use std::time::Duration;

use super::console_types::*;
use crate::config::MqttConfig;
use crate::devices::{DeviceEntity, device_positioning::DevicePositionUpdateEvent};
use crate::mqtt::TemperatureResource;
use crate::script::{ScriptExecutor, execute_script};

pub struct ConsolePlugin;

impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BlinkState::default())
            .add_systems(Update, handle_blink_command)
            .add_systems(Update, handle_mqtt_command)
            .add_systems(Update, handle_spawn_command)
            .add_systems(Update, handle_spawn_door_command)
            .add_systems(Update, handle_load_command)
            .add_systems(Update, handle_move_command);
    }
}

pub fn handle_blink_command(
    mut log: ConsoleCommand<BlinkCommand>,
    mut blink_state: ResMut<BlinkState>,
) {
    if let Some(Ok(BlinkCommand { action })) = log.take() {
        info!("Console command: blink {}", action);
        match action.as_str() {
            "start" => {
                blink_state.blinking = true;
                reply!(log, "Blink started");
                info!("Blink started via console");
            }
            "stop" => {
                blink_state.blinking = false;
                reply!(log, "Blink stopped");
                info!("Blink stopped via console");
            }
            _ => {
                reply!(log, "Usage: blink [start|stop]");
            }
        }
    }
}

pub fn handle_mqtt_command(
    mut log: ConsoleCommand<MqttCommand>,
    temperature: Res<TemperatureResource>,
) {
    if let Some(Ok(MqttCommand { action })) = log.take() {
        info!("Console command: mqtt {}", action);
        match action.as_str() {
            "status" => {
                let status = if temperature.value.is_some() {
                    "Connected to MQTT broker"
                } else {
                    "Connecting to MQTT broker..."
                };
                reply!(log, "{}", status);
                info!("MQTT status requested via console");
            }
            "temp" => {
                let temp_msg = if let Some(val) = temperature.value {
                    format!("Current temperature: {:.1}°C", val)
                } else {
                    "No temperature data available".to_string()
                };
                reply!(log, "{}", temp_msg);
            }
            _ => {
                reply!(log, "Usage: mqtt [status|temp]");
            }
        }
    }
}

pub fn handle_spawn_command(mut log: ConsoleCommand<SpawnCommand>, mqtt_config: Res<MqttConfig>) {
    if let Some(Ok(SpawnCommand { device_id, x, y, z })) = log.take() {
        info!("Console command: spawn {}", device_id);
        let payload = json!({
            "device_id": device_id,
            "device_type": "lamp",
            "state": "online",
            "location": { "x": x, "y": y, "z": z }
        })
        .to_string();

        // Create a temporary client for simulation
        let mut mqtt_options =
            MqttOptions::new("spawn-client", &mqtt_config.host, mqtt_config.port);
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

        reply!(log, "Spawn command sent for device {}", device_id);
    }
}

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

pub fn handle_load_command(
    mut log: ConsoleCommand<LoadCommand>,
    mut script_executor: ResMut<ScriptExecutor>,
) {
    if let Some(Ok(LoadCommand { filename })) = log.take() {
        info!("Console command: load {}", filename);
        match fs::read_to_string(&filename) {
            Ok(content) => {
                let commands = execute_script(&content);
                script_executor.commands = commands;
                script_executor.current_index = 0;
                reply!(
                    log,
                    "Loaded {} commands from {}",
                    script_executor.commands.len(),
                    filename
                );
                info!("Loaded script file: {}", filename);
            }
            Err(e) => {
                reply!(log, "Error loading script {}: {}", filename, e);
            }
        }
    }
}

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
