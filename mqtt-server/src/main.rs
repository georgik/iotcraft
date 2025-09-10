use clap::Parser;
use config::Config as FileConfig;
use config::File as ConfigFile;
use rumqttd::{Broker, Config};
use std::thread;
use tracing::info;
use tracing_subscriber;

#[derive(Parser, Debug)]
#[command(name = "iotcraft-mqtt-server")]
#[command(about = "IoTCraft MQTT Server with configurable port")]
struct Args {
    /// Port to bind the MQTT server to (overrides config file)
    #[arg(short, long)]
    port: Option<u16>,
}

fn main() {
    // Parse command line arguments
    let args = Args::parse();

    // initialize logging
    tracing_subscriber::fmt().init();

    // load broker configuration

    // If port is specified, create a dynamic configuration
    let rumq_cfg: Config = if let Some(port) = args.port {
        info!("Using dynamic port configuration: {}", port);

        // Create a minimal dynamic configuration
        let config_str = format!(
            r#"
            id = 0

            [router]
            id = 0
            max_connections = 1000
            max_outgoing_packet_count = 200
            max_segment_size = 104857600
            max_segment_count = 10

            [v4.1]
            name = "v4-1"
            listen = "0.0.0.0:{}"
            next_connection_delay_ms = 1
            [v4.1.connections]
            connection_timeout_ms = 60000
            max_payload_size = 1048576
            max_inflight_count = 100
            dynamic_filters = true

            [console]
            listen = "0.0.0.0:3030"
            "#,
            port
        );

        FileConfig::builder()
            .add_source(config::File::from_str(
                &config_str,
                config::FileFormat::Toml,
            ))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap()
    } else {
        info!("Using static configuration from rumqttd.toml");
        FileConfig::builder()
            .add_source(ConfigFile::with_name("rumqttd.toml"))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap()
    };

    let mut broker = Broker::new(rumq_cfg);

    // Log the IP addresses on which the server is binding
    info!("MQTT Server configured to bind to 0.0.0.0, available IP addresses:");
    if let Ok(ifaces) = if_addrs::get_if_addrs() {
        for iface in &ifaces {
            if !iface.is_loopback() {
                info!("  - {} ({})", iface.addr.ip(), iface.name);
            }
        }
        // Also log loopback for completeness
        for iface in &ifaces {
            if iface.is_loopback() {
                info!("  - {} ({}) [loopback]", iface.addr.ip(), iface.name);
            }
        }
    } else {
        info!("Could not retrieve network interfaces.");
    }

    // create a link to receive broker notifications
    let (mut _link_tx, mut link_rx) = broker.link("mqtt-server").unwrap();

    // run the broker in a background thread
    thread::spawn(move || {
        broker.start().unwrap();
    });

    // process notifications (blocking on recv)
    loop {
        match link_rx.recv() {
            Ok(Some(notification)) => {
                info!("Broker notification: {:?}", notification);
            }
            Ok(None) => {
                // link closed, exit loop
                break;
            }
            Err(err) => {
                // channel error, log and exit
                info!("LinkRx recv error: {:?}", err);
                break;
            }
        }
    }
}
