//! MQTT Observer - Subscribe to all MQTT topics and output messages
//! 
//! This binary is designed to replace mosquitto_sub in the mcplay scenario runner.
//! It subscribes to all topics (#) and outputs messages in a format that mcplay can consume.

use rumqttc::{AsyncClient, MqttOptions, QoS, Event, Packet};
use std::time::Duration;
use std::{env, process};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    
    // Parse command line arguments similar to mosquitto_sub
    let mut host = "localhost".to_string();
    let mut port = 1883u16;
    let mut topic = "#".to_string(); // Default to all topics
    let mut client_id = "mqtt-observer".to_string();
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" => {
                if i + 1 < args.len() {
                    host = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: -h requires a host argument");
                    process::exit(1);
                }
            }
            "-p" => {
                if i + 1 < args.len() {
                    port = args[i + 1].parse().unwrap_or_else(|_| {
                        eprintln!("Error: Invalid port number");
                        process::exit(1);
                    });
                    i += 2;
                } else {
                    eprintln!("Error: -p requires a port argument");
                    process::exit(1);
                }
            }
            "-t" => {
                if i + 1 < args.len() {
                    topic = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: -t requires a topic argument");
                    process::exit(1);
                }
            }
            "-i" => {
                if i + 1 < args.len() {
                    client_id = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: -i requires a client ID argument");
                    process::exit(1);
                }
            }
            "--help" => {
                println!("mqtt-observer - MQTT topic observer");
                println!("Usage: mqtt-observer [OPTIONS]");
                println!("Options:");
                println!("  -h <host>       MQTT broker host (default: localhost)");
                println!("  -p <port>       MQTT broker port (default: 1883)");
                println!("  -t <topic>      Topic pattern to subscribe to (default: #)");
                println!("  -i <client_id>  Client ID (default: mqtt-observer)");
                println!("  --help          Show this help message");
                process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                process::exit(1);
            }
        }
    }
    
    // Configure MQTT options
    let mut mqttoptions = MqttOptions::new(&client_id, &host, port);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    
    // Create an async client and event loop
    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    
    // Subscribe to the specified topic pattern
    if let Err(e) = client.subscribe(&topic, QoS::AtMostOnce).await {
        eprintln!("Failed to subscribe to topic '{}': {}", topic, e);
        process::exit(1);
    }
    
    // Output subscription confirmation to stderr so it doesn't interfere with message output
    eprintln!("mqtt-observer: Connected to {}:{}, subscribed to '{}'", host, port, topic);
    
    // Process incoming messages using async event loop
    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Packet::Publish(publish))) => {
                // Output message in a format similar to mosquitto_sub
                // Format: topic payload
                let payload = String::from_utf8_lossy(&publish.payload);
                println!("{} {}", publish.topic, payload);
                
                // Flush stdout to ensure immediate output
                use std::io::{self, Write};
                let _ = io::stdout().flush();
            }
            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                eprintln!("mqtt-observer: Connection acknowledged");
            }
            Ok(Event::Incoming(Packet::SubAck(_))) => {
                eprintln!("mqtt-observer: Subscription acknowledged");
            }
            Ok(Event::Outgoing(_)) => {
                // Ignore outgoing events
            }
            Ok(_) => {
                // Ignore other events
            }
            Err(e) => {
                eprintln!("mqtt-observer: Connection error: {}", e);
                // With async, we can just continue the loop - no sleep needed
                // The event loop will handle reconnection automatically
                continue;
            }
        }
    }
}
