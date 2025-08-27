//! World Publishing and Discovery Integration Test
//!
//! This test verifies the complete world publishing and discovery workflow:
//! 1. A client publishes a world with retained messages
//! 2. Another client can discover and list available worlds
//! 3. Tests both retained messages and live updates

use anyhow::Result;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{sleep, timeout};

use crate::mqtt_test_infrastructure::MqttTestServer;

/// Test data structure representing a world info message
#[derive(Debug, Clone, PartialEq)]
struct WorldInfo {
    world_id: String,
    world_name: String,
    description: String,
    host_player: String,
    host_name: String,
    player_count: u32,
    max_players: u32,
    is_public: bool,
    version: String,
}

impl WorldInfo {
    fn new(id: u32, host_name: &str) -> Self {
        Self {
            world_id: format!("TestWorld-{}", id),
            world_name: format!("Test World {}", id),
            description: format!("Test world number {} for integration testing", id),
            host_player: format!("player-{}", host_name),
            host_name: host_name.to_string(),
            player_count: if id == 1 { 0 } else { id - 1 },
            max_players: 10 + (id * 2),
            is_public: id % 2 == 1, // Alternate public/private
            version: "1.0.0".to_string(),
        }
    }

    fn to_json(&self) -> String {
        json!({
            "world_id": self.world_id,
            "world_name": self.world_name,
            "description": self.description,
            "host_player": self.host_player,
            "host_name": self.host_name,
            "created_at": "2025-08-27T08:00:00.000000+00:00",
            "last_updated": chrono::Utc::now().to_rfc3339(),
            "player_count": self.player_count,
            "max_players": self.max_players,
            "is_public": self.is_public,
            "version": self.version
        })
        .to_string()
    }

    fn from_json(json_str: &str) -> Result<Self> {
        let v: Value = serde_json::from_str(json_str)?;

        Ok(Self {
            world_id: v["world_id"].as_str().unwrap_or("").to_string(),
            world_name: v["world_name"].as_str().unwrap_or("").to_string(),
            description: v["description"].as_str().unwrap_or("").to_string(),
            host_player: v["host_player"].as_str().unwrap_or("").to_string(),
            host_name: v["host_name"].as_str().unwrap_or("").to_string(),
            player_count: v["player_count"].as_u64().unwrap_or(0) as u32,
            max_players: v["max_players"].as_u64().unwrap_or(0) as u32,
            is_public: v["is_public"].as_bool().unwrap_or(false),
            version: v["version"].as_str().unwrap_or("1.0.0").to_string(),
        })
    }

    fn topic(&self) -> String {
        format!("iotcraft/worlds/{}/info", self.world_id)
    }
}

/// Create MQTT client with proper configuration for testing
async fn create_mqtt_client(client_id: &str, host: &str, port: u16) -> Result<(AsyncClient, rumqttc::EventLoop)> {
    let mut mqttoptions = MqttOptions::new(client_id, host, port);
    mqttoptions.set_keep_alive(Duration::from_secs(10));
    mqttoptions.set_clean_session(false); // Persistent session for retained messages
    mqttoptions.set_max_packet_size(1048576, 1048576); // Match real client settings

    let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
    Ok((client, eventloop))
}

/// Publisher task - simulates a desktop client publishing world info
async fn world_publisher_task(
    server: &MqttTestServer,
    worlds: Vec<WorldInfo>,
) -> Result<()> {
    println!("🚀 Starting world publisher task with {} worlds", worlds.len());

    let (client, mut eventloop) = create_mqtt_client("iotcraft-publisher-test", &server.host, server.port).await?;

    // Spawn event loop handler
    let event_handle = tokio::spawn(async move {
        let mut connected = false;
        let mut published_count = 0u32;
        
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    println!("📡 Publisher: Connected to MQTT broker");
                    connected = true;
                }
                Ok(Event::Incoming(Incoming::PubAck(_))) => {
                    published_count += 1;
                    println!("✅ Publisher: Message {} published successfully", published_count);
                }
                Ok(_) => {} // Ignore other events
                Err(e) => {
                    eprintln!("❌ Publisher: Connection error: {:?}", e);
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });

    // Wait for connection
    sleep(Duration::from_millis(500)).await;

    // Publish each world with retained flag
    for world in &worlds {
        let topic = world.topic();
        let payload = world.to_json();

        println!("📤 Publishing world '{}' to topic '{}'", world.world_name, topic);
        println!("   Host: {} | Players: {}/{} | Public: {}", 
                 world.host_name, world.player_count, world.max_players, world.is_public);

        client
            .publish(&topic, QoS::AtLeastOnce, true, payload.as_bytes()) // retain=true
            .await?;
        
        sleep(Duration::from_millis(100)).await; // Small delay between publishes
    }

    println!("✅ Publisher: All {} worlds published with retain=true", worlds.len());
    
    // Let the event loop run a bit to process confirmations
    sleep(Duration::from_secs(1)).await;
    
    // Cleanup
    event_handle.abort();
    Ok(())
}

