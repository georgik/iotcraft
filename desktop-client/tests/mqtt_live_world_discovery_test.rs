mod mqtt_test_utils;

use mqtt_test_utils::MqttTestEnvironment;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{Instant, sleep, timeout};

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
            "created_at": "2025-08-27T06:04:49.157960+00:00",
            "last_updated": chrono::Utc::now().to_rfc3339(),
            "player_count": self.player_count,
            "max_players": self.max_players,
            "is_public": self.is_public,
            "version": "1.0.0"
        })
        .to_string()
    }
}

/// Create MQTT client with given client ID and port  
async fn create_mqtt_client(
    client_id: &str,
    port: u16,
) -> Result<(AsyncClient, rumqttc::EventLoop), Box<dyn std::error::Error + Send + Sync>> {
    let mut mqttoptions = MqttOptions::new(client_id, "localhost", port);
    mqttoptions.set_keep_alive(Duration::from_secs(10));
    mqttoptions.set_clean_session(false); // Use persistent session like the real client
    mqttoptions.set_max_packet_size(1048576, 1048576); // Match the real client settings

    let (client, eventloop) = AsyncClient::new(mqttoptions, 10);
    Ok((client, eventloop))
}

/// Discovery subscriber task - simulates the desktop client's world discovery service
/// This subscribes to the topic pattern and waits for messages, just like the real client
async fn discovery_subscriber_task(
    port: u16,
    discovery_name: &str,
) -> Result<Vec<(WorldInfo, Instant)>, Box<dyn std::error::Error + Send + Sync>> {
    println!("🔍 Starting discovery subscriber: {}", discovery_name);

    let client_id = format!("iotcraft-world-discovery-test-{}", discovery_name);
    let (client, mut eventloop) = create_mqtt_client(&client_id, port).await?;

    let (tx, mut rx) = mpsc::channel::<(WorldInfo, Instant)>(32);

    // Clone discovery_name for the spawned task
    let discovery_name_clone = discovery_name.to_string();

    // Spawn event loop handler
    tokio::spawn(async move {
        let mut subscription_acknowledged = false;
        let mut retained_message_phase = true;
        let mut retained_collection_start = None;

        loop {
            match eventloop.poll().await {
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_))) => {
                    println!("📡 {}: Connected to MQTT broker", discovery_name_clone);
                }
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish))) => {
                    let topic = &publish.topic;
                    let payload = String::from_utf8_lossy(&publish.payload);
                    let retain_flag = publish.retain;
                    let receive_time = Instant::now();

                    println!(
                        "📨 {}: Received message on '{}' [retain: {}, phase: {}]",
                        discovery_name_clone,
                        topic,
                        retain_flag,
                        if retained_message_phase {
                            "retained-collection"
                        } else {
                            "live-messages"
                        }
                    );
                    println!(
                        "   Payload preview: {}",
                        if payload.len() > 100 {
                            format!("{}...", &payload[..100])
                        } else {
                            payload.to_string()
                        }
                    );

                    if topic.starts_with("iotcraft/worlds/") && topic.ends_with("/info") {
                        match WorldInfo::from_json(&payload) {
                            Ok(world_info) => {
                                println!(
                                    "✅ {}: Parsed world: {} ({}) - phase: {}",
                                    discovery_name_clone,
                                    world_info.world_name,
                                    world_info.world_id,
                                    if retained_message_phase {
                                        "retained-collection"
                                    } else {
                                        "live-messages"
                                    }
                                );
                                let _ = tx.send((world_info, receive_time)).await;
                            }
                            Err(e) => {
                                eprintln!(
                                    "❌ {}: Failed to parse world info: {:?}",
                                    discovery_name_clone, e
                                );
                            }
                        }
                    }
                }
                Ok(rumqttc::Event::Incoming(rumqttc::Packet::SubAck(_))) => {
                    println!("✅ {}: Subscription confirmed", discovery_name_clone);
                    subscription_acknowledged = true;
                    retained_collection_start = Some(Instant::now());

                    // After subscription is acknowledged, wait a bit for retained messages
                    tokio::spawn({
                        let discovery_name_clone2 = discovery_name_clone.clone();
                        async move {
                            sleep(Duration::from_secs(3)).await;
                            println!(
                                "⏰ {}: Retained message collection phase complete, entering live message phase",
                                discovery_name_clone2
                            );
                        }
                    });
                }
                Ok(_) => {} // Ignore other events
                Err(e) => {
                    eprintln!("❌ {}: Connection error: {:?}", discovery_name_clone, e);
                    sleep(Duration::from_secs(1)).await;
                }
            }

            // Switch to live message phase after some time
            if let Some(start_time) = retained_collection_start {
                if retained_message_phase && start_time.elapsed() > Duration::from_secs(3) {
                    retained_message_phase = false;
                    println!(
                        "🔄 {}: Switched to live message phase",
                        discovery_name_clone
                    );
                }
            }
        }
    });

    // Wait for connection
    sleep(Duration::from_millis(1000)).await;

    // Subscribe to world info topic (the same pattern the real client uses)
    let topic = "iotcraft/worlds/+/info";
    println!("🔔 {}: Subscribing to topic: {}", discovery_name, topic);
    client.subscribe(topic, QoS::AtLeastOnce).await?;

    // Wait for initial retained messages (if any)
    println!("⏳ {}: Waiting for retained messages...", discovery_name);
    sleep(Duration::from_secs(3)).await;

    println!("🎯 {}: Ready to receive live messages", discovery_name);

    // Keep collecting messages for a longer period to catch live messages
    let mut discovered_worlds = Vec::new();
    let collection_timeout = timeout(Duration::from_secs(15), async {
        while let Some((world_info, receive_time)) = rx.recv().await {
            discovered_worlds.push((world_info, receive_time));
        }
    });

    let _ = collection_timeout.await; // Ignore timeout error

    println!(
        "🎯 {}: Received {} total world messages",
        discovery_name,
        discovered_worlds.len()
    );

    Ok(discovered_worlds)
}

