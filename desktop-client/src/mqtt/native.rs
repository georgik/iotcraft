use bevy::prelude::*;
use rumqttc::{Client, Event, Incoming, MqttOptions, QoS};
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::mqtt_types::*;
use crate::config::MqttConfig;
use crate::devices::DeviceAnnouncementReceiver;
use crate::multiplayer::mqtt_utils::generate_unique_client_id;
use crate::profile::PlayerProfile;

/// Native MQTT implementation for desktop using rumqttc
pub struct NativeMqttPlugin;

impl Plugin for NativeMqttPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TemperatureResource::default())
            .add_systems(Startup, spawn_native_mqtt_subscriber)
            .add_systems(Update, update_temperature);
    }
}

/// Spawn native MQTT subscribers using rumqttc (desktop only)
pub fn spawn_native_mqtt_subscriber(
    mut commands: Commands,
    mqtt_config: Res<MqttConfig>,
    _profile: Res<PlayerProfile>,
) {
    let (tx, rx) = mpsc::channel::<f32>();
    let mqtt_host = mqtt_config.host.clone();
    let mqtt_port = mqtt_config.port;
    let temp_client_id = generate_unique_client_id("temperature-sensor-sub");

    thread::spawn(move || {
        info!(
            "MQTT Native: Starting temperature subscriber on {}:{}",
            mqtt_host, mqtt_port
        );
        let mut mqttoptions = MqttOptions::new(&temp_client_id, &mqtt_host, mqtt_port);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        let (client, mut connection) = Client::new(mqttoptions, 10);

        match client.subscribe("home/sensor/temperature", QoS::AtMostOnce) {
            Ok(_) => info!("MQTT Native: Successfully subscribed to home/sensor/temperature"),
            Err(e) => error!(
                "MQTT Native: Failed to subscribe to temperature topic: {}",
                e
            ),
        }

        for notification in connection.iter() {
            match notification {
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    info!(
                        "MQTT Native: Received temperature message on topic: {}",
                        p.topic
                    );
                    if let Ok(s) = String::from_utf8(p.payload.to_vec()) {
                        if let Ok(val) = s.parse::<f32>() {
                            let _ = tx.send(val);
                        }
                    }
                }
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    info!("MQTT Native: Temperature subscriber connected successfully");
                }
                Err(e) => {
                    error!("MQTT Native: Temperature subscriber error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });
    commands.insert_resource(TemperatureReceiver(Mutex::new(rx)));

    // Create device announcement channel
    let (device_tx, device_rx) = mpsc::channel::<String>();
    commands.insert_resource(DeviceAnnouncementReceiver(Mutex::new(device_rx)));

    // Subscribe to device announcements
    let mqtt_host2 = mqtt_config.host.clone();
    let mqtt_port2 = mqtt_config.port;
    let device_client_id = generate_unique_client_id("device-announcements-sub");

    thread::spawn(move || {
        info!(
            "MQTT Native: Starting device announcements subscriber on {}:{}",
            mqtt_host2, mqtt_port2
        );
        let mut mqttoptions = MqttOptions::new(&device_client_id, &mqtt_host2, mqtt_port2);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        let (client, mut connection) = Client::new(mqttoptions, 10);

        match client.subscribe("devices/announce", QoS::AtMostOnce) {
            Ok(_) => info!("MQTT Native: Successfully subscribed to devices/announce"),
            Err(e) => error!(
                "MQTT Native: Failed to subscribe to devices/announce: {}",
                e
            ),
        }

        for notification in connection.iter() {
            match notification {
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    info!(
                        "MQTT Native: Received device announcement on topic: {} with payload: {}",
                        p.topic,
                        String::from_utf8_lossy(&p.payload)
                    );
                    if let Ok(s) = String::from_utf8(p.payload.to_vec()) {
                        let _ = device_tx.send(s);
                    }
                }
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    info!("MQTT Native: Device announcements subscriber connected successfully");
                }
                Err(e) => {
                    error!("MQTT Native: Device announcements subscriber error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });
}

pub fn update_temperature(
    mut temp_res: ResMut<TemperatureResource>,
    receiver: Res<TemperatureReceiver>,
) {
    if let Ok(rx) = receiver.0.lock() {
        if let Ok(val) = rx.try_recv() {
            temp_res.value = Some(val);
        }
    }
}
