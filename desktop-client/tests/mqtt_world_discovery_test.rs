use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};

/// Test data structure to represent a world info message
#[derive(Debug, Clone)]
struct WorldInfo {
    world_id: String,
    world_name: String,
    description: String,
    host_player: String,
    host_name: String,
    player_count: u32,
    max_players: u32,
    is_public: bool,
}

impl WorldInfo {
    fn from_json(json_str: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let v: Value = serde_json::from_str(json_str)?;

        Ok(WorldInfo {
            world_id: v["world_id"].as_str().unwrap_or("").to_string(),
            world_name: v["world_name"].as_str().unwrap_or("").to_string(),
            description: v["description"].as_str().unwrap_or("").to_string(),
            host_player: v["host_player"].as_str().unwrap_or("").to_string(),
            host_name: v["host_name"].as_str().unwrap_or("").to_string(),
            player_count: v["player_count"].as_u64().unwrap_or(0) as u32,
            max_players: v["max_players"].as_u64().unwrap_or(0) as u32,
            is_public: v["is_public"].as_bool().unwrap_or(false),
        })
    }

    fn to_json(&self) -> String {
        serde_json::json!({
            "world_id": self.world_id,
            "world_name": self.world_name,
            "description": self.description,
            "host_player": self.host_player,
            "host_name": self.host_name,
            "created_at": "2025-08-21T19:25:51.477077+00:00",
            "last_updated": "2025-08-22T05:14:35.916185+00:00",
            "player_count": self.player_count,
            "max_players": self.max_players,
            "is_public": self.is_public,
            "version": "1.0.0"
        })
        .to_string()
    }
}

/// Create MQTT client with given client ID
async fn create_mqtt_client(
    client_id: &str,
) -> Result<(AsyncClient, rumqttc::EventLoop), Box<dyn std::error::Error>> {
    let mut mqttoptions = MqttOptions::new(client_id, "localhost", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(10));
    mqttoptions.set_clean_session(false); // Use persistent session to receive retained messages

    let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
    Ok((client, eventloop))
}

/// Publisher task - simulates the desktop client publishing world info
async fn publisher_task() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting publisher task...");

    let (client, mut eventloop) = create_mqtt_client("test-publisher").await?;

    // Test worlds to publish
    let worlds = vec![
        WorldInfo {
            world_id: "TestWorld-1".to_string(),
            world_name: "Test World 1".to_string(),
            description: "A test world".to_string(),
            host_player: "player-test1".to_string(),
            host_name: "test-user-1".to_string(),
            player_count: 1,
            max_players: 10,
            is_public: true,
        },
        WorldInfo {
            world_id: "TestWorld-2".to_string(),
            world_name: "Test World 2".to_string(),
            description: "Another test world".to_string(),
            host_player: "player-test2".to_string(),
            host_name: "test-user-2".to_string(),
            player_count: 2,
            max_players: 8,
            is_public: true,
        },
        WorldInfo {
            world_id: "TestWorld-3".to_string(),
            world_name: "Test World 3".to_string(),
            description: "Third test world".to_string(),
            host_player: "player-test3".to_string(),
            host_name: "test-user-3".to_string(),
            player_count: 0,
            max_players: 12,
            is_public: false,
        },
    ];

    // Spawn event loop handler
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_))) => {
                    println!("📡 Publisher: Connected to MQTT broker");
                }
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::PubAck(_))) => {
                    println!("✅ Publisher: Message published successfully");
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

    // Publish each world
    for world in &worlds {
        let topic = format!("iotcraft/worlds/{}/info", world.world_id);
        let payload = world.to_json();

        println!(
            "📤 Publishing world '{}' to topic '{}'",
            world.world_name, topic
        );
        println!("   Payload: {}", payload);

        client
            .publish(&topic, QoS::AtLeastOnce, true, payload.as_bytes())
            .await?;
        sleep(Duration::from_millis(100)).await; // Small delay between publishes
    }

    println!("✅ Publisher: All test worlds published");
    sleep(Duration::from_secs(1)).await; // Give time for messages to be retained

    Ok(())
}

