// Standalone MQTT World Discovery Test
// This test runs without depending on the main library to avoid compilation issues

use rumqttc::{Client, Event, Incoming, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

// Define the SharedWorldInfo struct directly (same as in desktop client)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SharedWorldInfo {
    pub world_id: String,
    pub world_name: String,
    pub description: String,
    pub host_player: String,
    pub host_name: String,
    pub created_at: String,
    pub last_updated: String,
    pub player_count: u32,
    pub max_players: u32,
    pub is_public: bool,
    pub version: String,
}

/// Generate a unique MQTT client ID to avoid conflicts
fn generate_unique_client_id(prefix: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let pid = std::process::id();
    let random = rand::random::<u16>();

    format!("{}-{}-{}-{}", prefix, timestamp, pid, random)
}

#[test]
fn test_shared_world_info_json_deserialization() {
    println!("🧪 Testing SharedWorldInfo JSON Deserialization");

    // Test with the exact JSON format that the desktop client should publish
    let test_json = r#"{
        "world_id": "test-world-123",
        "world_name": "My Test World",
        "description": "A test world for debugging",
        "host_player": "player-456",
        "host_name": "TestHost",
        "created_at": "2024-01-01T00:00:00Z",
        "last_updated": "2024-01-01T01:00:00Z",
        "player_count": 2,
        "max_players": 8,
        "is_public": true,
        "version": "1.0.0"
    }"#;

    println!("Testing JSON payload:");
    println!("{}", test_json);

    match serde_json::from_str::<SharedWorldInfo>(test_json) {
        Ok(world_info) => {
            println!("✅ Deserialization successful!");
            println!("   World ID: {}", world_info.world_id);
            println!("   World Name: {}", world_info.world_name);
            println!("   Host: {}", world_info.host_name);
            println!(
                "   Players: {}/{}",
                world_info.player_count, world_info.max_players
            );
            println!("   Public: {}", world_info.is_public);

            // Verify all fields are correctly parsed
            assert_eq!(world_info.world_id, "test-world-123");
            assert_eq!(world_info.world_name, "My Test World");
            assert_eq!(world_info.host_player, "player-456");
            assert_eq!(world_info.host_name, "TestHost");
            assert_eq!(world_info.player_count, 2);
            assert_eq!(world_info.max_players, 8);
            assert_eq!(world_info.is_public, true);
            assert_eq!(world_info.version, "1.0.0");
        }
        Err(e) => {
            println!("❌ Deserialization failed: {}", e);
            panic!("JSON deserialization should succeed");
        }
    }

    println!("✅ SharedWorldInfo deserialization test passed!");
}

// Test structure for MQTT results
#[derive(Debug, Clone, Default)]
struct MqttTestResults {
    publisher_connected: bool,
    discovery_connected: bool,
    world_info_published: bool,
    retained_message_received: bool,
    json_parsing_successful: bool,
    world_cache_updated: bool,
    received_payload: Option<String>,
    parse_error: Option<String>,
}