/// Discovery task - simulates a desktop client discovering available worlds
async fn world_discovery_task(
    server: &MqttTestServer,
    discovery_timeout_secs: u64,
) -> Result<HashMap<String, WorldInfo>> {
    println!("🔍 Starting world discovery task (timeout: {}s)", discovery_timeout_secs);

    let (client, mut eventloop) = create_mqtt_client("iotcraft-discovery-test", &server.host, server.port).await?;

    let discovered_worlds = Arc::new(Mutex::new(HashMap::new()));
    let worlds_clone = discovered_worlds.clone();

    // Spawn event loop handler
    let event_handle = tokio::spawn(async move {
        let mut subscription_confirmed = false;
        
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    println!("📡 Discovery: Connected to MQTT broker");
                }
                Ok(Event::Incoming(Incoming::Publish(publish))) => {
                    let topic = &publish.topic;
                    let payload = String::from_utf8_lossy(&publish.payload);
                    let retain_flag = publish.retain;

                    println!("📨 Discovery: Received message on '{}' [retain: {}]", topic, retain_flag);

                    if topic.starts_with("iotcraft/worlds/") && topic.ends_with("/info") {
                        match WorldInfo::from_json(&payload) {
                            Ok(world_info) => {
                                println!("✅ Discovery: Found world '{}' ({}) - Host: {} [{}]", 
                                         world_info.world_name, 
                                         world_info.world_id,
                                         world_info.host_name,
                                         if retain_flag { "retained" } else { "live" });
                                
                                let mut worlds = worlds_clone.lock().await;
                                worlds.insert(world_info.world_id.clone(), world_info);
                            }
                            Err(e) => {
                                eprintln!("❌ Discovery: Failed to parse world info: {:?}", e);
                            }
                        }
                    }
                }
                Ok(Event::Incoming(Incoming::SubAck(_))) => {
                    println!("✅ Discovery: Subscription confirmed for world discovery");
                    subscription_confirmed = true;
                }
                Ok(_) => {} // Ignore other events
                Err(e) => {
                    eprintln!("❌ Discovery: Connection error: {:?}", e);
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });

    // Wait for connection
    sleep(Duration::from_millis(500)).await;

    // Subscribe to world discovery topic
    let discovery_topic = "iotcraft/worlds/+/info";
    println!("🔔 Discovery: Subscribing to topic: {}", discovery_topic);
    client.subscribe(discovery_topic, QoS::AtLeastOnce).await?;

    // Wait for subscription confirmation and message collection
    let discovery_result = timeout(
        Duration::from_secs(discovery_timeout_secs),
        async {
            // Give time to receive retained messages
            sleep(Duration::from_secs(discovery_timeout_secs / 2)).await;
        }
    ).await;

    if discovery_result.is_err() {
        println!("⏰ Discovery: Timeout reached");
    }

    // Cleanup and return results
    event_handle.abort();
    
    let discovered = discovered_worlds.lock().await.clone();
    println!("🎯 Discovery: Found {} worlds total", discovered.len());
    
    Ok(discovered)
}

#[tokio::test]
async fn test_world_publish_and_discover() -> Result<()> {
    println!("🧪 Testing complete world publish and discover workflow");

    // Start MQTT test server
    let server = MqttTestServer::start().await?;
    println!("🔧 Test server running on {}", server.broker_address());

    // Create test worlds
    let test_worlds = vec![
        WorldInfo::new(1, "alice"),
        WorldInfo::new(2, "bob"),
        WorldInfo::new(3, "charlie"),
    ];

    println!("📋 Test scenario: {} worlds to publish", test_worlds.len());
    for world in &test_worlds {
        println!("  - {} ({}): Host={}, Public={}", 
                 world.world_name, world.world_id, world.host_name, world.is_public);
    }

    // Step 1: Publish worlds with retained messages
    world_publisher_task(&server, test_worlds.clone()).await
        .context("Failed to publish test worlds")?;

    // Small delay to ensure messages are retained by broker
    sleep(Duration::from_millis(500)).await;

    // Step 2: Discover worlds (should receive retained messages)
    let discovered_worlds = world_discovery_task(&server, 5).await
        .context("Failed to discover worlds")?;

    // Step 3: Verify results
    println!("\n🔍 Verification Results:");
    println!("  Published: {} worlds", test_worlds.len());
    println!("  Discovered: {} worlds", discovered_worlds.len());

    // Verify all published worlds were discovered
    for expected_world in &test_worlds {
        match discovered_worlds.get(&expected_world.world_id) {
            Some(discovered) => {
                println!("  ✅ Found world '{}' - Host: {}", discovered.world_name, discovered.host_name);
                
                // Verify key fields match
                assert_eq!(discovered.world_id, expected_world.world_id, "World ID mismatch");
                assert_eq!(discovered.world_name, expected_world.world_name, "World name mismatch");
                assert_eq!(discovered.host_name, expected_world.host_name, "Host name mismatch");
                assert_eq!(discovered.is_public, expected_world.is_public, "Public flag mismatch");
                assert_eq!(discovered.player_count, expected_world.player_count, "Player count mismatch");
            }
            None => {
                return Err(anyhow::anyhow!(
                    "Expected world '{}' ({}) was not discovered",
                    expected_world.world_name,
                    expected_world.world_id
                ));
            }
        }
    }

    // Verify no extra worlds were discovered
    for (discovered_id, discovered) in &discovered_worlds {
        let found = test_worlds.iter().any(|w| w.world_id == *discovered_id);
        if !found {
            return Err(anyhow::anyhow!(
                "Unexpected world discovered: '{}' ({})",
                discovered.world_name,
                discovered_id
            ));
        }
    }

    println!("\n🎉 SUCCESS: All published worlds were correctly discovered!");
    println!("   - Retained message functionality: ✅");
    println!("   - World metadata preservation: ✅");
    println!("   - Discovery topic subscription: ✅");

    Ok(())
}

