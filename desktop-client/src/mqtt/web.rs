use bevy::prelude::*;
use js_sys::Uint8Array;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Mutex;
use std::sync::mpsc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{BinaryType, ErrorEvent, MessageEvent, WebSocket};

use super::mqtt_types::*;
use crate::config::MqttConfig;
use crate::profile::PlayerProfile;

/// Web MQTT implementation using WebSocket with MQTT protocol for WASM
pub struct WebMqttPlugin;

impl Plugin for WebMqttPlugin {
    fn build(&self, app: &mut App) {
        // Add explicit logging for plugin initialization
        info!("WebMqttPlugin: Initializing MQTT plugin for web");
        web_sys::console::log_1(&"WebMqttPlugin: Initializing MQTT plugin for web".into());

        app.insert_resource(TemperatureResource::default())
            .add_systems(Startup, spawn_web_mqtt_subscriber)
            .add_systems(Update, update_temperature);

        info!("WebMqttPlugin: MQTT plugin setup complete");
        web_sys::console::log_1(&"WebMqttPlugin: MQTT plugin setup complete".into());
    }
}

/// Simple MQTT packet builder for WebSocket
struct SimpleMqttPackets;

impl SimpleMqttPackets {
    /// Create MQTT CONNECT packet
    fn connect_packet(client_id: &str) -> Vec<u8> {
        let mut packet = Vec::new();

        // Fixed header: CONNECT (0x10), remaining length will be calculated
        packet.push(0x10);

        let mut variable_header = Vec::new();

        // Protocol name "MQTT"
        variable_header.extend_from_slice(&[0x00, 0x04]); // Length
        variable_header.extend_from_slice(b"MQTT");

        // Protocol level (4 for MQTT 3.1.1)
        variable_header.push(0x04);

        // Connect flags (clean session = 1)
        variable_header.push(0x02);

        // Keep alive (60 seconds)
        variable_header.extend_from_slice(&[0x00, 0x3C]);

        // Payload: Client ID
        let client_id_bytes = client_id.as_bytes();
        variable_header.extend_from_slice(&[
            (client_id_bytes.len() >> 8) as u8,
            (client_id_bytes.len() & 0xFF) as u8,
        ]);
        variable_header.extend_from_slice(client_id_bytes);

        // Calculate remaining length
        let remaining_length = variable_header.len();
        packet.push(remaining_length as u8); // Simplified for small packets

        packet.extend_from_slice(&variable_header);
        packet
    }

    /// Create MQTT SUBSCRIBE packet
    fn subscribe_packet(topic: &str, packet_id: u16) -> Vec<u8> {
        let mut packet = Vec::new();

        // Fixed header: SUBSCRIBE (0x82), remaining length will be calculated
        packet.push(0x82);

        let mut variable_header = Vec::new();

        // Packet identifier
        variable_header.extend_from_slice(&[(packet_id >> 8) as u8, (packet_id & 0xFF) as u8]);

        // Payload: Topic filter
        let topic_bytes = topic.as_bytes();
        variable_header.extend_from_slice(&[
            (topic_bytes.len() >> 8) as u8,
            (topic_bytes.len() & 0xFF) as u8,
        ]);
        variable_header.extend_from_slice(topic_bytes);

        // QoS level (0)
        variable_header.push(0x00);

        // Calculate remaining length
        let remaining_length = variable_header.len();
        packet.push(remaining_length as u8); // Simplified for small packets

        packet.extend_from_slice(&variable_header);
        packet
    }

    /// Parse MQTT PUBLISH packet
    fn parse_publish_packet(data: &[u8]) -> Option<(String, Vec<u8>)> {
        if data.len() < 2 || (data[0] & 0xF0) != 0x30 {
            return None; // Not a PUBLISH packet
        }

        let remaining_length = data[1] as usize;
        if data.len() < 2 + remaining_length {
            return None; // Incomplete packet
        }

        let mut offset = 2;

        // Topic length
        if offset + 2 > data.len() {
            return None;
        }
        let topic_length = ((data[offset] as usize) << 8) | (data[offset + 1] as usize);
        offset += 2;

        // Topic name
        if offset + topic_length > data.len() {
            return None;
        }
        let topic = String::from_utf8_lossy(&data[offset..offset + topic_length]).to_string();
        offset += topic_length;

        // Payload
        let payload = data[offset..].to_vec();

        Some((topic, payload))
    }
}

/// Simplified device announcement receiver for web WASM
#[derive(Resource)]
pub struct DeviceAnnouncementReceiver(pub Mutex<mpsc::Receiver<String>>);

