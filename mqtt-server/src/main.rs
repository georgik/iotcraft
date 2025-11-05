use clap::Parser;
use config::Config as FileConfig;
use config::File as ConfigFile;
use rumqttd::{Broker, Config};
use std::thread;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber;

mod mdns_service;
use mdns_service::MdnsService;

#[derive(Parser, Debug)]
#[command(name = "iotcraft-mqtt-server")]
#[command(about = "IoTCraft MQTT Server with configurable port and mDNS discovery")]
struct Args {
    /// Port to bind the MQTT server to (overrides config file)
    #[arg(short, long)]
    port: Option<u16>,

    /// Enable mDNS service discovery (enabled by default for LAN networks)
    #[arg(long, default_value_t = true)]
    enable_mdns: bool,

    /// Disable mDNS service discovery (useful for online/cloud deployments)
    #[arg(long, conflicts_with = "enable_mdns")]
    disable_mdns: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Determine final mDNS setting (disable_mdns takes precedence)
    let enable_mdns = args.enable_mdns && !args.disable_mdns;

    // Initialize logging
    tracing_subscriber::fmt().init();

    info!(
        "Starting IoTCraft MQTT Server (mDNS: {})",
        if enable_mdns { "enabled" } else { "disabled" }
    );

    // Check if another instance might be running on the requested port
    if let Some(port) = args.port {
        if let Ok(_) = std::net::TcpStream::connect(format!("127.0.0.1:{}", port)) {
            warn!("⚠️ Another service appears to be running on port {}", port);
            warn!("💡 Consider using a different port or stopping the existing service");
        }
    }

    // Load broker configuration
    let (rumq_cfg, actual_port): (Config, u16) = if let Some(port) = args.port {
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
            listen = "0.0.0.0:3031"
            "#,
            port
        );

        let config = FileConfig::builder()
            .add_source(config::File::from_str(
                &config_str,
                config::FileFormat::Toml,
            ))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap();

        (config, port)
    } else {
        info!("Using static configuration from rumqttd.toml");
        let config = FileConfig::builder()
            .add_source(ConfigFile::with_name("rumqttd.toml"))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap();
        // Default port from rumqttd.toml is 1883
        (config, 1883)
    };

    // Initialize mDNS service before starting broker
    let mdns_service = if enable_mdns {
        match MdnsService::new() {
            Ok(service) => {
                info!("📡 mDNS service initialized");
                Some(service)
            }
            Err(e) => {
                warn!("⚠️ Failed to initialize mDNS service: {}", e);
                warn!("📡 Continuing without mDNS discovery");
                None
            }
        }
    } else {
        None
    };

    let mut broker = Broker::new(rumq_cfg);

    // Log the IP addresses on which the server is binding
    info!(
        "MQTT Server configured to bind to 0.0.0.0:{}, available IP addresses:",
        actual_port
    );
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

    // Create a link to receive broker notifications
    let (mut _link_tx, mut link_rx) = broker.link("mqtt-server").unwrap();

    // Create a shutdown signal for the broker thread
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);

    // Check if port is already in use before starting
    let port_check = std::net::TcpListener::bind(format!("0.0.0.0:{}", actual_port));
    if let Err(e) = port_check {
        error!("❌ Port {} is already in use: {}", actual_port, e);
        error!("💡 Try using a different port with --port <PORT> or stop the existing service");
        return Err(anyhow::anyhow!("Port {} already in use", actual_port));
    }
    drop(port_check); // Release the port for the broker

    // Run the broker in a background thread with shutdown handling
    let broker_handle = thread::spawn(move || {
        // Use a simple runtime for the broker thread
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            tokio::select! {
                _ = async {
                    if let Err(e) = broker.start() {
                        error!("❌ MQTT broker failed to start: {}", e);
                        error!("💡 This might be due to a port conflict or configuration issue");
                    }
                } => {
                    info!("🛑 Broker thread finished normally");
                }
                _ = shutdown_rx.recv() => {
                    info!("🛑 Broker thread received shutdown signal");
                    // Broker will be dropped here, which should trigger cleanup
                }
            }
        });
    });

    info!("🚀 MQTT broker started on port {}", actual_port);

    // Register mDNS services after broker is started
    if let Some(ref service) = mdns_service {
        if let Err(e) = service.register(actual_port, enable_mdns).await {
            error!("❌ Failed to register mDNS services: {}", e);
        }
    }

    info!("🌍 IoTCraft MQTT Server is ready and discoverable!");
    if enable_mdns {
        info!("🔍 Use 'dns-sd -B _iotcraft._tcp local' to verify mDNS discovery");
    }

    // Handle graceful shutdown on SIGINT/SIGTERM - simplified without broker notifications
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("🛑 Received SIGINT, shutting down gracefully...");
        }
        _ = async {
            #[cfg(unix)]
            {
                let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                    .expect("Failed to register SIGTERM handler");
                sigterm.recv().await;
            }
            #[cfg(not(unix))]
            {
                std::future::pending::<()>().await;
            }
        } => {
            info!("🛑 Received SIGTERM, shutting down gracefully...");
        }
    }

    // Signal broker thread to shutdown
    info!("🛑 Signaling broker thread to shutdown...");
    let _ = shutdown_tx.send(()).await;

    // Wait for broker thread to finish (with shorter timeout)
    let broker_shutdown = tokio::task::spawn_blocking(move || broker_handle.join());

    match tokio::time::timeout(tokio::time::Duration::from_secs(2), broker_shutdown).await {
        Ok(Ok(Ok(()))) => info!("👍 Broker thread shut down cleanly"),
        Ok(Ok(Err(e))) => warn!("⚠️ Broker thread panicked: {:?}", e),
        Ok(Err(e)) => warn!("⚠️ Failed to join broker thread: {:?}", e),
        Err(_) => {
            warn!("⚠️ Broker thread shutdown timed out after 2s, forcing exit");
            std::process::exit(1);
        }
    }

    // Cleanup mDNS service
    if let Some(service) = mdns_service {
        info!("📡 Shutting down mDNS service...");
        if let Err(e) = service.shutdown() {
            warn!("⚠️ Failed to shutdown mDNS service cleanly: {}", e);
        }
    }

    info!("👋 IoTCraft MQTT Server shutdown complete");

    // Force exit to ensure clean termination
    std::process::exit(0);
}
