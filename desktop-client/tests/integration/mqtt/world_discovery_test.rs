//! World Publishing and Discovery Integration Test
//!
//! This test verifies the complete world publishing and discovery workflow using
//! the xtask MQTT infrastructure to diagnose the issue where published worlds
//! are not being discovered by the desktop client.

use anyhow::{Context, Result};
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};

use crate::mqtt_test_infrastructure::MqttTestServer;

/// Test data structure representing a world info message (matches SharedWorldInfo)
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
        let v: serde_json::Value = serde_json::from_str(json_str)?;

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

/// Create MQTT client with proper configuration for testing
async fn create_mqtt_client(client_id: &str, host: &str, port: u16) -> Result<(AsyncClient, rumqttc::EventLoop)> {
    let mut mqttoptions = MqttOptions::new(client_id, host, port);
    mqttoptions.set_keep_alive(Duration::from_secs(10));
    mqttoptions.set_clean_session(false); // Persistent session for retained messages
    mqttoptions.set_max_packet_size(1048576, 1048576); // Match real client settings

    let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
    Ok((client, eventloop))
}

/// Test that exactly replicates the desktop client's world publishing behavior
#[tokio::test]
async fn test_world_publish_and_discover_exact_replica() -> Result<()> {
    println!("🧪 Testing world publish and discover - Exact Desktop Client Replica");
    println!("====================================================================");
    println!("This test replicates exactly how the desktop client publishes and discovers worlds");

    // Start MQTT test server using xtask infrastructure
    let server = MqttTestServer::start().await
        .context("Failed to start MQTT test server")?;
    println!("🔧 Test server running on {}", server.broker_address());

    // Create a test world matching the format used by desktop client
    let test_world = WorldInfo {
        world_id: "TestWorld-1756283736_1756283788".to_string(), // Format: WorldName-timestamp_timestamp
        world_name: "TestWorld-1756283736".to_string(),
        description: "A test world".to_string(),
        host_player: "player-player-1".to_string(),
        host_name: "testuser".to_string(),
        player_count: 1,
        max_players: 10,
        is_public: true,
        version: "1.0.0".to_string(),
    };

    println!("📋 Test world to publish:");
    println!("  - ID: {}", test_world.world_id);
    println!("  - Name: {}", test_world.world_name);
    println!("  - Host: {}", test_world.host_name);
    println!("  - Topic: {}", test_world.topic());

    // STEP 1: Publish world exactly like desktop client does
    println!("\n📤 Step 1: Publishing world (replicating desktop client behavior)...");
    
    let publisher_client_id = generate_unique_client_id("iotcraft-world-publisher");
    let (publisher_client, mut publisher_eventloop) = create_mqtt_client(&publisher_client_id, &server.host, server.port).await?;
    println!("   Publisher client ID: {}", publisher_client_id);

    // Handle publisher connection
    let publisher_handle = tokio::spawn(async move {
        let mut connected = false;
        let mut published = false;
        
        loop {
            match publisher_eventloop.poll().await {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    println!("   📡 Publisher: Connected to MQTT broker");
                    connected = true;
                }
                Ok(Event::Incoming(Incoming::PubAck(_))) => {
                    if !published {
                        println!("   ✅ Publisher: World info published successfully");
                        published = true;
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("   ❌ Publisher: Connection error: {:?}", e);
                    break;
                }
            }
            
            if connected && published {
                break;
            }
        }
        (connected, published)
    });

    // Wait for connection
    sleep(Duration::from_millis(1000)).await;

    // Publish the world info
    let topic = test_world.topic();
    let payload = test_world.to_json();
    println!("   📤 Publishing to topic: {}", topic);
    println!("   📄 Payload: {}", payload);

    publisher_client
        .publish(&topic, QoS::AtLeastOnce, true, payload.as_bytes()) // retain=true, QoS=AtLeastOnce
        .await
        .context("Failed to publish world info")?;

    // Wait for publish confirmation
    let publisher_result = timeout(Duration::from_secs(5), publisher_handle).await
        .context("Publisher task timeout")?
        .context("Publisher task failed")?;

    if !publisher_result.0 || !publisher_result.1 {
        return Err(anyhow::anyhow!("Publisher failed: connected={}, published={}", publisher_result.0, publisher_result.1));
    }

    println!("   ✅ World published successfully with retain=true");

    // Small delay to ensure message is retained by broker
    sleep(Duration::from_millis(1000)).await;

    // STEP 2: Discover worlds exactly like desktop client does
    println!("\n🔍 Step 2: Discovering worlds (replicating world_discovery.rs behavior)...");
    
    let discovery_client_id = generate_unique_client_id("iotcraft-world-discovery");
    let (discovery_client, mut discovery_eventloop) = create_mqtt_client(&discovery_client_id, &server.host, server.port).await?;
    println!("   Discovery client ID: {}", discovery_client_id);

    let discovered_worlds = Arc::new(Mutex::new(HashMap::new()));
    let worlds_clone = discovered_worlds.clone();

    // Handle discovery connection and messages
    let discovery_handle = tokio::spawn(async move {
        let mut connected = false;
        let mut subscribed = false;
        let mut retained_collection_complete = false;
        let mut subscription_start_time = None;
        
        loop {
            match discovery_eventloop.poll().await {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    println!("   📡 Discovery: Connected to MQTT broker");
                    connected = true;
                }
                Ok(Event::Incoming(Incoming::SubAck(_))) => {
                    println!("   ✅ Discovery: Subscription acknowledged");
                    subscribed = true;
                    subscription_start_time = Some(std::time::Instant::now());
                }
                Ok(Event::Incoming(Incoming::Publish(publish))) => {
                    let topic = &publish.topic;
                    let payload = String::from_utf8_lossy(&publish.payload);
                    let retain_flag = publish.retain;

                    println!("   📨 Discovery: Received message on '{}' [retain: {}]", topic, retain_flag);

                    if topic.starts_with("iotcraft/worlds/") && topic.ends_with("/info") {
                        if payload.is_empty() {
                            println!("      ℹ️ Empty payload (world unpublished)");
                        } else {
                            match WorldInfo::from_json(&payload) {
                                Ok(world_info) => {
                                    println!("      ✅ Successfully parsed world: '{}' ({})", 
                                             world_info.world_name, world_info.world_id);
                                    println!("         Host: {}, Public: {}, Players: {}/{}",
                                             world_info.host_name, world_info.is_public,
                                             world_info.player_count, world_info.max_players);
                                    
                                    let mut worlds = worlds_clone.lock().await;
                                    worlds.insert(world_info.world_id.clone(), world_info);
                                }
                                Err(e) => {
                                    eprintln!("      ❌ Failed to parse world info: {:?}", e);
                                    eprintln!("      📄 Payload was: {}", payload);
                                }
                            }
                        }
                    }
                }
                Ok(_) => {} // Ignore other events
                Err(e) => {
                    eprintln!("   ❌ Discovery: Connection error: {:?}", e);
                    break;
                }
            }

            // Check if we should finish retained message collection
            if let Some(start_time) = subscription_start_time {
                if !retained_collection_complete && start_time.elapsed() > Duration::from_secs(3) {
                    retained_collection_complete = true;
                    let world_count = worlds_clone.lock().await.len();
                    println!("   ⏰ Discovery: Retained message collection complete. Found {} worlds", world_count);
                    break;
                }
            }
        }
        
        (connected, subscribed, retained_collection_complete)
    });

    // Wait for connection
    sleep(Duration::from_millis(1000)).await;

    // Subscribe to world discovery topic (exactly like desktop client)
    let discovery_topic = "iotcraft/worlds/+/info";
    println!("   🔔 Discovery: Subscribing to topic: {}", discovery_topic);
    discovery_client.subscribe(discovery_topic, QoS::AtLeastOnce).await
        .context("Failed to subscribe to world discovery topic")?;

    // Wait for discovery to complete
    let discovery_result = timeout(Duration::from_secs(8), discovery_handle).await
        .context("Discovery task timeout")?
        .context("Discovery task failed")?;

    if !discovery_result.0 || !discovery_result.1 {
        return Err(anyhow::anyhow!("Discovery failed: connected={}, subscribed={}", discovery_result.0, discovery_result.1));
    }

    // STEP 3: Verify results
    println!("\n🔍 Step 3: Verification Results:");
    let discovered = discovered_worlds.lock().await.clone();
    println!("   Discovered {} worlds total", discovered.len());

    for (world_id, world_info) in &discovered {
        println!("   ✅ Found world: '{}' ({})", world_info.world_name, world_id);
    }

    // Check if our test world was discovered
    if let Some(found_world) = discovered.get(&test_world.world_id) {
        println!("\n🎉 SUCCESS: Test world was discovered!");
        println!("   - World ID: {} ✅", found_world.world_id);
        println!("   - World Name: {} ✅", found_world.world_name);
        println!("   - Host Name: {} ✅", found_world.host_name);
        println!("   - Retained message discovery: ✅");

        // Verify all fields match
        assert_eq!(found_world.world_id, test_world.world_id);
        assert_eq!(found_world.world_name, test_world.world_name);
        assert_eq!(found_world.host_name, test_world.host_name);
        assert_eq!(found_world.is_public, test_world.is_public);
        assert_eq!(found_world.player_count, test_world.player_count);
    } else {
        return Err(anyhow::anyhow!(
            "FAILURE: Test world '{}' was NOT discovered!\nDiscovered worlds: {:?}", 
            test_world.world_id,
            discovered.keys().collect::<Vec<_>>()
        ));
    }

    println!("\n✅ All tests passed! World publishing and discovery is working correctly.");
    Ok(())
}

