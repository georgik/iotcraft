use rumqttc::{AsyncClient, MqttOptions, QoS};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_simple_retained_messages() {
    println!("🧪 Testing simple retained message behavior");

    // Step 1: Connect and check for existing retained messages (simulating desktop client behavior)
    let mut mqttoptions = MqttOptions::new("desktop-client-test", "localhost", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(10));
    mqttoptions.set_clean_session(false); // Important: persistent session to receive retained

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    // Subscribe to the topic
    println!("🔔 Subscribing to iotcraft/worlds/+/info...");
    client
        .subscribe("iotcraft/worlds/+/info", QoS::AtLeastOnce)
        .await
        .unwrap();

    let mut received_count = 0;

    // Process events for a short time to see retained messages
    let timeout_duration = Duration::from_secs(3);
    let start_time = tokio::time::Instant::now();

    while start_time.elapsed() < timeout_duration {
        match tokio::time::timeout(Duration::from_millis(100), eventloop.poll()).await {
            Ok(Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(_)))) => {
                println!("📡 Connected to MQTT broker");
            }
            Ok(Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish)))) => {
                println!(
                    "📨 Received message on '{}' [retain: {}]",
                    publish.topic, publish.retain
                );
                println!("   Payload size: {} bytes", publish.payload.len());
                received_count += 1;
            }
            Ok(Ok(rumqttc::Event::Incoming(rumqttc::Packet::SubAck(_)))) => {
                println!("✅ Subscription confirmed");
            }
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                println!("❌ Connection error: {:?}", e);
                break;
            }
            Err(_) => {
                // Timeout - continue
            }
        }
    }

    println!("🎯 Total messages received: {}", received_count);

    if received_count == 0 {
        println!("⚠️  No retained messages found. This suggests either:");
        println!("   1. No worlds are currently published with retain=true");
        println!("   2. The broker is not storing retained messages properly");
        println!("   3. There's a timing issue with retained message delivery");
    } else {
        println!("✅ Found {} retained messages", received_count);
    }
}