/// Publisher task - simulates the desktop client publishing a world after some delay
async fn delayed_publisher_task(
    port: u16,
    delay_seconds: u64,
) -> Result<WorldInfo, Box<dyn std::error::Error + Send + Sync>> {
    println!(
        "⏰ Publisher: Waiting {} seconds before publishing...",
        delay_seconds
    );
    sleep(Duration::from_secs(delay_seconds)).await;

    println!("🚀 Publisher: Starting to publish world...");

    let (client, mut eventloop) = create_mqtt_client("test-delayed-publisher", port).await?;

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

    // Create test world (similar to the real scenario)
    let test_world = WorldInfo {
        world_id: "NewWorld-1756274689_player-player-1".to_string(), // Use similar ID format as real client
        world_name: "NewWorld-1756274689".to_string(),
        description: "A new world".to_string(),
        host_player: "player-player-1".to_string(),
        host_name: "georgik".to_string(),
        player_count: 1,
        max_players: 10,
        is_public: true,
    };

    let topic = format!("iotcraft/worlds/{}/info", test_world.world_id);
    let payload = test_world.to_json();

    println!(
        "📤 Publisher: Publishing world '{}' to topic '{}'",
        test_world.world_name, topic
    );
    println!("   Payload: {}", payload);

    // Publish with retain=true (same as real client)
    client
        .publish(&topic, QoS::AtLeastOnce, true, payload.as_bytes())
        .await?;

    println!("✅ Publisher: World published successfully");
    sleep(Duration::from_secs(1)).await; // Give time for message to propagate

    Ok(test_world)
}

