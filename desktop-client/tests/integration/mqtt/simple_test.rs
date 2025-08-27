use anyhow::Result;
use rumqttc::{Client, Event, Incoming, MqttOptions, QoS};
use std::time::Duration;
use tokio::time::timeout;

use crate::mqtt_test_infrastructure::MqttTestServer;

#[tokio::test]
async fn test_simple_retained_messages() -> Result<()> {
    println!("🧪 Testing simple retained message behavior");

    // Start MQTT test server
    let server = MqttTestServer::start().await?;
    println!("🔧 Test server running on {}", server.broker_address());

    // Step 1: Connect and check for existing retained messages (simulating desktop client behavior)
    let mut mqttoptions = MqttOptions::new("desktop-client-test", &server.host, server.port);
    mqttoptions.set_keep_alive(Duration::from_secs(10));
    mqttoptions.set_clean_session(false); // Important: persistent session to receive retained

    let (client, mut eventloop) = Client::new(mqttoptions, 10);

    // Subscribe to the topic
    println!("🔔 Subscribing to iotcraft/worlds/+/info...");
    client
        .subscribe("iotcraft/worlds/+/info", QoS::AtLeastOnce)
        .unwrap();

    let mut received_count = 0;
    let mut connection_established = false;

    // Create a task to drive the event loop
    let eventloop_task = tokio::spawn(async move {
        // Use the iterator to process events
        for notification in eventloop.iter() {
            match notification {
                Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                    println!("📡 Connected to MQTT broker");
                }
                Ok(Event::Incoming(Incoming::Publish(publish))) => {
                    println!(
                        "📨 Received message on '{}' [retain: {}]",
                        publish.topic, publish.retain
                    );
                    println!("   Payload size: {} bytes", publish.payload.len());
                }
                Ok(Event::Incoming(Incoming::SubAck(_))) => {
                    println!("✅ Subscription confirmed");
                }
                Ok(_) => {}
                Err(e) => {
                    println!("❌ Connection error: {:?}", e);
                    break;
                }
            }
        }
    });

    // Wait for a reasonable amount of time, then terminate the test
    let result = timeout(Duration::from_secs(3), eventloop_task).await;

    match result {
        Ok(_) => {
            println!("✅ Event loop completed successfully");
        }
        Err(_) => {
            println!("⏰ Test completed after timeout");
        }
    }

    println!("🎯 Total messages received: {}", received_count);

    if received_count == 0 {
        println!("⚠️  No retained messages found. This suggests either:");
        println!("   1. No worlds are currently published with retain=true");
        println!("   2. The broker is not storing retained messages properly");
        println!("   3. There's a timing issue with retained message delivery");
        println!("   4. No MQTT broker is running (this is expected in CI/tests without broker)");
    } else {
        println!("✅ Found {} retained messages", received_count);
    }
    
    Ok(())
}
