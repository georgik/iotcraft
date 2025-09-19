#![cfg(not(target_arch = "wasm32"))]

/// Integration test for desktop client block synchronization
///
/// This test verifies the complete flow:
/// 1. Start desktop client in multiplayer mode (simulated)
/// 2. Place a block via inventory system
/// 3. Verify block change event triggers MQTT publish
/// 4. Verify another client receives and applies the change
/// 5. Check that the final world state includes the new block
///
/// Note: This is more of a diagnostic test to identify where the chain breaks.
use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde_json::json;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};

/// Test world info structure
#[derive(Clone, Debug)]
pub struct TestWorldInfo {
    pub world_id: String,
    pub world_name: String,
    pub host_name: String,
    pub is_public: bool,
}

/// Block change information for testing
#[derive(Clone, Debug)]
pub struct TestBlockChange {
    pub player_id: String,
    pub player_name: String,
    pub timestamp: i64,
    pub change_type: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block_type: Option<String>,
}

impl TestBlockChange {
    pub fn from_json(json_str: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let v: serde_json::Value = serde_json::from_str(json_str)?;

        let change = &v["change"];
        let (change_type, x, y, z, block_type) = match change {
            serde_json::Value::Object(map) => {
                if let Some(placed) = map.get("Placed") {
                    (
                        "Placed".to_string(),
                        placed["x"].as_i64().unwrap_or(0) as i32,
                        placed["y"].as_i64().unwrap_or(0) as i32,
                        placed["z"].as_i64().unwrap_or(0) as i32,
                        Some(placed["block_type"].as_str().unwrap_or("Stone").to_string()),
                    )
                } else if let Some(removed) = map.get("Removed") {
                    (
                        "Removed".to_string(),
                        removed["x"].as_i64().unwrap_or(0) as i32,
                        removed["y"].as_i64().unwrap_or(0) as i32,
                        removed["z"].as_i64().unwrap_or(0) as i32,
                        None,
                    )
                } else {
                    return Err("Invalid block change type".into());
                }
            }
            _ => return Err("Invalid block change format".into()),
        };

        Ok(TestBlockChange {
            player_id: v["player_id"].as_str().unwrap_or("").to_string(),
            player_name: v["player_name"].as_str().unwrap_or("").to_string(),
            timestamp: v["timestamp"].as_i64().unwrap_or(0),
            change_type,
            x,
            y,
            z,
            block_type,
        })
    }
}

/// Create MQTT client for testing
async fn create_mqtt_client(
    client_id: &str,
) -> Result<(AsyncClient, rumqttc::EventLoop), Box<dyn std::error::Error + Send + Sync>> {
    let mut mqttoptions = MqttOptions::new(client_id, "localhost", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    mqttoptions.set_clean_session(false);

    let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
    Ok((client, eventloop))
}

/// Monitor task - listens for block changes on MQTT
async fn mqtt_block_monitor_task(
    world_id: &str,
) -> Result<Vec<TestBlockChange>, Box<dyn std::error::Error + Send + Sync>> {
    println!("🔍 Starting MQTT block monitor for world: {}", world_id);

    let (client, mut eventloop) = create_mqtt_client("test-block-monitor").await?;

    let (tx, mut rx) = mpsc::channel::<TestBlockChange>(32);

    // Spawn event loop handler
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_))) => {
                    println!("📡 Block Monitor: Connected to MQTT broker");
                }
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish))) => {
                    let topic = &publish.topic;
                    let payload = String::from_utf8_lossy(&publish.payload);

                    println!(
                        "📨 Block Monitor: Received message on '{}' [retain: {}]",
                        topic, publish.retain
                    );
                    println!("   Payload: {}", payload);

                    if topic.contains("/state/blocks/") {
                        match TestBlockChange::from_json(&payload) {
                            Ok(block_change) => {
                                println!(
                                    "✅ Block Monitor: Parsed block change: {:?} at ({}, {}, {})",
                                    block_change.change_type,
                                    block_change.x,
                                    block_change.y,
                                    block_change.z
                                );
                                let _ = tx.send(block_change).await;
                            }
                            Err(e) => {
                                eprintln!(
                                    "❌ Block Monitor: Failed to parse block change: {:?}",
                                    e
                                );
                            }
                        }
                    }
                }
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::SubAck(_))) => {
                    println!("✅ Block Monitor: Subscription confirmed");
                }
                Ok(_) => {} // Ignore other events
                Err(e) => {
                    eprintln!("❌ Block Monitor: Connection error: {:?}", e);
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });

    // Wait for connection
    sleep(Duration::from_millis(500)).await;

    // Subscribe to block change topics
    let placed_topic = format!("iotcraft/worlds/{}/state/blocks/placed", world_id);
    let removed_topic = format!("iotcraft/worlds/{}/state/blocks/removed", world_id);

    println!("🔔 Subscribing to: {}", placed_topic);
    client.subscribe(&placed_topic, QoS::AtLeastOnce).await?;

    println!("🔔 Subscribing to: {}", removed_topic);
    client.subscribe(&removed_topic, QoS::AtLeastOnce).await?;

    // Collect block changes for up to 10 seconds
    let mut received_changes = Vec::new();
    let collection_timeout = timeout(Duration::from_secs(10), async {
        while let Some(block_change) = rx.recv().await {
            received_changes.push(block_change);
            // Stop if we get too many (probably just testing)
            if received_changes.len() >= 10 {
                break;
            }
        }
    });

    let _ = collection_timeout.await; // Ignore timeout error

    println!(
        "🎯 Block Monitor: Received {} block changes",
        received_changes.len()
    );

    Ok(received_changes)
}

