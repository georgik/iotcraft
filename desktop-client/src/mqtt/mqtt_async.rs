use bevy::prelude::*;
use rumqttc::{AsyncClient, Event, MqttOptions, QoS};
use std::sync::Arc;
use tokio::sync::{Mutex as TokioMutex, broadcast, mpsc};
use tokio::time::Duration;

use super::mqtt_types::*;
use crate::config::MqttConfig;
use crate::profile::PlayerProfile;

/// Async MQTT plugin that uses tokio for non-blocking operations
pub struct AsyncMqttPlugin;

impl Plugin for AsyncMqttPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TemperatureResource::default())
            .insert_resource(AsyncMqttState::default())
            .add_systems(Startup, setup_async_mqtt)
            .add_systems(Update, (update_temperature_async, process_mqtt_commands));
    }
}

/// Resource to track async MQTT state
#[derive(Resource, Default)]
pub struct AsyncMqttState {
    pub connected: bool,
    pub last_error: Option<String>,
}

/// Commands that can be sent to the async MQTT task
#[derive(Debug, Clone)]
pub enum MqttCommand {
    Publish {
        topic: String,
        payload: String,
        retain: bool,
    },
    Subscribe {
        topic: String,
    },
    Unsubscribe {
        topic: String,
    },
}

/// Channel for sending MQTT commands from Bevy systems to async task
#[derive(Resource)]
pub struct MqttCommandSender(pub mpsc::UnboundedSender<MqttCommand>);

/// Channel for receiving temperature updates from async task
#[derive(Resource)]
pub struct AsyncTemperatureReceiver(pub Arc<TokioMutex<broadcast::Receiver<f32>>>);

/// Channel for receiving device announcements from async task
#[derive(Resource)]
pub struct AsyncDeviceReceiver(pub Arc<TokioMutex<broadcast::Receiver<String>>>);

/// Async task handle for MQTT operations
#[derive(Resource)]
pub struct MqttTaskHandle(pub tokio::task::JoinHandle<()>);

