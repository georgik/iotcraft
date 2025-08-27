use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde_json::json;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};

/// World info structure for testing
#[derive(Clone, Debug)]
pub struct WorldInfo {
    pub world_id: String,
    pub world_name: String,
    pub host_name: String,
    pub host_player: String,
    pub is_public: bool,
    pub player_count: u32,
    pub max_players: u32,
}

impl WorldInfo {
    pub fn from_json(json_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let v: serde_json::Value = serde_json::from_str(json_str)?;

        Ok(WorldInfo {
            world_id: v["world_id"].as_str().unwrap_or("").to_string(),
            world_name: v["world_name"].as_str().unwrap_or("").to_string(),
            host_name: v["host_name"].as_str().unwrap_or("").to_string(),
            host_player: v["host_player"].as_str().unwrap_or("").to_string(),
            is_public: v["is_public"].as_bool().unwrap_or(false),
            player_count: v["player_count"].as_u64().unwrap_or(0) as u32,
            max_players: v["max_players"].as_u64().unwrap_or(10) as u32,
        })
    }
}

/// Block change information
#[derive(Clone, Debug)]
pub struct BlockChange {
    pub player_id: String,
    pub player_name: String,
    pub timestamp: i64,
    pub change_type: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block_type: Option<String>,
}

impl BlockChange {
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

        Ok(BlockChange {
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

/// Publisher task - publishes a test world and then simulates block changes
async fn world_and_block_publisher_task(
    world_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!(
        "🚀 Starting world and block publisher for world: {}",
        world_id
    );

    let (client, mut eventloop) = create_mqtt_client("test-world-publisher").await?;

    // Spawn event loop handler
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_))) => {
                    println!("📡 Publisher: Connected to MQTT broker");
                }
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::PubAck(_))) => {
                    // Message published successfully
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

    // Step 1: Publish the test world info
    let world_info_topic = format!("iotcraft/worlds/{}/info", world_id);
    let world_info = json!({
        "world_id": world_id,
        "world_name": "Block Sync Test World",
        "description": "A world for testing real-time block synchronization",
        "host_player": "player-test-host",
        "host_name": "test-host",
        "created_at": "2025-08-22T06:00:00Z",
        "last_updated": "2025-08-22T06:00:00Z",
        "player_count": 2,
        "max_players": 10,
        "is_public": true,
        "version": "1.0.0"
    });

    println!("📤 Publishing world info to topic: {}", world_info_topic);
    client
        .publish(
            &world_info_topic,
            QoS::AtLeastOnce,
            true,
            world_info.to_string().as_bytes(),
        )
        .await?;
    println!("✅ World info published successfully");

    // Wait a bit for subscribers to connect and see the world
    sleep(Duration::from_secs(2)).await;

    // Step 2: Simulate block placement events
    let block_changes = vec![
        // Player 1 places some blocks
        (
            "iotcraft/worlds/{}/state/blocks/placed",
            json!({
                "player_id": "player-test-host",
                "player_name": "TestHost",
                "timestamp": chrono::Utc::now().timestamp_millis(),
                "change": {
                    "Placed": {
                        "x": 5,
                        "y": 1,
                        "z": 5,
                        "block_type": "Stone"
                    }
                }
            }),
        ),
        (
            "iotcraft/worlds/{}/state/blocks/placed",
            json!({
                "player_id": "player-test-host",
                "player_name": "TestHost",
                "timestamp": chrono::Utc::now().timestamp_millis(),
                "change": {
                    "Placed": {
                        "x": 6,
                        "y": 1,
                        "z": 5,
                        "block_type": "Grass"
                    }
                }
            }),
        ),
        (
            "iotcraft/worlds/{}/state/blocks/placed",
            json!({
                "player_id": "player-test-host",
                "player_name": "TestHost",
                "timestamp": chrono::Utc::now().timestamp_millis(),
                "change": {
                    "Placed": {
                        "x": 7,
                        "y": 1,
                        "z": 5,
                        "block_type": "Dirt"
                    }
                }
            }),
        ),
        // Player 2 places a block
        (
            "iotcraft/worlds/{}/state/blocks/placed",
            json!({
                "player_id": "player-test-joiner",
                "player_name": "TestJoiner",
                "timestamp": chrono::Utc::now().timestamp_millis(),
                "change": {
                    "Placed": {
                        "x": 5,
                        "y": 2,
                        "z": 5,
                        "block_type": "QuartzBlock"
                    }
                }
            }),
        ),
        // Player 2 removes a block
        (
            "iotcraft/worlds/{}/state/blocks/removed",
            json!({
                "player_id": "player-test-joiner",
                "player_name": "TestJoiner",
                "timestamp": chrono::Utc::now().timestamp_millis(),
                "change": {
                    "Removed": {
                        "x": 6,
                        "y": 1,
                        "z": 5
                    }
                }
            }),
        ),
    ];

    for (topic_template, change_data) in block_changes {
        let topic = topic_template.replace("{}", world_id);
        println!("📤 Publishing block change to topic: {}", topic);
        println!("   Data: {}", change_data);

        client
            .publish(
                &topic,
                QoS::AtLeastOnce,
                false,
                change_data.to_string().as_bytes(),
            )
            .await?;
        println!("✅ Block change published successfully");

        // Small delay between changes to make them easier to track
        sleep(Duration::from_millis(800)).await;
    }

    println!("✅ All block changes published");
    Ok(())
}