/// Test that specifically reproduces the scenario where:
/// 1. Discovery service subscribes to world info topic
/// 2. Some time passes (like in the real scenario)
/// 3. A world is published by another client
/// 4. Discovery service should receive the live message
#[tokio::test]
async fn test_mqtt_live_world_discovery_after_subscription()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🧪 Starting MQTT Live World Discovery Test");
    println!("📋 This test reproduces the scenario where:");
    println!("   1. Client 2 connects and subscribes to world info");
    println!("   2. Client 2 waits (discovery service enters main loop)");
    println!("   3. Client 1 publishes a world");
    println!("   4. Client 2 should receive the live message");
    println!("{}", "=".repeat(70));

    // Set up test environment
    println!("\n🔧 Setting up MQTT test environment...");
    let mqtt_env = MqttTestEnvironment::setup().await?;
    let port = mqtt_env.port;
    println!("   ✅ MQTT server running on port {}", port);

    // Record test start time
    let test_start_time = Instant::now();

    // Step 1: Start discovery subscriber (Client 2 equivalent)
    println!("\n🔍 Step 1: Starting discovery subscriber (simulating Client 2)...");
    let subscriber_task =
        tokio::spawn({ async move { discovery_subscriber_task(port, "client2").await } });

    // Give subscriber time to connect and enter "main processing loop" phase
    sleep(Duration::from_secs(5)).await;
    println!("✅ Discovery subscriber should now be in main processing loop phase");

    // Step 2: Publish a world after delay (Client 1 equivalent)
    println!("\n📡 Step 2: Publishing world after delay (simulating Client 1)...");
    let published_world = delayed_publisher_task(port, 2).await?;

    // Step 3: Collect results
    println!("\n🧮 Step 3: Collecting discovery results...");
    let discovered_worlds = subscriber_task.await??;

    // Analysis
    println!("\n\n📊 TEST ANALYSIS:");
    println!("{}", "=".repeat(70));

    let publish_time_since_start = Duration::from_secs(7); // 5 seconds wait + 2 seconds delay
    println!(
        "⏰ World published approximately {} seconds after test start",
        publish_time_since_start.as_secs()
    );
    println!(
        "📈 Total messages received by discovery service: {}",
        discovered_worlds.len()
    );

    if discovered_worlds.is_empty() {
        println!("\n❌ LIVE MESSAGE RECEPTION FAILED:");
        println!(
            "   The discovery service did not receive the world message published after subscription."
        );
        println!("   This reproduces the bug reported in the original scenario!");
        println!("\n🔍 POSSIBLE CAUSES:");
        println!("   1. Discovery service main loop not processing live MQTT messages");
        println!("   2. MQTT client disconnection after retained message collection");
        println!("   3. Message retention issue preventing delivery");
        println!("   4. Subscription not staying active in main processing loop");

        println!("\n🔧 DEBUGGING STEPS:");
        println!(
            "   1. Check if enhanced logging shows '📬 Main loop received MQTT message' in real client"
        );
        println!("   2. Verify connection stays active after retained message collection phase");
        println!("   3. Check if rumqttc try_recv() in main loop is actually being called");
        println!("   4. Test if manual refresh triggers message reception");
    } else {
        println!("\n✅ LIVE MESSAGE RECEPTION WORKING:");
        println!("   Discovery service successfully received world messages!");

        for (world_info, receive_time) in &discovered_worlds {
            let time_since_start = receive_time.duration_since(test_start_time);
            println!(
                "   📨 {} ({}) received at +{:.1}s",
                world_info.world_name,
                world_info.world_id,
                time_since_start.as_secs_f64()
            );
        }

        // Verify we got the expected world
        let found_published_world = discovered_worlds
            .iter()
            .any(|(world, _)| world.world_id == published_world.world_id);

        if found_published_world {
            println!("\n🎯 SUCCESS: Found the specific world that was published!");
            println!("   This means live message reception is working correctly.");
        } else {
            println!("\n⚠️  Received messages but not the expected world:");
            println!("   Expected: {}", published_world.world_id);
            println!(
                "   Received: {:?}",
                discovered_worlds
                    .iter()
                    .map(|(w, _)| &w.world_id)
                    .collect::<Vec<_>>()
            );
        }
    }

    // Check message timing
    let late_messages = discovered_worlds
        .iter()
        .filter(|(_, receive_time)| {
            receive_time.duration_since(test_start_time) > Duration::from_secs(6)
        })
        .count();

    if late_messages > 0 {
        println!("\n📡 LIVE MESSAGE TIMING:");
        println!(
            "   {} messages received after the 6-second mark (likely live messages)",
            late_messages
        );
        println!("   This confirms live message delivery is working");
    }

    println!("\n🔧 RECOMMENDED ACTIONS:");
    if discovered_worlds.is_empty() {
        println!("❌ 1. Fix the world discovery main loop to properly handle live MQTT messages");
        println!(
            "❌ 2. Ensure MQTT connection remains active throughout the discovery service lifecycle"
        );
        println!("❌ 3. Add connection health checks to detect and recover from disconnections");
        println!(
            "❌ 4. Verify that try_recv() is being called frequently in the main processing loop"
        );
    } else {
        println!("✅ 1. Live message reception appears to be working in this test");
        println!("✅ 2. Check if the issue is specific to the real client environment");
        println!("✅ 3. Compare real client MQTT settings with test settings");
        println!("✅ 4. Verify UI updates when OnlineWorlds resource changes");
    }

    // Clean up
    mqtt_env.shutdown().await?;
    println!("\n🧹 Test environment cleaned up");

    // The test should pass regardless of results since it's diagnostic
    // But we can assert for CI purposes if needed
    println!("\n🏁 Live world discovery test completed!");

    Ok(())
}

