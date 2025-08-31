#[cfg(not(target_arch = "wasm32"))]
use log::{error, info, warn};

#[cfg(not(target_arch = "wasm32"))]
use rumqttc::{Client, Event, Incoming, MqttOptions, QoS};
#[cfg(not(target_arch = "wasm32"))]
use std::process;
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Generate a unique MQTT client ID to avoid conflicts
#[cfg(not(target_arch = "wasm32"))]
fn generate_unique_client_id(prefix: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let pid = process::id();
    let random = rand::random::<u16>();

    format!("{}-{}-{}-{}", prefix, timestamp, pid, random)
}

/// Simple MQTT test utility to verify retained messages
///
/// This tool can:
/// 1. Connect to MQTT broker
/// 2. Subscribe to world info topics
/// 3. Display any retained messages that are received
///
/// Usage: cargo run --bin mqtt_test
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    env_logger::init();

    info!("Starting MQTT test utility...");

    // MQTT broker configuration (should match your config)
    let mqtt_host = "localhost"; // Change to your MQTT broker
    let mqtt_port = 1883;

    // Create MQTT options with unique client ID
    let client_id = generate_unique_client_id("iotcraft-test-client");
    info!("Using client ID: {}", client_id);
    let mut opts = MqttOptions::new(&client_id, mqtt_host, mqtt_port);
    opts.set_keep_alive(Duration::from_secs(30));
    opts.set_clean_session(true); // Start clean to better see retained messages
    opts.set_max_packet_size(1048576, 1048576);

    let (client, mut conn) = Client::new(opts, 10);

    info!(
        "Connecting to MQTT broker at {}:{}...",
        mqtt_host, mqtt_port
    );

    // Spawn a thread to handle MQTT events
    let client_clone = client.clone();
    thread::spawn(move || {
        // Give the main thread a moment to subscribe
        thread::sleep(Duration::from_secs(1));

        // Test: Publish a retained world info message
        let test_world_id = "test-world-12345";
        let test_info = serde_json::json!({
            "world_id": test_world_id,
            "world_name": "Test World",
            "host_name": "test-host",
            "player_count": 1,
            "max_players": 10,
            "is_public": true
        });

        let info_topic = format!("iotcraft/worlds/{}/info", test_world_id);
        let payload = test_info.to_string();

        info!("Publishing test world info to topic: {}", info_topic);
        info!("Payload: {}", payload);

        // Publish with retain flag set to true
        if let Err(e) = client_clone.publish(&info_topic, QoS::AtLeastOnce, true, payload) {
            error!("Failed to publish test message: {}", e);
        } else {
            info!("Successfully published retained test message");
        }

        // Wait a bit then clear the retained message
        thread::sleep(Duration::from_secs(5));
        info!("Clearing retained message...");
        if let Err(e) = client_clone.publish(&info_topic, QoS::AtLeastOnce, true, "") {
            error!("Failed to clear retained message: {}", e);
        } else {
            info!("Successfully cleared retained message");
        }
    });

    // Main event loop
    let mut connected = false;
    let mut message_count = 0;

    for event in conn.iter() {
        match event {
            Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                info!("Connected to MQTT broker successfully");
                connected = true;

                // Subscribe to world info topics
                if let Err(e) = client.subscribe("iotcraft/worlds/+/info", QoS::AtLeastOnce) {
                    error!("Failed to subscribe: {}", e);
                    break;
                } else {
                    info!("Subscribed to iotcraft/worlds/+/info");
                    info!("Waiting for retained messages...");
                }
            }
            Ok(Event::Incoming(Incoming::Publish(p))) => {
                message_count += 1;
                info!("📨 Received message #{}: ", message_count);
                info!("  Topic: {}", p.topic);
                info!("  Retained: {}", p.retain);
                info!("  QoS: {:?}", p.qos);
                info!("  Payload length: {} bytes", p.payload.len());

                // Try to decode payload as string
                match String::from_utf8(p.payload.to_vec()) {
                    Ok(payload_str) => {
                        if payload_str.is_empty() {
                            info!("  Payload: <empty> (world unpublished)");
                        } else {
                            info!("  Payload: {}", payload_str);

                            // Try to parse as JSON
                            match serde_json::from_str::<serde_json::Value>(&payload_str) {
                                Ok(json) => {
                                    if let Some(world_name) = json.get("world_name") {
                                        if let Some(host_name) = json.get("host_name") {
                                            info!("  -> World: {} by {}", world_name, host_name);
                                        }
                                    }
                                }
                                Err(_) => {
                                    info!("  -> Payload is not valid JSON");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        info!(
                            "  Payload: <binary data, {} bytes> (decode error: {})",
                            p.payload.len(),
                            e
                        );
                    }
                }
                info!("  ---");
            }
            Ok(Event::Incoming(incoming)) => {
                // Log other incoming events at debug level
                info!("MQTT Event: {:?}", incoming);
            }
            Ok(Event::Outgoing(outgoing)) => {
                // Log outgoing events at debug level
                info!("MQTT Outgoing: {:?}", outgoing);
            }
            Err(e) => {
                error!("MQTT connection error: {:?}", e);
                if connected {
                    warn!("Connection lost, attempting to reconnect...");
                    connected = false;
                }
                thread::sleep(Duration::from_secs(2));
            }
        }
    }

    info!("MQTT test utility finished");
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // MQTT test not available on WASM
}
