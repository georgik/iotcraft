use rumqttc::{Client, MqttOptions, QoS};
use rumqttc::{Event, Outgoing};
use std::time::Duration;
use std::{env, thread};

fn main() {
    // Read payload from command-line argument
    let payload = env::args().nth(1).expect("Usage: mqtt-client <message>");

    // Configure MQTT options: client ID, broker address, keep alive
    let mut mqttoptions = MqttOptions::new("rust-mqtt-client", "127.0.0.1", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    // Create a synchronous client and connection
    let (mut client, mut connection) = Client::new(mqttoptions, 10);

    // Publish the message to the topic
    client
        .publish(
            "home/cube/light",
            QoS::AtMostOnce,
            false,
            payload.as_bytes(),
        )
        .expect("Failed to publish");

    // Drive the event loop until our publish packet is sent
    for notification in connection.iter() {
        println!("Notification = {:?}", notification);
        if let Ok(Event::Outgoing(Outgoing::Publish(_))) = notification {
            break;
        }
    }
    // give the broker a moment to process
    thread::sleep(Duration::from_millis(100));

    println!("Message published successfully.");
}