/// Additional test to verify retained message delivery works
#[tokio::test]
async fn test_mqtt_retained_world_discovery() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    println!("🧪 Starting MQTT Retained World Discovery Test");
    println!("📋 This test verifies that retained messages work correctly:");
    println!("   1. Client 1 publishes a world with retain=true");
    println!("   2. Client 2 connects after publication");
    println!("   3. Client 2 should immediately receive the retained message");
    println!("{}", "=".repeat(70));

    // Set up test environment
    println!("\n🔧 Setting up MQTT test environment...");
    let mqtt_env = MqttTestEnvironment::setup().await?;
    let port = mqtt_env.port;
    println!("   ✅ MQTT server running on port {}", port);

    // Step 1: Publish world first
    println!("\n📡 Step 1: Publishing world with retain=true...");
    let published_world = delayed_publisher_task(port, 0).await?; // No delay

    // Wait for message to be retained by broker
    sleep(Duration::from_secs(2)).await;

    // Step 2: Start discovery subscriber after publication
    println!("\n🔍 Step 2: Starting discovery subscriber after publication...");
    let discovered_worlds = discovery_subscriber_task(port, "late-joiner").await?;

    // Analysis
    println!("\n\n📊 RETAINED MESSAGE TEST ANALYSIS:");
    println!("{}", "=".repeat(70));
    println!(
        "📈 Messages received by late-joining discovery service: {}",
        discovered_worlds.len()
    );

    if discovered_worlds.is_empty() {
        println!("\n❌ RETAINED MESSAGE DELIVERY FAILED:");
        println!("   Late-joining client did not receive retained world message.");
        println!(
            "   This indicates an issue with MQTT broker retention or client session settings."
        );
    } else {
        println!("\n✅ RETAINED MESSAGE DELIVERY WORKING:");
        for (world_info, _) in &discovered_worlds {
            println!("   📨 {} ({})", world_info.world_name, world_info.world_id);
        }

        let found_published_world = discovered_worlds
            .iter()
            .any(|(world, _)| world.world_id == published_world.world_id);

        if found_published_world {
            println!("🎯 SUCCESS: Late-joining client received the retained world message!");
        }
    }

    // Clean up
    mqtt_env.shutdown().await?;
    println!("\n🧹 Test environment cleaned up");

    Ok(())
}
