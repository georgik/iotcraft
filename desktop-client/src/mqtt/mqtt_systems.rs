use bevy::prelude::*;
use rumqttc::{Client, Event, Incoming, MqttOptions, QoS};
use std::sync::Mutex;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::mqtt_types::*;
use crate::devices::DeviceAnnouncementReceiver;

pub struct MqttPlugin;

impl Plugin for MqttPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TemperatureResource::default())
            .add_systems(Startup, spawn_mqtt_subscriber)
            .add_systems(Update, update_temperature);
    }
}

/// Spawn a background thread to subscribe to the temperature topic and feed readings into the channel.
pub fn spawn_mqtt_subscriber(mut commands: Commands) {
    let (tx, rx) = mpsc::channel::<f32>();
    thread::spawn(move || {
        info!("MQTT: Starting temperature subscriber on port 1883");
        let mut mqttoptions = MqttOptions::new("desktop-subscriber", "localhost", 1883);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        let (client, mut connection) = Client::new(mqttoptions, 10);

        match client.subscribe("home/sensor/temperature", QoS::AtMostOnce) {
            Ok(_) => info!("MQTT: Successfully subscribed to home/sensor/temperature"),
            Err(e) => error!("MQTT: Failed to subscribe to temperature topic: {}", e),
        }

        for notification in connection.iter() {
            match notification {
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    info!("MQTT: Received temperature message on topic: {}", p.topic);
                    if let Ok(s) = String::from_utf8(p.payload.to_vec()) {
                        if let Ok(val) = s.parse::<f32>() {
                            let _ = tx.send(val);
                        }
                    }
                }
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    info!("MQTT: Temperature subscriber connected successfully");
                }
                Err(e) => {
                    error!("MQTT: Temperature subscriber error: {}", e);
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
    thread::spawn(move || {
        info!("MQTT: Starting device announcements subscriber on port 1883");
        let mut mqttoptions = MqttOptions::new("device-subscriber", "localhost", 1883);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        let (client, mut connection) = Client::new(mqttoptions, 10);

        match client.subscribe("devices/announce", QoS::AtMostOnce) {
            Ok(_) => info!("MQTT: Successfully subscribed to devices/announce"),
            Err(e) => error!("MQTT: Failed to subscribe to devices/announce: {}", e),
        }

        for notification in connection.iter() {
            match notification {
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    info!(
                        "MQTT: Received device announcement on topic: {} with payload: {}",
                        p.topic,
                        String::from_utf8_lossy(&p.payload)
                    );
                    if let Ok(s) = String::from_utf8(p.payload.to_vec()) {
                        let _ = device_tx.send(s);
                    }
                }
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    info!("MQTT: Device announcements subscriber connected successfully");
                }
                Err(e) => {
                    error!("MQTT: Device announcements subscriber error: {}", e);
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