/// Web MQTT subscriber using WebSocket with MQTT protocol (WASM only)
pub fn spawn_web_mqtt_subscriber(
    mut commands: Commands,
    mqtt_config: Res<MqttConfig>,
    profile: Res<PlayerProfile>,
) {
    // Set up panic hook for better error reporting in web console
    console_error_panic_hook::set_once();

    // Add startup debug logging
    info!("spawn_web_mqtt_subscriber: Starting MQTT WebSocket connection");
    web_sys::console::log_1(
        &"spawn_web_mqtt_subscriber: Starting MQTT WebSocket connection".into(),
    );

    let (temp_tx, temp_rx) = mpsc::channel::<f32>();
    let (device_tx, device_rx) = mpsc::channel::<String>();

    commands.insert_resource(TemperatureReceiver(Mutex::new(temp_rx)));
    commands.insert_resource(DeviceAnnouncementReceiver(Mutex::new(device_rx)));

    let client_id = format!("web-{}", profile.player_id);

    // Determine WebSocket URL based on MQTT config
    let ws_port = if mqtt_config.port == 1883 {
        8083
    } else {
        mqtt_config.port + 1000
    };
    let ws_url = format!("ws://{}:{}/", mqtt_config.host, ws_port);

    info!("MQTT Web: Connecting to WebSocket MQTT at {}", ws_url);
    web_sys::console::log_1(&format!("Connecting to MQTT broker at {}", ws_url).into());

    // Create WebSocket connection with MQTT subprotocol
    let websocket = match WebSocket::new_with_str(&ws_url, "mqtt") {
        Ok(ws) => {
            web_sys::console::log_1(&"Successfully created WebSocket with MQTT subprotocol".into());
            ws
        }
        Err(err) => {
            web_sys::console::log_1(
                &format!(
                    "Failed to create WebSocket with MQTT subprotocol: {:?}",
                    err
                )
                .into(),
            );
            // Fallback to connection without subprotocol
            match WebSocket::new(&ws_url) {
                Ok(ws) => {
                    info!("MQTT Web: Connected without subprotocol");
                    web_sys::console::log_1(&"Connected without MQTT subprotocol".into());
                    ws
                }
                Err(e) => {
                    error!("MQTT Web: Failed to create WebSocket: {:?}", e);
                    web_sys::console::log_1(&format!("Failed to create WebSocket: {:?}", e).into());
                    return;
                }
            }
        }
    };

    websocket.set_binary_type(BinaryType::Arraybuffer);

    // Wrap channels in Rc<RefCell<>> for sharing between closures
    let temp_tx = Rc::new(RefCell::new(temp_tx));
    let device_tx = Rc::new(RefCell::new(device_tx));

    // Clone references for closures
    let temp_tx_clone = temp_tx.clone();
    let device_tx_clone = device_tx.clone();

    // Message handler
    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
        if let Ok(array_buffer) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            let uint8_array = Uint8Array::new(&array_buffer);
            let data: Vec<u8> = uint8_array.to_vec();

            if let Some((topic, payload)) = SimpleMqttPackets::parse_publish_packet(&data) {
                info!("MQTT Web: Received message on topic: {}", topic);

                match topic.as_str() {
                    "home/sensor/temperature" => {
                        if let Ok(temp_str) = String::from_utf8(payload) {
                            if let Ok(temp_val) = temp_str.parse::<f32>() {
                                if let Ok(tx) = temp_tx_clone.try_borrow() {
                                    let _ = tx.send(temp_val);
                                }
                            }
                        }
                    }
                    "devices/announce" => {
                        if let Ok(device_msg) = String::from_utf8(payload) {
                            if let Ok(tx) = device_tx_clone.try_borrow() {
                                let _ = tx.send(device_msg);
                            }
                        }
                    }
                    _ => {
                        info!("MQTT Web: Unhandled topic: {}", topic);
                    }
                }
            }
        }
    });

    websocket.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();

    // Connection open handler
    let client_id_clone = client_id.clone();
    let websocket_clone = websocket.clone();
    let onopen_callback = Closure::<dyn FnMut()>::new(move || {
        info!("MQTT Web: WebSocket connection opened to MQTT broker");
        info!("MQTT Web: Client ID: {}", client_id_clone);

        // Send CONNECT packet
        let connect_packet = SimpleMqttPackets::connect_packet(&client_id_clone);
        if let Err(e) = websocket_clone.send_with_u8_array(&connect_packet) {
            error!("MQTT Web: Failed to send CONNECT packet: {:?}", e);
            return;
        }

        // Send SUBSCRIBE packets after a short delay to allow CONNECT to process
        let websocket_clone2 = websocket_clone.clone();
        let timeout_callback = Closure::<dyn FnMut()>::new(move || {
            // Subscribe to temperature topic
            let sub_temp_packet = SimpleMqttPackets::subscribe_packet("home/sensor/temperature", 1);
            if let Err(e) = websocket_clone2.send_with_u8_array(&sub_temp_packet) {
                error!(
                    "MQTT Web: Failed to send temperature SUBSCRIBE packet: {:?}",
                    e
                );
            }

            // Subscribe to device announcements topic
            let sub_device_packet = SimpleMqttPackets::subscribe_packet("devices/announce", 2);
            if let Err(e) = websocket_clone2.send_with_u8_array(&sub_device_packet) {
                error!("MQTT Web: Failed to send device SUBSCRIBE packet: {:?}", e);
            }

            info!("MQTT Web: Subscribed to MQTT topics");
        });

        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                timeout_callback.as_ref().unchecked_ref(),
                100, // 100ms delay
            )
            .unwrap();
        timeout_callback.forget();
    });

    websocket.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    // Error handler
    let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
        error!("MQTT Web: WebSocket error: {:?}", e.message());
    });

    websocket.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    // Close handler
    let onclose_callback = Closure::<dyn FnMut(_)>::new(move |_: web_sys::CloseEvent| {
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