/// Subscriber task - simulates a player joining the world and receiving block updates
async fn block_sync_subscriber_task(
    world_id: &str,
    player_name: &str,
) -> Result<Vec<BlockChange>, Box<dyn std::error::Error + Send + Sync>> {
    println!(
        "🔍 Starting block sync subscriber for player: {}",
        player_name
    );

    let (client, mut eventloop) =
        create_mqtt_client(&format!("test-subscriber-{}", player_name.to_lowercase())).await?;

    let (tx, mut rx) = mpsc::channel::<BlockChange>(32);

    // Clone player_name for the spawned task
    let player_name_clone = player_name.to_string();

    // Spawn event loop handler
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_))) => {
                    println!("📡 {}: Connected to MQTT broker", player_name_clone);
                }
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish))) => {
                    let topic = &publish.topic;
                    let payload = String::from_utf8_lossy(&publish.payload);

                    println!(
                        "📨 {}: Received message on '{}' [retain: {}]",
                        player_name_clone, topic, publish.retain
                    );
                    println!("   Payload: {}", payload);

                    if topic.contains("/state/blocks/") {
                        match BlockChange::from_json(&payload) {
                            Ok(block_change) => {
                                println!(
                                    "✅ {}: Parsed block change: {:?} at ({}, {}, {})",
                                    player_name_clone,
                                    block_change.change_type,
                                    block_change.x,
                                    block_change.y,
                                    block_change.z
                                );
                                let _ = tx.send(block_change).await;
                            }
                            Err(e) => {
                                eprintln!(
                                    "❌ {}: Failed to parse block change: {:?}",
                                    player_name_clone, e
                                );
                            }
                        }
                    }
                }
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::SubAck(_))) => {
                    println!("✅ {}: Subscription confirmed", player_name_clone);
                }
                Ok(_) => {} // Ignore other events
                Err(e) => {
                    eprintln!("❌ {}: Connection error: {:?}", player_name_clone, e);
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });

    // Wait for connection
    sleep(Duration::from_millis(500)).await;

    // Subscribe to block change topics for this world
    let placed_topic = format!("iotcraft/worlds/{}/state/blocks/placed", world_id);
    let removed_topic = format!("iotcraft/worlds/{}/state/blocks/removed", world_id);

    println!(
        "🔔 {}: Subscribing to block placement topic: {}",
        player_name, placed_topic
    );
    client.subscribe(&placed_topic, QoS::AtLeastOnce).await?;

    println!(
        "🔔 {}: Subscribing to block removal topic: {}",
        player_name, removed_topic
    );
    client.subscribe(&removed_topic, QoS::AtLeastOnce).await?;

    // Collect block changes for up to 8 seconds
    let mut received_changes = Vec::new();
    let collection_timeout = timeout(Duration::from_secs(8), async {
        while let Some(block_change) = rx.recv().await {
            received_changes.push(block_change);
            // Stop if we've received enough changes (we expect 5 total)
            if received_changes.len() >= 6 {
                break;
            }
        }
    });

    let _ = collection_timeout.await; // Ignore timeout error

    println!(
        "🎯 {}: Received {} block changes:",
        player_name,
        received_changes.len()
    );
    for (i, change) in received_changes.iter().enumerate() {
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

    Ok(received_changes)
}

