#!/usr/bin/env rust
//! Test binary to reproduce mDNS blocking behavior
//!
//! This test demonstrates the issue where mDNS discovery can block
//! application termination when trying to connect to discovered services
//! that are not actually reachable.

use iotcraft_desktop_client::discovery::{
    discover_best_mqtt_service, discover_best_mqtt_service_with_connectivity_test,
};
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    info!("🧪 Testing mDNS discovery blocking behavior...");
    info!("💡 This test will demonstrate potential blocking issues");
    info!("🔍 Press Ctrl+C to test if the application can terminate gracefully");

    // Set up Ctrl+C handler
    let shutdown_signal = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install SIGINT handler");
        warn!("⚠️ Received Ctrl+C, attempting graceful shutdown...");
    };

    // Run discovery with different timeouts to test blocking behavior
    let discovery_task = async {
        for timeout_seconds in [2, 5] {
            info!(
                "🔍 Testing basic discovery with {} second timeout...",
                timeout_seconds
            );

            let start = std::time::Instant::now();
            match discover_best_mqtt_service(timeout_seconds).await {
                Ok(Some(service)) => {
                    info!(
                        "✅ Discovered service: {} at {}",
                        service.name,
                        service.broker_address()
                    );
                    info!("⚠️ Note: This service may not be reachable - that's the blocking issue");
                }
                Ok(None) => {
                    info!("ℹ️ No MQTT services discovered");
                }
                Err(e) => {
                    warn!("❌ Discovery failed: {}", e);
                }
            }

            let elapsed = start.elapsed();
            info!(
                "⏱️ Basic discovery completed in {:.2}s",
                elapsed.as_secs_f64()
            );
        }

        info!("🔄 Now testing improved discovery with connectivity testing...");

        for timeout_seconds in [2, 5] {
            info!(
                "🔍 Testing improved discovery with {} second timeout...",
                timeout_seconds
            );

            let start = std::time::Instant::now();
            match discover_best_mqtt_service_with_connectivity_test(timeout_seconds, 2).await {
                Ok(Some(service)) => {
                    info!(
                        "✅ Discovered reachable service: {} at {}",
                        service.name,
                        service.broker_address()
                    );
                    info!("🔌 This service has been verified as reachable");
                }
                Ok(None) => {
                    info!("ℹ️ No reachable MQTT services found");
                }
                Err(e) => {
                    warn!("❌ Improved discovery failed: {}", e);
                }
            }

            let elapsed = start.elapsed();
            info!(
                "⏱️ Improved discovery completed in {:.2}s",
                elapsed.as_secs_f64()
            );
        }

        info!("🎯 Discovery testing completed");
    };

    // Race between discovery and shutdown signal
    tokio::select! {
        _ = discovery_task => {
            info!("✅ Discovery completed normally");
        }
        _ = shutdown_signal => {
            warn!("🛑 Shutdown signal received during discovery");
            info!("🧪 Testing if cleanup happens properly...");

            // Give some time to see if cleanup works
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            error!("❌ If you see this message, the application handled shutdown gracefully");
        }
    }

    info!("🏁 Test completed - application should exit cleanly now");
    Ok(())
}
