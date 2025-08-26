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
use crate::multiplayer::mqtt_utils::generate_unique_client_id;
use crate::profile::PlayerProfile;

// Import multiplayer types
use serde::{Deserialize, Serialize};

/// Multiplayer pose message format (compatible with desktop)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PoseMessage {
    pub player_id: String,
    pub player_name: String,
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub ts: u64,
}

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
    /// Encode variable length integer for MQTT protocol
    fn encode_remaining_length(length: usize) -> Vec<u8> {
        let mut encoded = Vec::new();
        let mut x = length;

        loop {
            let mut encoded_byte = (x % 128) as u8;
            x /= 128;
            if x > 0 {
                encoded_byte |= 0x80;
            }
            encoded.push(encoded_byte);
            if x == 0 {
                break;
            }
        }

        encoded
    }

    /// Decode variable length integer from MQTT packet
    fn decode_remaining_length(data: &[u8], start_offset: usize) -> Option<(usize, usize)> {
        let mut value = 0usize;
        let mut multiplier = 1;
        let mut offset = start_offset;

        loop {
            if offset >= data.len() {
                return None;
            }

            let encoded_byte = data[offset];
            value += ((encoded_byte & 0x7F) as usize) * multiplier;

            if multiplier > 128 * 128 * 128 {
                return None; // Malformed remaining length
            }

            offset += 1;

            if (encoded_byte & 0x80) == 0 {
                break;
            }

            multiplier *= 128;
        }

        Some((value, offset - start_offset))
    }

    /// Parse MQTT SUBACK packet to check subscription acknowledgment
    fn parse_suback_packet(data: &[u8]) -> Option<(u16, Vec<u8>)> {
        if data.len() < 2 || (data[0] & 0xF0) != 0x90 {
            return None; // Not a SUBACK packet
        }

        let (remaining_length, length_bytes) = Self::decode_remaining_length(data, 1)?;
        let header_end = 1 + length_bytes;

        if data.len() < header_end + remaining_length {
            return None; // Incomplete packet
        }

        // Extract packet identifier (2 bytes)
        let packet_id = ((data[header_end] as u16) << 8) | (data[header_end + 1] as u16);

        // Extract return codes (remaining bytes)
        let return_codes = data[header_end + 2..header_end + remaining_length].to_vec();

        Some((packet_id, return_codes))
    }

    /// Parse MQTT CONNACK packet to check connection acknowledgment
    fn parse_connack_packet(data: &[u8]) -> Option<u8> {
        if data.len() < 2 || (data[0] & 0xF0) != 0x20 {
            return None; // Not a CONNACK packet
        }

        let (remaining_length, length_bytes) = Self::decode_remaining_length(data, 1)?;
        let header_end = 1 + length_bytes;

        if data.len() < header_end + remaining_length || remaining_length < 2 {
            return None; // Incomplete packet or invalid length
        }

        // Return code is in the second byte of variable header
        Some(data[header_end + 1])
    }

    /// Create MQTT CONNECT packet
    fn connect_packet(client_id: &str) -> Vec<u8> {
        let mut packet = Vec::new();

        // Fixed header: CONNECT (0x10)
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

        // Add proper variable length encoding for remaining length
        let remaining_length_bytes = Self::encode_remaining_length(variable_header.len());
        packet.extend_from_slice(&remaining_length_bytes);

        packet.extend_from_slice(&variable_header);
        packet
    }

    /// Create MQTT SUBSCRIBE packet
    fn subscribe_packet(topic: &str, packet_id: u16) -> Vec<u8> {
        let mut packet = Vec::new();

        // Fixed header: SUBSCRIBE (0x82)
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

        // Add proper variable length encoding for remaining length
        let remaining_length_bytes = Self::encode_remaining_length(variable_header.len());
        packet.extend_from_slice(&remaining_length_bytes);

        packet.extend_from_slice(&variable_header);
        packet
    }

    /// Create MQTT PUBLISH packet
    fn publish_packet(topic: &str, payload: &[u8]) -> Vec<u8> {
        let mut packet = Vec::new();

        // Fixed header: PUBLISH (0x30)
        packet.push(0x30);

        let mut variable_header = Vec::new();

        // Topic name
        let topic_bytes = topic.as_bytes();
        variable_header.extend_from_slice(&[
            (topic_bytes.len() >> 8) as u8,
            (topic_bytes.len() & 0xFF) as u8,
        ]);
        variable_header.extend_from_slice(topic_bytes);

        // Payload
        variable_header.extend_from_slice(payload);

        // Add proper variable length encoding for remaining length
        let remaining_length_bytes = Self::encode_remaining_length(variable_header.len());
        packet.extend_from_slice(&remaining_length_bytes);

        packet.extend_from_slice(&variable_header);
        packet
    }

    /// Parse MQTT PUBLISH packet
    fn parse_publish_packet(data: &[u8]) -> Option<(String, Vec<u8>)> {
        if data.len() < 2 || (data[0] & 0xF0) != 0x30 {
            return None; // Not a PUBLISH packet
        }

        let (remaining_length, length_bytes) = Self::decode_remaining_length(data, 1)?;
        let header_end = 1 + length_bytes;

        if data.len() < header_end + remaining_length {
            return None; // Incomplete packet
        }

        let mut offset = header_end;

        // Topic length (2 bytes)
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

        // Payload (rest of the packet within remaining length)
        let payload_end = header_end + remaining_length;
        if offset > payload_end {
            return None;
        }
        let payload = data[offset..payload_end].to_vec();

        Some((topic, payload))
    }
}