/// Setup the async MQTT system
fn setup_async_mqtt(
    mut commands: Commands,
    mqtt_config: Res<MqttConfig>,
    profile: Res<PlayerProfile>,
) {
    let mqtt_host = mqtt_config.host.clone();
    let mqtt_port = mqtt_config.port;
    let client_id = format!("desktop-{}-async", profile.player_id);

    // Create channels for communication with async task
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<MqttCommand>();
    let (temp_tx, temp_rx) = broadcast::channel::<f32>(100);
    let (device_tx, device_rx) = broadcast::channel::<String>(100);

    // Spawn the async MQTT task
    let task_handle = tokio::spawn(async move {
        info!(
            "Async MQTT: Starting client on {}:{} with ID {}",
            mqtt_host, mqtt_port, client_id
        );

        // Set up MQTT client with optimal settings
        let mut mqttoptions = MqttOptions::new(&client_id, &mqtt_host, mqtt_port);
        mqttoptions.set_keep_alive(Duration::from_secs(30)); // Longer keep-alive for stability
        // Note: rumqttc doesn't have set_connection_timeout, using defaults

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 100);
        info!("Async MQTT: Client created successfully");

        // Subscribe to default topics
        if let Err(e) = client
            .subscribe("home/sensor/temperature", QoS::AtMostOnce)
            .await
        {
            error!("Async MQTT: Failed to subscribe to temperature: {}", e);
        } else {
            info!("Async MQTT: Subscribed to temperature topic");
        }

        if let Err(e) = client.subscribe("devices/announce", QoS::AtMostOnce).await {
            error!(
                "Async MQTT: Failed to subscribe to device announcements: {}",
                e
            );
        } else {
            info!("Async MQTT: Subscribed to device announcements");
        }

        // Main event loop
        loop {
            tokio::select! {
                // Handle incoming MQTT events
                notification = eventloop.poll() => {
                    match notification {
                        Ok(Event::Incoming(rumqttc::Packet::Publish(publish))) => {
                            match publish.topic.as_str() {
                                "home/sensor/temperature" => {
                                    if let Ok(payload_str) = String::from_utf8(publish.payload.to_vec()) {
                                        if let Ok(temp_value) = payload_str.parse::<f32>() {
                                            let _ = temp_tx.send(temp_value);
                                            trace!("Async MQTT: Temperature update: {}", temp_value);
                                        }
                                    }
                                }
                                "devices/announce" => {
                                    if let Ok(payload_str) = String::from_utf8(publish.payload.to_vec()) {
                                        let _ = device_tx.send(payload_str.clone());
                                        trace!("Async MQTT: Device announcement: {}", payload_str);
                                    }
                                }
                                topic => {
                                    trace!("Async MQTT: Received message on topic: {}", topic);
                                }
                            }
                        }
                        Ok(Event::Incoming(rumqttc::Packet::ConnAck(_))) => {
                            info!("Async MQTT: Connected to broker");
                        }
                        Err(e) => {
                            error!("Async MQTT: Connection error: {}", e);
                            tokio::time::sleep(Duration::from_secs(5)).await; // Reconnect delay
                        }
                        _ => {} // Other events
                    }
                }

                // Handle commands from Bevy systems
                Some(command) = cmd_rx.recv() => {
                    match command {
                        MqttCommand::Publish { topic, payload, retain } => {
                            if let Err(e) = client.publish(&topic, QoS::AtMostOnce, retain, payload.as_bytes()).await {
                                error!("Async MQTT: Failed to publish to {}: {}", topic, e);
                            } else {
                                trace!("Async MQTT: Published to topic: {}", topic);
                            }
                        }
                        MqttCommand::Subscribe { topic } => {
                            if let Err(e) = client.subscribe(&topic, QoS::AtMostOnce).await {
                                error!("Async MQTT: Failed to subscribe to {}: {}", topic, e);
                            } else {
                                info!("Async MQTT: Subscribed to topic: {}", topic);
                            }
                        }
                        MqttCommand::Unsubscribe { topic } => {
                            if let Err(e) = client.unsubscribe(&topic).await {
                                error!("Async MQTT: Failed to unsubscribe from {}: {}", topic, e);
                            } else {
                                info!("Async MQTT: Unsubscribed from topic: {}", topic);
                            }
                        }
                    }
                }
            }
        }
    });

    // Insert resources for Bevy systems to use
    commands.insert_resource(MqttCommandSender(cmd_tx));
    commands.insert_resource(AsyncTemperatureReceiver(Arc::new(TokioMutex::new(temp_rx))));
    commands.insert_resource(AsyncDeviceReceiver(Arc::new(TokioMutex::new(device_rx))));
    commands.insert_resource(MqttTaskHandle(task_handle));

    info!("Async MQTT system initialized");
}

/// Non-blocking system to update temperature from async channel
fn update_temperature_async(
    mut temp_resource: ResMut<TemperatureResource>,
    temp_receiver: Res<AsyncTemperatureReceiver>,
) {
    // Try to get the latest temperature without blocking
    if let Ok(mut rx) = temp_receiver.0.try_lock() {
        while let Ok(temp) = rx.try_recv() {
            temp_resource.value = Some(temp);
        }
    }
}

/// Non-blocking system to process MQTT commands from Bevy
fn process_mqtt_commands(
    mut mqtt_state: ResMut<AsyncMqttState>,
    // Add any command processing logic here if needed
) {
    // This system can be used to update MQTT state or handle connection status
    // For now, just ensure the resource exists and can be updated
    mqtt_state.connected = true; // This would be updated based on actual connection status
}

/// Helper function to publish MQTT message asynchronously
pub fn publish_mqtt_async(
    mqtt_sender: &Res<MqttCommandSender>,
    topic: &str,
    payload: &str,
    retain: bool,
) -> Result<(), &'static str> {
    mqtt_sender
        .0
        .send(MqttCommand::Publish {
            topic: topic.to_string(),
            payload: payload.to_string(),
            retain,
        })
        .map_err(|_| "Failed to send MQTT publish command")
}

/// Helper function to subscribe to MQTT topic asynchronously  
pub fn subscribe_mqtt_async(
    mqtt_sender: &Res<MqttCommandSender>,
    topic: &str,
) -> Result<(), &'static str> {
    mqtt_sender
        .0
        .send(MqttCommand::Subscribe {
            topic: topic.to_string(),
        })
        .map_err(|_| "Failed to send MQTT subscribe command")
}
