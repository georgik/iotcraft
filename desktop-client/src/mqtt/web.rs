use bevy::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Mutex;
use std::sync::mpsc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{MessageEvent, WebSocket};

use super::mqtt_types::*;
use crate::config::MqttConfig;
use crate::devices::DeviceAnnouncementReceiver;
use crate::profile::PlayerProfile;

/// Web MQTT implementation using WebSockets for WASM
pub struct WebMqttPlugin;

impl Plugin for WebMqttPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TemperatureResource::default())
            .insert_resource(WebMqttConnection::default())
            .add_systems(Startup, spawn_web_mqtt_subscriber)
            .add_systems(Update, update_temperature);
    }
}

#[derive(Resource, Default)]
pub struct WebMqttConnection {
    pub websocket: Option<WebSocket>,
    pub connected: bool,
}

/// Simple MQTT-like message format for WebSocket communication
#[derive(serde::Serialize, serde::Deserialize)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: String,
    pub qos: u8,
}

/// Web MQTT subscriber using WebSockets (WASM only)
pub fn spawn_web_mqtt_subscriber(
    mut commands: Commands,
    mqtt_config: Res<MqttConfig>,
    profile: Res<PlayerProfile>,
) {
    // Set up panic hook for better error reporting in web console
    console_error_panic_hook::set_once();

    let (temp_tx, temp_rx) = mpsc::channel::<f32>();
    let (device_tx, device_rx) = mpsc::channel::<String>();

    commands.insert_resource(TemperatureReceiver(Mutex::new(temp_rx)));
    commands.insert_resource(DeviceAnnouncementReceiver(Mutex::new(device_rx)));

    let client_id = format!("web-{}", profile.player_id);

    // Create WebSocket URL - convert TCP MQTT address to WebSocket
    // Assume MQTT broker has WebSocket support on port 8080 or similar
    let ws_url = if mqtt_config.port == 1883 {
        format!("ws://{}:8083/mqtt", mqtt_config.host) // Common WebSocket MQTT port
    } else {
        format!("ws://{}:{}/mqtt", mqtt_config.host, mqtt_config.port + 1000) // Try port + 1000
    };

    info!(
        "MQTT Web: Attempting to connect to WebSocket MQTT at {}",
        ws_url
    );

    // Create WebSocket connection
    let websocket = WebSocket::new(&ws_url).unwrap_or_else(|_| {
        error!("Failed to create WebSocket connection to {}", ws_url);
        panic!("WebSocket creation failed");
    });

    websocket.set_binary_type(web_sys::BinaryType::Arraybuffer);

    // Wrap channels in Rc<RefCell<>> for sharing between closures
    let temp_tx = Rc::new(RefCell::new(temp_tx));
    let device_tx = Rc::new(RefCell::new(device_tx));

    // Clone references for closures
    let temp_tx_clone = temp_tx.clone();
    let device_tx_clone = device_tx.clone();

    // Set up message handler
    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
        if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
            let message_str = txt.as_string().unwrap_or_default();

            // Try to parse as MQTT-like message
            if let Ok(mqtt_msg) = serde_json::from_str::<MqttMessage>(&message_str) {
                info!("MQTT Web: Received message on topic: {}", mqtt_msg.topic);

                match mqtt_msg.topic.as_str() {
                    "home/sensor/temperature" => {
                        if let Ok(temp) = mqtt_msg.payload.parse::<f32>() {
                            if let Ok(tx) = temp_tx_clone.try_borrow() {
                                let _ = tx.send(temp);
                            }
                        }
                    }
                    "devices/announce" => {
                        if let Ok(tx) = device_tx_clone.try_borrow() {
                            let _ = tx.send(mqtt_msg.payload);
                        }
                    }
                    _ => {
                        info!("MQTT Web: Unhandled topic: {}", mqtt_msg.topic);
                    }
                }
            } else {
                // Handle raw messages (fallback)
                warn!("MQTT Web: Received non-JSON message: {}", message_str);
            }
        }
    });

    websocket.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();

    // Set up connection open handler
    let client_id_clone = client_id.clone();
    let websocket_clone = websocket.clone();
    let onopen_callback = Closure::<dyn FnMut()>::new(move || {
        info!("MQTT Web: WebSocket connection opened");

        // Subscribe to topics after connection opens
        let subscribe_temp = MqttMessage {
            topic: "home/sensor/temperature".to_string(),
            payload: "subscribe".to_string(),
            qos: 0,
        };

        let subscribe_devices = MqttMessage {
            topic: "devices/announce".to_string(),
            payload: "subscribe".to_string(),
            qos: 0,
        };

        if let Ok(temp_msg) = serde_json::to_string(&subscribe_temp) {
            let _ = websocket_clone.send_with_str(&temp_msg);
        }

        if let Ok(device_msg) = serde_json::to_string(&subscribe_devices) {
            let _ = websocket_clone.send_with_str(&device_msg);
        }

        info!(
            "MQTT Web: Subscribed to temperature and device topics for client {}",
            client_id_clone
        );
    });

    websocket.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    // Set up error handler
    let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e| {
        error!("MQTT Web: WebSocket error occurred: {:?}", e);
    });

    websocket.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    // Set up close handler
    let onclose_callback = Closure::<dyn FnMut(_)>::new(move |_| {
        warn!("MQTT Web: WebSocket connection closed");
    });

    websocket.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
    onclose_callback.forget();
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