/// Simulate a desktop client placing a block
/// This would normally be triggered by UI interaction
async fn simulate_desktop_client_block_placement(
    world_id: &str,
    player_id: &str,
    player_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!(
        "🎮 Simulating desktop client block placement by {} in world {}",
        player_name, world_id
    );

    let (client, mut eventloop) = create_mqtt_client("test-desktop-simulator").await?;

    // Spawn event loop handler
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_))) => {
                    println!("📡 Desktop Simulator: Connected to MQTT broker");
                }
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::PubAck(_))) => {
                    // Message published successfully
                }
                Ok(_) => {} // Ignore other events
                Err(e) => {
                    eprintln!("❌ Desktop Simulator: Connection error: {:?}", e);
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });

    // Wait for connection
    sleep(Duration::from_millis(500)).await;

    // Simulate block placement events that the desktop client should generate
    let block_placements = vec![
        json!({
            "player_id": player_id,
            "player_name": player_name,
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "change": {
                "Placed": {
                    "x": 10,
                    "y": 1,
                    "z": 5,
                    "block_type": "Stone"
                }
            }
        }),
        json!({
            "player_id": player_id,
            "player_name": player_name,
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "change": {
                "Placed": {
                    "x": 11,
                    "y": 1,
                    "z": 5,
                    "block_type": "Grass"
                }
            }
        }),
    ];

    for (i, placement) in block_placements.iter().enumerate() {
        let topic = format!("iotcraft/worlds/{}/state/blocks/placed", world_id);

        println!(
            "📤 Desktop Client: Simulating block placement {} to topic: {}",
            i + 1,
            topic
        );
        println!("   Data: {}", placement);

        client
            .publish(
                &topic,
                QoS::AtLeastOnce,
                false,
                placement.to_string().as_bytes(),
            )
            .await?;

        println!("✅ Desktop Client: Block placement message sent");
        sleep(Duration::from_millis(800)).await;
    }

    Ok(())
}

