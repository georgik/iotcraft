use rumqttc::{Client, Event, Incoming, MqttOptions, QoS};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_debug_mqtt_subscription() {
    println!("🧪 DEBUG: Testing MQTT subscription and message reception");
    println!("======================================================================");

    // Create publisher client
    let mut opts1 = MqttOptions::new("debug-publisher", "localhost", 1883);
    opts1.set_keep_alive(Duration::from_secs(30));

    let (client1, mut eventloop1) = Client::new(opts1, 10);

    // Create subscriber client
    let mut opts2 = MqttOptions::new("debug-subscriber", "localhost", 1883);
    opts2.set_keep_alive(Duration::from_secs(30));

    let (client2, mut eventloop2) = Client::new(opts2, 10);

    // Connect both clients
    tokio::spawn(async move {
        for notification in eventloop1.iter() {
            match notification {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Publisher connection error: {}", e);
                    break;
                }
            }
        }
    });

    let received_messages = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let received_messages_clone = received_messages.clone();

    // Subscriber task
    tokio::spawn(async move {
        let mut connected = false;

        for notification in eventloop2.iter() {
            match notification {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    println!("📡 Subscriber connected!");
                    connected = true;

                    // Subscribe to the exact topics the desktop client uses
                    let test_world_id = "TestWorld-Debug-123";
                    let topics = vec![
                        format!("iotcraft/worlds/{}/state/blocks/placed", test_world_id),
                        format!("iotcraft/worlds/{}/state/blocks/removed", test_world_id),
                        format!("iotcraft/worlds/{}/info", test_world_id),
                        format!("iotcraft/worlds/{}/data", test_world_id),
                    ];

                    for topic in &topics {
                        if let Err(e) = client2.subscribe(topic, QoS::AtLeastOnce) {
                            eprintln!("❌ Failed to subscribe to {}: {}", topic, e);
                        } else {
                            println!("✅ Subscribed to: {}", topic);
                        }
                    }
                }
                Ok(Event::Incoming(Incoming::Publish(p))) => {
                    println!(
                        "📨 Received message on topic: {} (retained: {})",
                        p.topic, p.retain
                    );
                    if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
                        println!("   Payload: {}", payload);
                    }
                    // Store for validation
                    if let Ok(mut messages) = received_messages_clone.lock() {
                        messages.push((p.topic.clone(), p.retain));
                    }
                }
                Ok(Event::Incoming(Incoming::SubAck(_))) => {
                    println!("✅ Subscription acknowledged");
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Subscriber connection error: {}", e);
                    break;
                }
            }
        }
    });

    // Wait for connections
    sleep(Duration::from_secs(2)).await;

    // Publish test messages
    let test_world_id = "TestWorld-Debug-123";

    // Publish world info
    let world_info = json!({
        "world_id": test_world_id,
        "world_name": "Debug Test World",
        "host_player": "debug-player",
        "host_name": "DebugHost",
        "player_count": 1,
        "max_players": 4,
        "is_public": true,
        "version": "1.0.0"
    });

    let topic = format!("iotcraft/worlds/{}/info", test_world_id);
    println!("📤 Publishing world info to: {}", topic);
    if let Err(e) = client1.publish(&topic, QoS::AtLeastOnce, true, world_info.to_string()) {
        eprintln!("❌ Failed to publish world info: {}", e);
    }

    sleep(Duration::from_millis(500)).await;

    // Publish block change
    let block_change = json!({
        "player_id": "debug-player-1",
        "player_name": "DebugPlayer1",
        "timestamp": 1234567890,
        "change": {
            "Placed": {
                "x": 10,
                "y": 5,
                "z": 10,
                "block_type": "Stone"
            }
        }
    });

    let topic = format!("iotcraft/worlds/{}/state/blocks/placed", test_world_id);
    println!("📤 Publishing block change to: {}", topic);
    if let Err(e) = client1.publish(&topic, QoS::AtLeastOnce, false, block_change.to_string()) {
        eprintln!("❌ Failed to publish block change: {}", e);
    }

    // Wait for message processing
    sleep(Duration::from_secs(3)).await;

    println!("\n🧮 DEBUG RESULTS:");
    println!("======================================================================");

    let messages = received_messages.lock().unwrap();
    if messages.is_empty() {
        println!("❌ NO MESSAGES RECEIVED - Check MQTT broker connection and topic matching");
    } else {
        println!("✅ Received {} messages:", messages.len());
        for (i, (topic, retain)) in messages.iter().enumerate() {
            println!("   {}. {} (retained: {})", i + 1, topic, retain);
        }
    }

    println!("🏁 Debug test completed!");
}