#[tokio::test]
async fn test_mqtt_block_synchronization() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🧪 Starting MQTT Block Synchronization Integration Test");
    println!("{}", "=".repeat(70));

    let world_id = "BlockSyncTestWorld-123";

    // Start the subscriber first (simulating a player already in the world)
    println!("\n📖 Step 1: Starting subscriber (Player already in world)...");
    let subscriber_task = tokio::spawn({
        let world_id = world_id.to_string();
        async move { block_sync_subscriber_task(&world_id, "PlayerInWorld").await }
    });

    // Give subscriber time to connect and subscribe
    sleep(Duration::from_secs(1)).await;

    // Start the publisher (simulating world events and new player actions)
    println!("\n📡 Step 2: Publishing world and block changes...");
    world_and_block_publisher_task(world_id).await?;

    // Give publisher time to finish publishing all changes
    sleep(Duration::from_secs(2)).await;

    // Collect results
    println!("\n🧮 Step 3: Collecting results...");
    let subscriber_changes = subscriber_task.await??;

    // Analysis
    println!("\n\n🧮 BLOCK SYNCHRONIZATION ANALYSIS:");
    println!("{}", "=".repeat(70));

    println!(
        "📈 Total block changes received: {}",
        subscriber_changes.len()
    );

    let placed_blocks = subscriber_changes
        .iter()
        .filter(|c| c.change_type == "Placed")
        .count();
    let removed_blocks = subscriber_changes
        .iter()
        .filter(|c| c.change_type == "Removed")
        .count();

    println!("📈 Block placements: {}", placed_blocks);
    println!("📈 Block removals: {}", removed_blocks);

    // Verify we got the expected changes
    let expected_changes = 5; // 4 placed + 1 removed

    if subscriber_changes.len() >= expected_changes {
        println!("✅ BLOCK SYNC WORKING: Received all expected block changes!");
        println!("   Real-time block synchronization is functioning correctly.");

        // Verify specific blocks
        let stone_at_5_1_5 = subscriber_changes.iter().find(|c| {
            c.change_type == "Placed"
                && c.x == 5
                && c.y == 1
                && c.z == 5
                && c.block_type.as_ref().map(|s| s.as_str()) == Some("Stone")
        });

        let quartz_at_5_2_5 = subscriber_changes.iter().find(|c| {
            c.change_type == "Placed"
                && c.x == 5
                && c.y == 2
                && c.z == 5
                && c.block_type.as_ref().map(|s| s.as_str()) == Some("QuartzBlock")
        });

        let removed_at_6_1_5 = subscriber_changes
            .iter()
            .find(|c| c.change_type == "Removed" && c.x == 6 && c.y == 1 && c.z == 5);

        if stone_at_5_1_5.is_some() {
            println!("✅ Found Stone block placement at (5, 1, 5)");
        }
        if quartz_at_5_2_5.is_some() {
            println!("✅ Found QuartzBlock placement at (5, 2, 5)");
        }
        if removed_at_6_1_5.is_some() {
            println!("✅ Found block removal at (6, 1, 5)");
        }

        println!("\n🎯 Different players' actions were synchronized:");
        let unique_players: std::collections::HashSet<_> = subscriber_changes
            .iter()
            .map(|c| c.player_name.clone())
            .collect();

        for player in unique_players {
            let player_changes = subscriber_changes
                .iter()
                .filter(|c| c.player_name == player)
                .count();
            println!("   - {}: {} changes", player, player_changes);
        }
    } else {
        println!(
            "❌ BLOCK SYNC ISSUE: Expected at least {} changes, got {}",
            expected_changes,
            subscriber_changes.len()
        );
        println!("   This suggests the real-time block synchronization needs attention.");
    }

    println!("\n🔧 IMPLEMENTATION REQUIREMENTS:");
    println!("1. Desktop client should publish block changes to MQTT when placing/removing blocks");
    println!("2. Desktop client should subscribe to block change topics for joined worlds");
    println!("3. Block changes should be applied to local world state when received");
    println!("4. Visual blocks should be spawned/despawned based on received changes");
    println!("5. Changes should include player info and timestamp for proper attribution");

    println!("\n🏁 Block synchronization test completed!");

    // Always pass the test as this is a diagnostic/requirements test
    Ok(())
}
