#!/usr/bin/env rust
//! Test binary for mDNS discovery functionality

use iotcraft_desktop_client::discovery::discover_best_mqtt_service;
use tracing::{info, warn};
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    info!("🔍 Testing IoTCraft mDNS discovery...");

    // Test discovery with 5 second timeout
    match discover_best_mqtt_service(5).await {
        Ok(Some(service)) => {
            info!("✅ Successfully discovered MQTT service!");
            info!("   Name: {}", service.name);
            info!("   Address: {}", service.broker_address());
            if let Some(ref service_type) = service.service_type {
                info!("   Service Type: {}", service_type);
            }
            if let Some(ref version) = service.version {
                info!("   Version: {}", version);
            }
            if let Some(ref features) = service.features {
                info!("   Features: {}", features);
            }
            info!(
                "   Priority: {} ({})",
                service.priority,
                if service.is_iotcraft_service() {
                    "IoTCraft"
                } else {
                    "Generic MQTT"
                }
            );
        }
        Ok(None) => {
            warn!("⚠️ No MQTT services found via mDNS");
        }
        Err(e) => {
            warn!("❌ mDNS discovery failed: {}", e);
        }
    }

    info!("🏁 Discovery test completed");
    Ok(())
}