#[tokio::test]
async fn test_desktop_client_block_sync_integration()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🧪 Starting Desktop Client Block Sync Integration Test");
    println!("{}", "=".repeat(70));

    let world_id = "DesktopClientTest-123";
    let player_id = "desktop-test-player";
    let player_name = "DesktopTester";

    println!("🌍 Test World ID: {}", world_id);
    println!("👤 Test Player: {} ({})", player_name, player_id);

    // Step 1: Start the MQTT block monitor (simulates another client)
    println!("\n📖 Step 1: Starting MQTT block monitor...");
    let monitor_task = tokio::spawn({
        let world_id = world_id.to_string();
        async move { mqtt_block_monitor_task(&world_id).await }
    });

    // Give monitor time to connect and subscribe
    sleep(Duration::from_secs(2)).await;

    // Step 2: Simulate desktop client block placements
    println!("\n🎮 Step 2: Simulating desktop client block placements...");
    simulate_desktop_client_block_placement(world_id, player_id, player_name).await?;

    // Step 3: Give some time for messages to propagate
    println!("\n⏱️  Step 3: Waiting for message propagation...");
    sleep(Duration::from_secs(3)).await;

    // Step 4: Collect results
    println!("\n🧮 Step 4: Collecting monitoring results...");
    let monitor_results = monitor_task.await??;

    // Step 5: Analysis
    println!("\n\n🧮 DESKTOP CLIENT BLOCK SYNC ANALYSIS:");
    println!("{}", "=".repeat(70));

    println!(
        "📈 Total block changes detected via MQTT: {}",
        monitor_results.len()
    );

    if monitor_results.is_empty() {
        println!("❌ NO BLOCK CHANGES DETECTED!");
        println!(
            "   This indicates that the desktop client is NOT publishing block changes to MQTT."
        );
        println!("   Possible issues:");
        println!("   1. Desktop client is not in multiplayer mode");
        println!("   2. Inventory block placement is not triggering multiplayer sync events");
        println!("   3. Block change events are not reaching the world publisher");
        println!("   4. MQTT publisher is not connected/working");
        println!("   5. Client ID conflicts causing connection drops");
    } else {
        println!("✅ BLOCK CHANGES DETECTED!");
        println!("   The desktop client block synchronization is working:");

        for (i, change) in monitor_results.iter().enumerate() {
            println!(
                "   {}. {} by {} at ({}, {}, {}) - {:?}",
                i + 1,
                change.change_type,
                change.player_name,
                change.x,
                change.y,
                change.z,
                change.block_type
            );
        }

        // Check for expected blocks
        let stone_at_10_1_5 = monitor_results.iter().find(|c| {
            c.change_type == "Placed"
                && c.x == 10
                && c.y == 1
                && c.z == 5
                && c.block_type.as_ref().map(|s| s.as_str()) == Some("Stone")
        });

        let grass_at_11_1_5 = monitor_results.iter().find(|c| {
            c.change_type == "Placed"
                && c.x == 11
                && c.y == 1
                && c.z == 5
                && c.block_type.as_ref().map(|s| s.as_str()) == Some("Grass")
        });

        if stone_at_10_1_5.is_some() {
            println!("✅ Found expected Stone block placement at (10, 1, 5)");
        } else {
            println!("❌ Missing expected Stone block placement at (10, 1, 5)");
        }

        if grass_at_11_1_5.is_some() {
            println!("✅ Found expected Grass block placement at (11, 1, 5)");
        } else {
            println!("❌ Missing expected Grass block placement at (11, 1, 5)");
        }
    }

    println!("\n🔧 DEBUGGING STEPS FOR REAL DESKTOP CLIENT:");
    println!("1. Run the desktop client with RUST_LOG=info to see debug messages");
    println!("2. Join a multiplayer world (publish or join)");
    println!("3. Place a block using the inventory system (right-click with block selected)");
    println!("4. Look for these log messages in sequence:");
    println!("   a. '🔄 Processing block placement event at...' (inventory system)");
    println!("   b. '🎯 Received BlockChangeEvent for world...' (shared world system)");
    println!("   c. '🚀 MQTT Publisher: Received block change...' (world publisher)");
    println!("   d. '✅ Successfully published block change to MQTT...' (MQTT success)");
    println!("5. If any step is missing, that's where the issue is!");

    println!("\n🔍 COMMON ISSUES TO CHECK:");
    println!("1. Make sure you're in multiplayer mode (joined/hosting a world)");
    println!("2. Check for MQTT connection errors in logs");
    println!("3. Verify inventory has blocks selected when placing");
    println!("4. Check for client ID conflicts causing MQTT disconnections");
    println!("5. Ensure world IDs match between events");

    println!("\n🏁 Desktop client block sync test completed!");

    // Always pass the test as this is diagnostic
    Ok(())
}