/// Subscriber task - simulates the desktop client subscribing to world info
async fn subscriber_task() -> Result<HashMap<String, WorldInfo>, Box<dyn std::error::Error>> {
    println!("🔍 Starting subscriber task...");

    let (client, mut eventloop) = create_mqtt_client("test-subscriber").await?;

    let (tx, mut rx) = mpsc::channel::<WorldInfo>(32);

    // Spawn event loop handler
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_))) => {
                    println!("📡 Subscriber: Connected to MQTT broker");
                }
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish))) => {
                    let topic = &publish.topic;
                    let payload = String::from_utf8_lossy(&publish.payload);
                    let retain_flag = publish.retain;

                    println!(
                        "📨 Subscriber: Received message on '{}' [retain: {}]",
                        topic, retain_flag
                    );
                    println!("   Payload: {}", payload);

                    if topic.starts_with("iotcraft/worlds/") && topic.ends_with("/info") {
                        match WorldInfo::from_json(&payload) {
                            Ok(world_info) => {
                                println!(
                                    "✅ Parsed world: {} ({})",
                                    world_info.world_name, world_info.world_id
                                );
                                let _ = tx.send(world_info).await;
                            }
                            Err(e) => {
                                eprintln!("❌ Failed to parse world info: {:?}", e);
                            }
                        }
                    }
                }
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::SubAck(_))) => {
                    println!("✅ Subscriber: Subscription confirmed");
                }
                Ok(_) => {} // Ignore other events
                Err(e) => {
                    eprintln!("❌ Subscriber: Connection error: {:?}", e);
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });

    // Wait for connection
    sleep(Duration::from_millis(500)).await;

    // Subscribe to world info topic
    let topic = "iotcraft/worlds/+/info";
    println!("🔔 Subscribing to topic: {}", topic);
    client.subscribe(topic, QoS::AtLeastOnce).await?;

    // Collect discovered worlds for up to 5 seconds
    let mut discovered_worlds = HashMap::new();
    let collection_timeout = timeout(Duration::from_secs(5), async {
        while let Some(world_info) = rx.recv().await {
            discovered_worlds.insert(world_info.world_id.clone(), world_info);
        }
    });

    let _ = collection_timeout.await; // Ignore timeout error

    println!(
        "🎯 Subscriber: Discovered {} worlds",
        discovered_worlds.len()
    );
    for (world_id, world_info) in &discovered_worlds {
        println!(
            "   - {} ({}): {} players/{} max",
            world_info.world_name, world_id, world_info.player_count, world_info.max_players
        );
    }

    Ok(discovered_worlds)
}

/// Direct reader task - reads the existing worlds from the broker (simulates what desktop client should do)
async fn direct_reader_task() -> Result<HashMap<String, WorldInfo>, Box<dyn std::error::Error>> {
    println!("👁️  Starting direct reader task (checking existing worlds)...");

    let (client, mut eventloop) = create_mqtt_client("test-reader").await?;

    let (tx, mut rx) = mpsc::channel::<WorldInfo>(32);

    // Spawn event loop handler
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_))) => {
                    println!("📡 Reader: Connected to MQTT broker");
                }
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish))) => {
                    let topic = &publish.topic;
                    let payload = String::from_utf8_lossy(&publish.payload);
                    let retain_flag = publish.retain;

                    println!(
                        "📨 Reader: Received message on '{}' [retain: {}]",
                        topic, retain_flag
                    );
                    println!("   Payload: {}", payload);

                    if topic.starts_with("iotcraft/worlds/") && topic.ends_with("/info") {
                        match WorldInfo::from_json(&payload) {
                            Ok(world_info) => {
                                println!(
                                    "✅ Reader parsed world: {} ({})",
                                    world_info.world_name, world_info.world_id
                                );
                                let _ = tx.send(world_info).await;
                            }
                            Err(e) => {
                                eprintln!("❌ Reader failed to parse world info: {:?}", e);
                            }
                        }
                    }
                }
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::SubAck(_))) => {
                    println!("✅ Reader: Subscription confirmed");
                }
                Ok(_) => {} // Ignore other events
                Err(e) => {
                    eprintln!("❌ Reader: Connection error: {:?}", e);
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }
    });

    // Wait for connection
    sleep(Duration::from_millis(500)).await;

    // Subscribe to world info topic (should immediately receive retained messages)
    let topic = "iotcraft/worlds/+/info";
    println!("🔔 Reader subscribing to topic: {}", topic);
    client.subscribe(topic, QoS::AtLeastOnce).await?;

    // Collect discovered worlds for up to 3 seconds (should be fast with retained messages)
    let mut discovered_worlds = HashMap::new();
    let collection_timeout = timeout(Duration::from_secs(3), async {
        while let Some(world_info) = rx.recv().await {
            discovered_worlds.insert(world_info.world_id.clone(), world_info);
            // Stop collecting after we get some worlds to avoid waiting too long
            if discovered_worlds.len() >= 5 {
                break;
            }
        }
    });

    let _ = collection_timeout.await; // Ignore timeout error

    println!(
        "🎯 Reader: Found {} existing worlds",
        discovered_worlds.len()
    );
    for (world_id, world_info) in &discovered_worlds {
        println!(
            "   - {} ({}): {} players/{} max, public: {}",
            world_info.world_name,
            world_id,
            world_info.player_count,
            world_info.max_players,
            world_info.is_public
        );
    }

    Ok(discovered_worlds)
}