/// Test that specifically checks the timing issue between publishing and discovery
#[tokio::test]
async fn test_discovery_timing_scenarios() -> Result<()> {
    println!("🧪 Testing World Discovery Timing Scenarios");
    println!("===========================================");

    let server = MqttTestServer::start().await?;
    println!("🔧 Test server running on {}", server.broker_address());

    let test_world = WorldInfo::new(42, "timing-test");
    
    // Scenario 1: Publish BEFORE discovery connects (retained message test)
    println!("\n📤 Scenario 1: Publish world BEFORE discovery client connects");
    
    let publisher_client_id = generate_unique_client_id("early-publisher");
    let (publisher_client, mut publisher_eventloop) = create_mqtt_client(&publisher_client_id, &server.host, server.port).await?;

    let publish_handle = tokio::spawn(async move {
        loop {
            match publisher_eventloop.poll().await {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    println!("   📡 Early publisher connected");
                    break;
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });

    sleep(Duration::from_millis(1000)).await;
    
    publisher_client.publish(&test_world.topic(), QoS::AtLeastOnce, true, test_world.to_json().as_bytes()).await?;
    println!("   ✅ World published with retain=true");
    
    timeout(Duration::from_secs(3), publish_handle).await.ok();
    sleep(Duration::from_millis(1000)).await; // Ensure message is retained

    // Now start discovery
    println!("   🔍 Starting discovery AFTER world is published...");
    let discovery_client_id = generate_unique_client_id("late-discovery");
    let (discovery_client, mut discovery_eventloop) = create_mqtt_client(&discovery_client_id, &server.host, server.port).await?;

    let discovered_worlds = Arc::new(Mutex::new(HashMap::new()));
    let worlds_clone = discovered_worlds.clone();

    let discovery_handle = tokio::spawn(async move {
        loop {
            match discovery_eventloop.poll().await {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    println!("      📡 Discovery connected");
                }
                Ok(Event::Incoming(Incoming::SubAck(_))) => {
                    println!("      ✅ Discovery subscribed");
                    // Wait for retained messages
                    sleep(Duration::from_secs(3)).await;
                    break;
                }
                Ok(Event::Incoming(Incoming::Publish(publish))) => {
                    if publish.topic.contains("/info") {
                        let payload = String::from_utf8_lossy(&publish.payload);
                        println!("      📨 Received world info [retain: {}]", publish.retain);
                        if let Ok(world) = WorldInfo::from_json(&payload) {
                            worlds_clone.lock().await.insert(world.world_id.clone(), world);
                        }
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });

    sleep(Duration::from_millis(1000)).await;
    discovery_client.subscribe("iotcraft/worlds/+/info", QoS::AtLeastOnce).await?;
    
    timeout(Duration::from_secs(6), discovery_handle).await.ok();
    
    let discovered = discovered_worlds.lock().await.len();
    if discovered > 0 {
        println!("   ✅ Scenario 1 SUCCESS: Discovered {} worlds via retained messages", discovered);
    } else {
        println!("   ❌ Scenario 1 FAILED: No worlds discovered from retained messages");
        return Err(anyhow::anyhow!("Retained message discovery failed"));
    }

    println!("\n🎉 SUCCESS: World discovery timing works correctly!");
    println!("   - Retained message discovery: ✅");
    println!("   - xtask MQTT infrastructure: ✅");

    Ok(())
}