/// Simplified device announcement receiver for web WASM
#[derive(Resource)]
pub struct DeviceAnnouncementReceiver(pub Mutex<mpsc::Receiver<String>>);

/// Resource for receiving multiplayer pose messages
#[derive(Resource)]
pub struct PoseReceiver(pub Mutex<mpsc::Receiver<PoseMessage>>);

/// Resource for sending multiplayer pose messages  
#[derive(Resource)]
pub struct PoseSender(pub Mutex<mpsc::Sender<PoseMessage>>);

/// Global WebSocket reference for publishing messages - Not used as Resource for thread safety
pub struct WebSocketSender(pub Rc<RefCell<Option<WebSocket>>>);

/// Web MQTT subscriber with multiplayer support using WebSocket with MQTT protocol (WASM only)
pub fn spawn_web_mqtt_subscriber(
    mut commands: Commands,
    mqtt_config: Res<MqttConfig>,
    profile: Res<PlayerProfile>,
) {
    // Set up panic hook for better error reporting in web console
    console_error_panic_hook::set_once();

    // Add startup debug logging
    info!("spawn_web_mqtt_subscriber: Starting MQTT WebSocket connection with multiplayer support");
    web_sys::console::log_1(
        &"spawn_web_mqtt_subscriber: Starting MQTT WebSocket connection with multiplayer support"
            .into(),
    );

    let (temp_tx, temp_rx) = mpsc::channel::<f32>();
    let (device_tx, device_rx) = mpsc::channel::<String>();
    let (pose_tx, pose_rx) = mpsc::channel::<PoseMessage>();
    let (outgoing_pose_tx, outgoing_pose_rx) = mpsc::channel::<PoseMessage>();

    commands.insert_resource(TemperatureReceiver(Mutex::new(temp_rx)));
    commands.insert_resource(DeviceAnnouncementReceiver(Mutex::new(device_rx)));
    commands.insert_resource(PoseReceiver(Mutex::new(pose_rx)));
    commands.insert_resource(PoseSender(Mutex::new(outgoing_pose_tx)));

    let client_id = generate_unique_client_id("web-mqtt-client");
    let world_id = "default"; // Use default world for web client
    let pose_subscribe_topic = format!("iotcraft/worlds/{}/players/+/pose", world_id);

    // Use the WebSocket port directly from MQTT config (should be 8083 for web)
    let ws_url = format!("ws://{}:{}/", mqtt_config.host, mqtt_config.port);

    info!("MQTT Web: Connecting to WebSocket MQTT at {}", ws_url);
    web_sys::console::log_1(&format!("Connecting to MQTT broker at {}", ws_url).into());

    // Create WebSocket connection with MQTT subprotocol
    let websocket = match WebSocket::new_with_str(&ws_url, "mqtt") {
        Ok(ws) => {
            web_sys::console::log_1(
                &"Successfully created WebSocket connection with MQTT subprotocol".into(),
            );
            ws
        }
        Err(e) => {
            error!("MQTT Web: Failed to create WebSocket: {:?}", e);
            web_sys::console::log_1(&format!("Failed to create WebSocket: {:?}", e).into());
            return;
        }
    };

    websocket.set_binary_type(BinaryType::Arraybuffer);

    // Connection and subscription state tracking
    let subscriptions_confirmed = Rc::new(RefCell::new(0u8)); // Track confirmed subscriptions (0-3)
    let pose_subscription_confirmed = Rc::new(RefCell::new(false)); // Track specifically pose subscription

    // Wrap channels in Rc<RefCell<>> for sharing between closures
    let temp_tx = Rc::new(RefCell::new(temp_tx));
    let device_tx = Rc::new(RefCell::new(device_tx));
    let pose_tx = Rc::new(RefCell::new(pose_tx));
    let outgoing_pose_rx = Rc::new(RefCell::new(outgoing_pose_rx));

    // Clone references for closures
    let temp_tx_clone = temp_tx.clone();
    let device_tx_clone = device_tx.clone();
    let pose_tx_clone = pose_tx.clone();
    let subscriptions_confirmed_clone = subscriptions_confirmed.clone();
    let pose_subscription_confirmed_clone = pose_subscription_confirmed.clone();

    // Message handler
    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
        if let Ok(array_buffer) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            let uint8_array = Uint8Array::new(&array_buffer);
            let data: Vec<u8> = uint8_array.to_vec();

            // First check for CONNACK packet
            if let Some(return_code) = SimpleMqttPackets::parse_connack_packet(&data) {
                if return_code == 0 {
                    info!("MQTT Web: Connection acknowledged successfully");
                } else {
                    error!(
                        "MQTT Web: Connection failed with return code: {}",
                        return_code
                    );
                }
                return;
            }

            // Then check for SUBACK packet
            if let Some((packet_id, return_codes)) = SimpleMqttPackets::parse_suback_packet(&data) {
                info!(
                    "MQTT Web: Subscription acknowledged for packet ID: {}, return codes: {:?}",
                    packet_id, return_codes
                );

                // Check if subscription was successful (return code 0x00 or 0x01 for QoS 0/1)
                if return_codes.iter().all(|&code| code <= 0x01) {
                    if let Ok(mut confirmed) = subscriptions_confirmed_clone.try_borrow_mut() {
                        *confirmed += 1;
                        info!("MQTT Web: Subscriptions confirmed: {}/3", *confirmed);

                        // Special handling for pose subscription (packet ID 3)
                        if packet_id == 3 {
                            if let Ok(mut pose_confirmed) =
                                pose_subscription_confirmed_clone.try_borrow_mut()
                            {
                                *pose_confirmed = true;
                            }
                            info!(
                                "🎉 MQTT Web: Pose subscription confirmed! Pose publishing enabled."
                            );
                            web_sys::console::log_1(
                                &"🎉 Pose subscription confirmed! Multiplayer enabled.".into(),
                            );
                        }

                        if *confirmed == 3 {
                            info!("🎉 MQTT Web: All subscriptions confirmed!");
                        }
                    }
                } else {
                    error!(
                        "MQTT Web: Subscription failed for packet ID: {}, return codes: {:?}",
                        packet_id, return_codes
                    );
                }
                return;
            }

            // Finally check for PUBLISH packets
            if let Some((topic, payload)) = SimpleMqttPackets::parse_publish_packet(&data) {
                info!("MQTT Web: Received message on topic: {}", topic);

                if topic.starts_with("iotcraft/worlds/") && topic.contains("/pose") {
                    // Handle multiplayer pose messages
                    if let Ok(pose_str) = String::from_utf8(payload) {
                        if let Ok(pose_msg) = serde_json::from_str::<PoseMessage>(&pose_str) {
                            let player_name = pose_msg.player_name.clone();
                            if let Ok(tx) = pose_tx_clone.try_borrow() {
                                let _ = tx.send(pose_msg);
                                info!("📡 Web: Received pose from player {}", player_name);
                            }
                        }
                    }
                } else {
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
        }
    });

    websocket.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();

    // Connection open handler
    let client_id_clone = client_id.clone();
    let websocket_clone = websocket.clone();
    let pose_subscribe_topic_clone = pose_subscribe_topic.clone();
    let outgoing_pose_rx_clone = outgoing_pose_rx.clone();
    let player_id_clone = profile.player_id.clone();
    let world_id_clone = world_id.to_string();

    let onopen_callback = Closure::<dyn FnMut()>::new(move || {
        info!("MQTT Web: WebSocket connection opened to MQTT broker");
        info!("MQTT Web: Client ID: {}", client_id_clone);

        // Send CONNECT packet
        let connect_packet = SimpleMqttPackets::connect_packet(&client_id_clone);
        info!(
            "MQTT Web: Sending CONNECT packet for client ID: {}, packet size: {} bytes",
            client_id_clone,
            connect_packet.len()
        );
        web_sys::console::log_1(&format!("MQTT CONNECT packet: {:?}", connect_packet).into());

        if let Err(e) = websocket_clone.send_with_u8_array(&connect_packet) {
            error!("MQTT Web: Failed to send CONNECT packet: {:?}", e);
            return;
        } else {
            info!("MQTT Web: CONNECT packet sent successfully");
        }

        // Send SUBSCRIBE packets after a short delay to allow CONNECT to process
        let websocket_clone2 = websocket_clone.clone();
        let pose_topic_clone2 = pose_subscribe_topic_clone.clone();
        let outgoing_rx_clone2 = outgoing_pose_rx_clone.clone();
        let player_id_clone2 = player_id_clone.clone();
        let world_id_clone2 = world_id_clone.clone();

        let subscriptions_confirmed_clone2 = subscriptions_confirmed.clone();
        let timeout_callback = Closure::<dyn FnMut()>::new(move || {
            // Strategy: Subscribe to working topics first, then publish pose to create the topic hierarchy

            // 1. Subscribe to temperature topic (this works)
            let sub_temp_packet = SimpleMqttPackets::subscribe_packet("home/sensor/temperature", 1);
            if let Err(e) = websocket_clone2.send_with_u8_array(&sub_temp_packet) {
                error!(
                    "MQTT Web: Failed to send temperature SUBSCRIBE packet: {:?}",
                    e
                );
            }

            // 2. Subscribe to device announcements topic (this works)
            let sub_device_packet = SimpleMqttPackets::subscribe_packet("devices/announce", 2);
            if let Err(e) = websocket_clone2.send_with_u8_array(&sub_device_packet) {
                error!("MQTT Web: Failed to send device SUBSCRIBE packet: {:?}", e);
            }

            // 3. FIRST: Publish an initial pose to CREATE the topic hierarchy for multiplayer
            let publish_topic = format!(
                "iotcraft/worlds/{}/players/{}/pose",
                world_id_clone2, player_id_clone2
            );

            let initial_pose = PoseMessage {
                player_id: player_id_clone2.clone(),
                player_name: "WebPlayer".to_string(),
                pos: [0.0, 2.0, 0.0], // Default spawn position
                yaw: 0.0,
                pitch: 0.0,
                ts: crate::mqtt::now_ts_web(),
            };

            if let Ok(pose_payload) = serde_json::to_string(&initial_pose) {
                let publish_packet =
                    SimpleMqttPackets::publish_packet(&publish_topic, pose_payload.as_bytes());
                if let Err(e) = websocket_clone2.send_with_u8_array(&publish_packet) {
                    error!("🚀 Web: Failed to publish initial pose: {:?}", e);
                } else {
                    info!(
                        "🚀 Web: Published initial pose to CREATE topic: {}",
                        publish_topic
                    );
                }
            }

            // Small delay to let broker process the publish
            let websocket_delay = websocket_clone2.clone();
            let pose_topic_delay = pose_topic_clone2.clone();
            let delay_callback = Closure::<dyn FnMut()>::new(move || {
                // 4. NOW subscribe to multiplayer poses topic (after topic exists)
                let sub_pose_packet = SimpleMqttPackets::subscribe_packet(&pose_topic_delay, 3);
                if let Err(e) = websocket_delay.send_with_u8_array(&sub_pose_packet) {
                    error!("MQTT Web: Failed to send pose SUBSCRIBE packet: {:?}", e);
                } else {
                    info!(
                        "🌐 Web: NOW subscribed to multiplayer poses: {}",
                        pose_topic_delay
                    );
                }
            });

            web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    delay_callback.as_ref().unchecked_ref(),
                    200, // 200ms delay to let publish create the topic
                )
                .unwrap();
            delay_callback.forget();

            info!("MQTT Web: Sent all SUBSCRIBE packets, waiting for confirmations...");

            // Set up delayed pose publishing - wait for subscriptions to be confirmed
            let websocket_pub = websocket_clone2.clone();
            let outgoing_rx_pub = outgoing_rx_clone2.clone();
            let player_id_pub = player_id_clone2.clone();
            let world_id_pub = world_id_clone2.clone();
            let subs_confirmed_pub = subscriptions_confirmed_clone2.clone();

            let publish_callback = Closure::<dyn FnMut()>::new(move || {
                // Check subscription status and log it
                let confirmed_count = if let Ok(confirmed) = subs_confirmed_pub.try_borrow() {
                    *confirmed
                } else {
                    0
                };

                // Only start publishing poses after pose subscription is confirmed (packet ID 3)
                // We need at least 2 confirmations (temperature + pose)
                if confirmed_count < 2 {
                    // Periodically log that we're waiting for subscriptions
                    static mut LOG_COUNTER: u32 = 0;
                    unsafe {
                        LOG_COUNTER += 1;
                        if LOG_COUNTER % 100 == 0 {
                            // Log every 5 seconds (50ms * 100)
                            info!(
                                "⏳ Web: Waiting for pose subscription to be confirmed ({}/3)",
                                confirmed_count
                            );
                        }
                    }
                    return; // Not all subscriptions confirmed yet
                }

                // Check for outgoing pose messages
                if let Ok(rx) = outgoing_rx_pub.try_borrow() {
                    while let Ok(pose_msg) = rx.try_recv() {
                        let topic = format!(
                            "iotcraft/worlds/{}/players/{}/pose",
                            world_id_pub, player_id_pub
                        );
                        if let Ok(payload) = serde_json::to_string(&pose_msg) {
                            let publish_packet =
                                SimpleMqttPackets::publish_packet(&topic, payload.as_bytes());
                            info!("📡 Web: Publishing pose to topic '{}': {}", topic, payload);
                            if let Err(e) = websocket_pub.send_with_u8_array(&publish_packet) {
                                error!("📡 Web: Failed to publish pose: {:?}", e);
                            } else {
                                info!(
                                    "📡 Web: Successfully sent pose packet for player {}",
                                    pose_msg.player_name
                                );
                            }
                        }
                    }
                }
            });

            let _publish_interval = web_sys::window()
                .unwrap()
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    publish_callback.as_ref().unchecked_ref(),
                    50, // Check every 50ms (reduced frequency while waiting for subscriptions)
                )
                .unwrap();

            publish_callback.forget();
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
        // Handle the case where message() might return undefined
        let error_msg = if let Ok(msg) = js_sys::Reflect::get(&e, &"message".into()) {
            if msg.is_undefined() {
                "Unknown WebSocket error (no message)".to_string()
            } else {
                msg.as_string()
                    .unwrap_or_else(|| "Unknown WebSocket error (invalid message)".to_string())
            }
        } else {
            "Unknown WebSocket error (no message property)".to_string()
        };
        error!("MQTT Web: WebSocket error: {}", error_msg);
        web_sys::console::log_1(&format!("MQTT WebSocket error: {}", error_msg).into());
    });

    websocket.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    // Close handler
    let onclose_callback = Closure::<dyn FnMut(_)>::new(move |_: web_sys::CloseEvent| {
        warn!("MQTT Web: WebSocket connection closed");
    });

    websocket.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
    onclose_callback.forget();

    info!("🌐 Web: MQTT multiplayer connection established");
    web_sys::console::log_1(&"🌐 Web: MQTT multiplayer connection established".into());
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
