use rumqttc::{Client, Event, Incoming, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

// Define the SharedWorldInfo struct directly for testing (same as in desktop client)
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

/// Generate a unique MQTT client ID to avoid conflicts (copied from desktop client)
fn generate_unique_client_id(prefix: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let pid = std::process::id();
    let random = rand::random::<u16>();

    format!("{}-{}-{}-{}", prefix, timestamp, pid, random)
}

/// Structure to track test results
#[derive(Debug, Clone)]
struct TestResults {
    publisher_connected: bool,
    discovery_connected: bool,
    world_info_published: bool,
    retained_message_received: bool,
    json_parsing_successful: bool,
    world_cache_updated: bool,
    received_payload: Option<String>,
    parse_error: Option<String>,
}

impl Default for TestResults {
    fn default() -> Self {
        Self {
            publisher_connected: false,
            discovery_connected: false,
            world_info_published: false,
            retained_message_received: false,
            json_parsing_successful: false,
            world_cache_updated: false,
            received_payload: None,
            parse_error: None,
        }
    }
}

#[tokio::test]
async fn test_mqtt_world_discovery_exact_replica() {
    println!("🧪 MQTT World Discovery Debug Test");
    println!("==================================");
    println!("This test replicates EXACTLY how the desktop client publishes and discovers worlds");

    let results = Arc::new(Mutex::new(TestResults::default()));

    // Test configuration - matches desktop client config
    let mqtt_host = "localhost";
    let mqtt_port = 1883;
    let test_world_id = "test-discovery-world-12345";

    // STEP 1: Create Publisher Client (replicates desktop world publishing)
    println!("\n📤 STEP 1: Setting up world publisher (replicates desktop client)");

    let publisher_client_id = generate_unique_client_id("iotcraft-world-publisher");
    let mut publisher_opts = MqttOptions::new(&publisher_client_id, mqtt_host, mqtt_port);
    publisher_opts.set_keep_alive(Duration::from_secs(30));
    publisher_opts.set_clean_session(false);
    publisher_opts.set_max_packet_size(1048576, 1048576);

    let (publisher_client, mut publisher_conn) = Client::new(publisher_opts, 10);
    println!("   Publisher client ID: {}", publisher_client_id);

    // STEP 2: Create Discovery Client (replicates world discovery)
    println!("\n🔍 STEP 2: Setting up world discovery client (replicates discovery service)");

    let discovery_client_id = generate_unique_client_id("iotcraft-world-discovery");
    let mut discovery_opts = MqttOptions::new(&discovery_client_id, mqtt_host, mqtt_port);
    discovery_opts.set_keep_alive(Duration::from_secs(30));
    discovery_opts.set_clean_session(false); // CRITICAL: Must be false to receive retained messages
    discovery_opts.set_max_packet_size(1048576, 1048576);

    let (discovery_client, mut discovery_conn) = Client::new(discovery_opts, 10);
    println!("   Discovery client ID: {}", discovery_client_id);

    let world_cache: Arc<Mutex<HashMap<String, SharedWorldInfo>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let world_cache_clone = world_cache.clone();
    let results_clone = results.clone();

    // STEP 3: Start Publisher Task
    println!("\n🚀 STEP 3: Starting publisher task");
    let publisher_results = results.clone();
    let publisher_task = tokio::spawn(async move {
        let mut connected = false;

        for event in publisher_conn.iter() {
            match event {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    println!("   ✅ Publisher connected to MQTT broker");
                    connected = true;

                    if let Ok(mut res) = publisher_results.lock() {
                        res.publisher_connected = true;
                    }

                    // Wait a moment, then publish the world info (exactly like desktop client)
                    tokio::time::sleep(Duration::from_millis(500)).await;

                    println!("   📤 Publishing world info (exactly like desktop client)...");

                    // Create SharedWorldInfo exactly like the desktop client does
                    let world_info = SharedWorldInfo {
                        world_id: test_world_id.to_string(),
                        world_name: "Debug Test World".to_string(),
                        description: "Test world for debugging MQTT discovery".to_string(),
                        host_player: "debug-player-123".to_string(),
                        host_name: "DebugHost".to_string(),
                        created_at: "2024-01-01T00:00:00Z".to_string(),
                        last_updated: "2024-01-01T00:00:00Z".to_string(),
                        player_count: 1,
                        max_players: 4,
                        is_public: true,
                        version: "1.0.0".to_string(),
                    };

                    // Serialize to JSON exactly like the desktop client
                    let world_info_json = match serde_json::to_string(&world_info) {
                        Ok(json) => json,
                        Err(e) => {
                            println!("   ❌ Failed to serialize world info: {}", e);
                            break;
                        }
                    };

                    let topic = format!("iotcraft/worlds/{}/info", test_world_id);
                    println!("   📍 Publishing to topic: {}", topic);
                    println!("   📄 Payload: {}", world_info_json);

                    // Publish with retain=true and QoS=AtLeastOnce (exactly like desktop client)
                    match publisher_client.publish(&topic, QoS::AtLeastOnce, true, world_info_json)
                    {
                        Ok(_) => {
                            println!("   ✅ World info published successfully with retain=true");
                            if let Ok(mut res) = publisher_results.lock() {
                                res.world_info_published = true;
                            }
                        }
                        Err(e) => {
                            println!("   ❌ Failed to publish world info: {}", e);
                            break;
                        }
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    println!("   ❌ Publisher connection error: {:?}", e);
                    break;
                }
            }
        }
    });

    // STEP 4: Start Discovery Task
    println!("\n🔍 STEP 4: Starting discovery task (replicates world_discovery.rs)");
    let discovery_results = results.clone();
    let discovery_task = tokio::spawn(async move {
        let mut connected = false;
        let mut subscribed = false;

        for event in discovery_conn.iter() {
            match event {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    println!("   ✅ Discovery client connected to MQTT broker");
                    connected = true;

                    if let Ok(mut res) = discovery_results.lock() {
                        res.discovery_connected = true;
                    }

                    // Subscribe exactly like the world discovery service does
                    println!("   🔔 Subscribing to world info topics...");
                    match discovery_client.subscribe("iotcraft/worlds/+/info", QoS::AtLeastOnce) {
                        Ok(_) => {
                            println!(
                                "   ✅ Subscribed to iotcraft/worlds/+/info with clean_session=false"
                            );
                            subscribed = true;
                        }
                        Err(e) => {
                            println!("   ❌ Failed to subscribe: {}", e);
                            break;
                        }
                    }
                }
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    if subscribed {
                        println!("\n   📨 DISCOVERY: Received MQTT message!");
                        println!("      Topic: {}", p.topic);
                        println!("      Retained: {}", p.retain);
                        println!("      QoS: {:?}", p.qos);
                        println!("      Payload length: {} bytes", p.payload.len());

                        if let Ok(mut res) = discovery_results.lock() {
                            res.retained_message_received = p.retain;
                        }

                        // Process exactly like handle_discovery_message in world_discovery.rs
                        let topic_parts: Vec<&str> = p.topic.split('/').collect();
                        if topic_parts.len() >= 4 && topic_parts[3] == "info" {
                            let world_id = topic_parts[2];
                            println!("      Processing world info for ID: {}", world_id);

                            if !p.payload.is_empty() {
                                // Parse exactly like the discovery service does
                                match String::from_utf8(p.payload.to_vec()) {
                                    Ok(payload_str) => {
                                        println!("      Decoded payload: {}", payload_str);

                                        if let Ok(mut res) = discovery_results.lock() {
                                            res.received_payload = Some(payload_str.clone());
                                        }

                                        // Try to parse as SharedWorldInfo exactly like the discovery service
                                        match serde_json::from_str::<SharedWorldInfo>(&payload_str)
                                        {
                                            Ok(world_info) => {
                                                println!("      ✅ JSON parsing successful!");
                                                println!(
                                                    "         World Name: {}",
                                                    world_info.world_name
                                                );
                                                println!(
                                                    "         Host Name: {}",
                                                    world_info.host_name
                                                );
                                                println!(
                                                    "         Player Count: {}",
                                                    world_info.player_count
                                                );

                                                // Update world cache exactly like the discovery service
                                                if let Ok(mut cache) = world_cache_clone.lock() {
                                                    cache.insert(
                                                        world_id.to_string(),
                                                        world_info.clone(),
                                                    );
                                                    println!(
                                                        "      ✅ World cache updated! Cache now has {} worlds",
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
                                        println!("      ❌ UTF-8 decoding failed: {}", e);
                                        if let Ok(mut res) = discovery_results.lock() {
                                            res.parse_error =
                                                Some(format!("UTF-8 decode error: {}", e));
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
                    println!("   ✅ Subscription acknowledged - waiting for retained messages...");
                }
                Ok(_) => {}
                Err(e) => {
                    println!("   ❌ Discovery connection error: {:?}", e);
                    break;
                }
            }
        }
    });

    // STEP 5: Wait for test completion
    println!("\n⏱️ STEP 5: Running test for 5 seconds...");
    sleep(Duration::from_secs(5)).await;

    // STEP 6: Analyze Results
    println!("\n📊 STEP 6: Test Results Analysis");
    println!("================================");

    let final_results = results.lock().unwrap().clone();
    let final_cache = world_cache.lock().unwrap().clone();

    println!("Publisher Results:");
    println!("   ✅ Connected: {}", final_results.publisher_connected);
    println!(
        "   📤 World Info Published: {}",
        final_results.world_info_published
    );

    println!("\nDiscovery Results:");
    println!("   ✅ Connected: {}", final_results.discovery_connected);
    println!(
        "   📨 Retained Message Received: {}",
        final_results.retained_message_received
    );
    println!(
        "   🔍 JSON Parsing Successful: {}",
        final_results.json_parsing_successful
    );
    println!(
        "   💾 World Cache Updated: {}",
        final_results.world_cache_updated
    );

    if let Some(payload) = &final_results.received_payload {
        println!("   📄 Received Payload: {}", payload);
    } else {
        println!("   📄 Received Payload: <none>");
    }

    if let Some(error) = &final_results.parse_error {
        println!("   ❌ Parse Error: {}", error);
    }

    println!("\nWorld Cache Status:");
    println!("   🌍 Cached Worlds: {}", final_cache.len());
    for (world_id, world_info) in &final_cache {
        println!("      - {} ({})", world_info.world_name, world_id);
    }

    // STEP 7: Determine Test Success
    println!("\n🏁 STEP 7: Final Verdict");
    println!("========================");

    let all_good = final_results.publisher_connected
        && final_results.discovery_connected
        && final_results.world_info_published
        && final_results.retained_message_received
        && final_results.json_parsing_successful
        && final_results.world_cache_updated
        && final_cache.len() > 0;

    if all_good {
        println!("✅ SUCCESS: MQTT World Discovery is working correctly!");
        println!("   The desktop client should be able to publish and discover worlds.");
    } else {
        println!("❌ FAILURE: There are issues with MQTT World Discovery:");

        if !final_results.publisher_connected {
            println!("   - Publisher failed to connect to MQTT broker");
        }
        if !final_results.discovery_connected {
            println!("   - Discovery client failed to connect to MQTT broker");
        }
        if !final_results.world_info_published {
            println!("   - World info failed to publish");
        }
        if !final_results.retained_message_received {
            println!("   - Discovery client didn't receive retained messages");
            println!("     This suggests MQTT broker retain functionality issues");
        }
        if !final_results.json_parsing_successful {
            println!("   - JSON payload parsing failed");
            println!("     This suggests payload format mismatch");
        }
        if !final_results.world_cache_updated {
            println!("   - World cache was not updated after successful parsing");
        }
        if final_cache.is_empty() {
            println!("   - No worlds in final cache");
        }
    }

    // Clean up tasks
    publisher_task.abort();
    discovery_task.abort();

    // Assert for test framework
    assert!(
        all_good,
        "MQTT World Discovery test failed - see output above for details"
    );
}

// Additional test for just the JSON deserialization
#[test]
fn test_shared_world_info_deserialization() {
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