#[tokio::test]
async fn test_mqtt_world_discovery_standalone() {
    println!("🧪 Standalone MQTT World Discovery Test");
    println!("========================================");
    println!("Testing MQTT world info publishing and discovery without library dependencies");

    let results = Arc::new(Mutex::new(MqttTestResults::default()));

    // Test configuration
    let mqtt_host = "localhost";
    let mqtt_port = 1883;
    let test_world_id = "standalone-test-world";

    // Test world cache
    let world_cache: Arc<Mutex<HashMap<String, SharedWorldInfo>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Setup publisher
    println!("\n📤 Setting up publisher...");
    let publisher_client_id = generate_unique_client_id("standalone-publisher");
    let mut publisher_opts = MqttOptions::new(&publisher_client_id, mqtt_host, mqtt_port);
    publisher_opts.set_keep_alive(Duration::from_secs(30));
    publisher_opts.set_clean_session(false);
    publisher_opts.set_max_packet_size(1048576, 1048576);

    let (publisher_client, mut publisher_conn) = Client::new(publisher_opts, 10);
    println!("   Publisher ID: {}", publisher_client_id);

    // Setup discovery client
    println!("\n🔍 Setting up discovery client...");
    let discovery_client_id = generate_unique_client_id("standalone-discovery");
    let mut discovery_opts = MqttOptions::new(&discovery_client_id, mqtt_host, mqtt_port);
    discovery_opts.set_keep_alive(Duration::from_secs(30));
    discovery_opts.set_clean_session(false);
    discovery_opts.set_max_packet_size(1048576, 1048576);

    let (discovery_client, mut discovery_conn) = Client::new(discovery_opts, 10);
    println!("   Discovery ID: {}", discovery_client_id);

    // Start publisher task
    let publisher_results = results.clone();
    let publisher_task = tokio::spawn(async move {
        for event in publisher_conn.iter() {
            match event {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    println!("   ✅ Publisher connected");

                    if let Ok(mut res) = publisher_results.lock() {
                        res.publisher_connected = true;
                    }

                    // Wait then publish
                    sleep(Duration::from_millis(500)).await;

                    let world_info = SharedWorldInfo {
                        world_id: test_world_id.to_string(),
                        world_name: "Standalone Test World".to_string(),
                        description: "Testing world discovery".to_string(),
                        host_player: "test-player".to_string(),
                        host_name: "TestHost".to_string(),
                        created_at: "2024-01-01T00:00:00Z".to_string(),
                        last_updated: "2024-01-01T00:00:00Z".to_string(),
                        player_count: 1,
                        max_players: 4,
                        is_public: true,
                        version: "1.0.0".to_string(),
                    };

                    let json_payload = serde_json::to_string(&world_info).unwrap();
                    let topic = format!("iotcraft/worlds/{}/info", test_world_id);

                    println!("   📤 Publishing to: {}", topic);
                    println!("   📄 Payload: {}", json_payload);

                    match publisher_client.publish(&topic, QoS::AtLeastOnce, true, json_payload) {
                        Ok(_) => {
                            println!("   ✅ Published with retain=true");
                            if let Ok(mut res) = publisher_results.lock() {
                                res.world_info_published = true;
                            }
                        }
                        Err(e) => println!("   ❌ Publish failed: {}", e),
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    println!("   ❌ Publisher error: {:?}", e);
                    break;
                }
            }
        }
    });

    // Start discovery task
    let discovery_results = results.clone();
    let discovery_cache = world_cache.clone();
    let discovery_task = tokio::spawn(async move {
        let mut subscribed = false;

        for event in discovery_conn.iter() {
            match event {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    println!("   ✅ Discovery client connected");

                    if let Ok(mut res) = discovery_results.lock() {
                        res.discovery_connected = true;
                    }

                    println!("   🔔 Subscribing to iotcraft/worlds/+/info");
                    match discovery_client.subscribe("iotcraft/worlds/+/info", QoS::AtLeastOnce) {
                        Ok(_) => {
                            println!("   ✅ Subscribed successfully");
                            subscribed = true;
                        }
                        Err(e) => println!("   ❌ Subscribe failed: {}", e),
                    }
                }
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    if subscribed {
                        println!("\n   📨 RECEIVED MESSAGE:");
                        println!("      Topic: {}", p.topic);
                        println!("      Retained: {}", p.retain);
                        println!("      Payload size: {} bytes", p.payload.len());

                        // Update retained message flag
                        if let Ok(mut res) = discovery_results.lock() {
                            res.retained_message_received = p.retain;
                        }

                        // Parse topic
                        let topic_parts: Vec<&str> = p.topic.split('/').collect();
                        if topic_parts.len() >= 4 && topic_parts[3] == "info" {
                            let world_id = topic_parts[2];
                            println!("      World ID from topic: {}", world_id);

                            if !p.payload.is_empty() {
                                match String::from_utf8(p.payload.to_vec()) {
                                    Ok(payload_str) => {
                                        println!("      Payload: {}", payload_str);

                                        if let Ok(mut res) = discovery_results.lock() {
                                            res.received_payload = Some(payload_str.clone());
                                        }

                                        // Try to parse JSON
                                        match serde_json::from_str::<SharedWorldInfo>(&payload_str)
                                        {
                                            Ok(world_info) => {
                                                println!("      ✅ JSON parsing successful!");
                                                println!(
                                                    "         Name: {}",
                                                    world_info.world_name
                                                );
                                                println!("         Host: {}", world_info.host_name);

                                                // Update cache
                                                if let Ok(mut cache) = discovery_cache.lock() {
                                                    cache.insert(world_id.to_string(), world_info);
                                                    println!(
                                                        "      ✅ World cache updated ({} worlds)",
                                                        cache.len()
                                                    );
                                                }

                                                if let Ok(mut res) = discovery_results.lock() {
                                                    res.json_parsing_successful = true;
                                                    res.world_cache_updated = true;
                                                }
                                            }
                                            Err(e) => {
                                                println!("      ❌ JSON parsing failed: {}", e);
                                                if let Ok(mut res) = discovery_results.lock() {
                                                    res.parse_error = Some(e.to_string());
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("      ❌ UTF-8 decode failed: {}", e);
                                        if let Ok(mut res) = discovery_results.lock() {
                                            res.parse_error = Some(format!("UTF-8 error: {}", e));
                                        }
                                    }
                                }
                            } else {
                                println!("      ℹ️ Empty payload (world unpublished)");
                            }
                        }
                    }
                }
                Ok(Event::Incoming(Incoming::SubAck(_))) => {
                    println!("   ✅ Subscription acknowledged");
                }
                Ok(_) => {}
                Err(e) => {
                    println!("   ❌ Discovery error: {:?}", e);
                    break;
                }
            }
        }
    });

    // Wait for test to complete
    println!("\n⏱️ Running test for 5 seconds...");
    sleep(Duration::from_secs(5)).await;

    // Analyze results
    println!("\n📊 Test Results:");
    println!("================");

    let final_results = results.lock().unwrap().clone();
    let final_cache = world_cache.lock().unwrap().clone();

    println!("Publisher:");
    println!("   Connected: {}", final_results.publisher_connected);
    println!("   Published: {}", final_results.world_info_published);

    println!("Discovery:");
    println!("   Connected: {}", final_results.discovery_connected);
    println!(
        "   Retained Msg Received: {}",
        final_results.retained_message_received
    );
    println!(
        "   JSON Parsing Success: {}",
        final_results.json_parsing_successful
    );
    println!("   Cache Updated: {}", final_results.world_cache_updated);

    if let Some(payload) = &final_results.received_payload {
        println!("   Received Payload: {}", payload);
    }

    if let Some(error) = &final_results.parse_error {
        println!("   Parse Error: {}", error);
    }

    println!("World Cache: {} worlds", final_cache.len());
    for (id, info) in &final_cache {
        println!("   - {} ({})", info.world_name, id);
    }

    // Clean up
    publisher_task.abort();
    discovery_task.abort();

    // Final verdict
    let success = final_results.publisher_connected
        && final_results.discovery_connected
        && final_results.world_info_published
        && final_results.retained_message_received
        && final_results.json_parsing_successful
        && final_results.world_cache_updated
        && final_cache.len() > 0;

    if success {
        println!("\n✅ SUCCESS: MQTT World Discovery is working!");
    } else {
        println!("\n❌ FAILURE: Issues detected in MQTT World Discovery");
        if !final_results.publisher_connected {
            println!("   - Publisher connection failed");
        }
        if !final_results.discovery_connected {
            println!("   - Discovery connection failed");
        }
        if !final_results.world_info_published {
            println!("   - World info publishing failed");
        }
        if !final_results.retained_message_received {
            println!("   - Retained messages not received (broker config issue?)");
        }
        if !final_results.json_parsing_successful {
            println!("   - JSON parsing failed (payload format issue?)");
        }
        if !final_results.world_cache_updated {
            println!("   - World cache not updated");
        }
    }

    // Only assert success if we're not just testing connectivity
    if final_results.publisher_connected && final_results.discovery_connected {
        assert!(
            success,
            "MQTT World Discovery test failed - see details above"
        );
    } else {
        println!(
            "\n⚠️  SKIPPING assertion due to connectivity issues (is MQTT broker running on localhost:1883?)"
        );
    }
}