#[tokio::test]
async fn test_mqtt_world_discovery() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Starting MQTT World Discovery Integration Test");
    println!("{}", "=".repeat(60));

    // Step 1: Check existing worlds first (what should be the main case)
    println!("\n📖 Step 1: Reading existing worlds from broker...");
    let existing_worlds = direct_reader_task().await?;

    println!("\n📊 EXISTING WORLDS SUMMARY:");
    if existing_worlds.is_empty() {
        println!("❌ No existing worlds found!");
    } else {
        println!("✅ Found {} existing worlds:", existing_worlds.len());
        for (world_id, world_info) in &existing_worlds {
            println!(
                "   - {}: {} by {} ({} players)",
                world_id, world_info.world_name, world_info.host_name, world_info.player_count
            );
        }
    }

    // Give some separation
    sleep(Duration::from_secs(2)).await;

    // Step 2: Test publishing new worlds
    println!("\n📡 Step 2: Publishing test worlds...");
    publisher_task().await?;

    // Give some time for messages to propagate
    sleep(Duration::from_secs(1)).await;

    // Step 3: Test subscribing to see if new worlds are discovered
    println!("\n🔍 Step 3: Testing subscriber (simulating desktop client)...");
    let discovered_worlds = subscriber_task().await?;

    // Step 4: Analysis and results
    println!("\n\n");
    println!("🧮 FINAL ANALYSIS:");
    println!("{}", "=".repeat(60));

    println!("📈 Existing worlds from broker: {}", existing_worlds.len());
    println!("📈 New discovered worlds: {}", discovered_worlds.len());

    let total_unique_worlds: std::collections::HashSet<_> = existing_worlds
        .keys()
        .chain(discovered_worlds.keys())
        .collect();
    println!("📈 Total unique worlds: {}", total_unique_worlds.len());

    // Check if desktop client would have the problem
    if existing_worlds.is_empty() && discovered_worlds.is_empty() {
        println!("🚨 PROBLEM CONFIRMED: No worlds discovered at all!");
        println!("   This suggests the MQTT discovery system is not working correctly.");
    } else if existing_worlds.is_empty() {
        println!("⚠️  POTENTIAL ISSUE: No existing worlds found, but new ones were discovered.");
        println!(
            "   This suggests the desktop client might not be receiving retained messages properly."
        );
        println!("   Check: clean_session settings, QoS levels, subscription timing");
    } else {
        println!("✅ DISCOVERY WORKING: Found existing worlds from broker");
        println!("   The MQTT discovery system appears to be functioning correctly.");
    }

    // Recommendations
    println!("\n🔧 TROUBLESHOOTING RECOMMENDATIONS:");
    println!("1. Check desktop client MQTT connection settings:");
    println!("   - clean_session should be false to receive retained messages");
    println!("   - QoS should be AtLeastOnce or ExactlyOnce");
    println!("   - Client should subscribe immediately after connecting");

    println!("2. Check desktop client subscription topic:");
    println!("   - Should be 'iotcraft/worlds/+/info'");
    println!("   - Make sure the + wildcard is working");

    println!("3. Check desktop client message processing:");
    println!("   - Verify JSON parsing is working correctly");
    println!("   - Check if OnlineWorlds resource is being updated");
    println!("   - Add logging to see if messages are received but not processed");

    println!("\n🏁 Test completed!");

    Ok(())
}
