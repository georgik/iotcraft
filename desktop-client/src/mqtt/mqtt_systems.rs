use bevy::prelude::*;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::sync::Mutex;
use rumqttc::{Client, MqttOptions, QoS, Event, Incoming};

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
        let mut mqttoptions = MqttOptions::new("desktop-subscriber", "127.0.0.1", 1883);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        let (client, mut connection) = Client::new(mqttoptions, 10);
        client.subscribe("home/sensor/temperature", QoS::AtMostOnce).unwrap();
        for notification in connection.iter() {
            if let Ok(Event::Incoming(Incoming::Publish(p))) = notification {
                if let Ok(s) = String::from_utf8(p.payload.to_vec()) {
                    if let Ok(val) = s.parse::<f32>() {
                        let _ = tx.send(val);
                    }
                }
            }
        }
    });
    commands.insert_resource(TemperatureReceiver(Mutex::new(rx)));

    // Create device announcement channel
    let (device_tx, device_rx) = mpsc::channel::<String>();
    commands.insert_resource(DeviceAnnouncementReceiver(Mutex::new(device_rx)));
    
    // Subscribe to device announcements
    thread::spawn(move || {
        let mut mqttoptions = MqttOptions::new("device-subscriber", "127.0.0.1", 1883);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        let (client, mut connection) = Client::new(mqttoptions, 10);
        client.subscribe("devices/announce", QoS::AtMostOnce).unwrap();
        
        for notification in connection.iter() {
            if let Ok(Event::Incoming(Incoming::Publish(p))) = notification {
                if let Ok(s) = String::from_utf8(p.payload.to_vec()) {
                    let _ = device_tx.send(s);
                }
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