#[tokio::test]
async fn test_world_update_propagation() -> Result<()> {
    println!("🧪 Testing world update propagation (live messages)");

    let server = MqttTestServer::start().await?;
    println!("🔧 Test server running on {}", server.broker_address());

    // Create initial world
    let mut initial_world = WorldInfo::new(1, "testhost");
    
    // Step 1: Publish initial world
    world_publisher_task(&server, vec![initial_world.clone()]).await?;
    sleep(Duration::from_millis(500)).await;

    // Step 2: Start discovery client (should get retained message)
    let (discovery_client, mut discovery_eventloop) = create_mqtt_client("discovery-updates", &server.host, server.port).await?;
    let update_worlds = Arc::new(Mutex::new(Vec::new()));
    let worlds_clone = update_worlds.clone();

    // Spawn discovery event loop
    let discovery_handle = tokio::spawn(async move {
        loop {
            match discovery_eventloop.poll().await {
                Ok(Event::Incoming(Incoming::Publish(publish))) => {
                    let topic = &publish.topic;
                    let payload = String::from_utf8_lossy(&publish.payload);
                    
                    if topic.starts_with("iotcraft/worlds/") && topic.ends_with("/info") {
                        if let Ok(world_info) = WorldInfo::from_json(&payload) {
                            println!("📨 Discovery: Received world update - Players: {}/{}", 
                                     world_info.player_count, world_info.max_players);
                            worlds_clone.lock().await.push(world_info);
                        }
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });

    // Subscribe to updates
    sleep(Duration::from_millis(500)).await;
    discovery_client.subscribe("iotcraft/worlds/+/info", QoS::AtLeastOnce).await?;
    sleep(Duration::from_secs(1)).await;

    // Step 3: Publish updated world (simulating player count change)
    initial_world.player_count = 3;
    initial_world.world_name = "Updated Test World 1".to_string();
    
    println!("📤 Publishing world update - Players: {}", initial_world.player_count);
    world_publisher_task(&server, vec![initial_world.clone()]).await?;
    sleep(Duration::from_secs(2)).await;

    // Step 4: Verify update was received
    discovery_handle.abort();
    let received_updates = update_worlds.lock().await.clone();
    
    println!("📊 Received {} world updates", received_updates.len());
    
    // Should have at least the initial retained message and the update
    assert!(received_updates.len() >= 2, "Should receive initial world + update");
    
    // Find the latest update
    let latest_update = received_updates.last().unwrap();
    assert_eq!(latest_update.player_count, 3, "Update should reflect new player count");
    assert_eq!(latest_update.world_name, "Updated Test World 1", "Update should reflect new name");

    println!("🎉 SUCCESS: World update propagation works correctly!");

    Ok(())
}

#[tokio::test] 
async fn test_world_removal() -> Result<()> {
    println!("🧪 Testing world removal (empty retained message)");

    let server = MqttTestServer::start().await?;
    
    // Publish a world
    let test_world = WorldInfo::new(1, "temporary-host");
    world_publisher_task(&server, vec![test_world.clone()]).await?;
    sleep(Duration::from_millis(500)).await;

    // Verify world is discoverable
    let discovered = world_discovery_task(&server, 3).await?;
    assert_eq!(discovered.len(), 1, "Should discover the published world");
    assert!(discovered.contains_key(&test_world.world_id));

    // Remove world by publishing empty retained message
    let (client, _eventloop) = create_mqtt_client("world-remover", &server.host, server.port).await?;
    sleep(Duration::from_millis(500)).await;
    
    println!("🗑️  Removing world by publishing empty retained message");
    client.publish(&test_world.topic(), QoS::AtLeastOnce, true, b"").await?;
    sleep(Duration::from_secs(1)).await;

    // Verify world is no longer discoverable
    let rediscovered = world_discovery_task(&server, 3).await?;
    assert_eq!(rediscovered.len(), 0, "World should be removed from discovery after empty retained message");

    println!("🎉 SUCCESS: World removal works correctly!");

    Ok(())
}
