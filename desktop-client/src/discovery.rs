//! mDNS service discovery for IoTCraft MQTT brokers

use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info, warn};

#[cfg(not(target_arch = "wasm32"))]
use mdns_sd::{ServiceDaemon, ServiceEvent};

#[cfg(not(target_arch = "wasm32"))]
use tokio::net::TcpStream;
#[cfg(not(target_arch = "wasm32"))]
use tokio::time::timeout;

/// Discovered IoTCraft MQTT service information
#[derive(Debug, Clone)]
pub struct DiscoveredMqttService {
    pub name: String,
    pub hostname: String,
    pub ip: std::net::IpAddr,
    pub port: u16,
    pub version: Option<String>,
    pub service_type: Option<String>,
    pub description: Option<String>,
    pub features: Option<String>,
    pub priority: u8, // Higher is better (0=generic MQTT, 1=IoTCraft service)
}

impl DiscoveredMqttService {
    /// Get the broker address as host:port string using IP address
    pub fn broker_address(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }

    /// Get the broker address as hostname:port string (alternative to IP)
    pub fn broker_hostname_address(&self) -> String {
        format!("{}:{}", self.hostname, self.port)
    }

    /// Check if this is an IoTCraft-specific service
    pub fn is_iotcraft_service(&self) -> bool {
        self.priority > 0
    }
}

/// Discover IoTCraft MQTT services on the local network using mDNS
#[cfg(not(target_arch = "wasm32"))]
pub async fn discover_mqtt_services(timeout_secs: u64) -> Result<Vec<DiscoveredMqttService>> {
    info!(
        "🔍 Starting mDNS discovery for MQTT services (timeout: {}s)",
        timeout_secs
    );

    let mdns =
        ServiceDaemon::new().map_err(|e| anyhow::anyhow!("Failed to create mDNS daemon: {}", e))?;

    let mut services = HashMap::new();
    let timeout = tokio::time::Duration::from_secs(timeout_secs);

    // Browse for both IoTCraft-specific and generic MQTT services
    let iotcraft_receiver = mdns
        .browse("_iotcraft._tcp.local.")
        .map_err(|e| anyhow::anyhow!("Failed to start mDNS browse for IoTCraft services: {}", e))?;

    let mqtt_receiver = mdns
        .browse("_mqtt._tcp.local.")
        .map_err(|e| anyhow::anyhow!("Failed to start mDNS browse for MQTT services: {}", e))?;

    info!("🔍 Browsing for _iotcraft._tcp.local and _mqtt._tcp.local services...");

    let start_time = tokio::time::Instant::now();

    // Listen for mDNS events with timeout
    while start_time.elapsed() < timeout {
        let remaining_time = timeout - start_time.elapsed();

        tokio::select! {
            // Handle IoTCraft-specific services (higher priority)
            event_result = iotcraft_receiver.recv_async() => {
                match event_result {
                    Ok(event) => {
                        if let Some(service) = handle_service_event(event, 1).await {
                            debug!("🎯 Found IoTCraft service: {}", service.name);
                            services.insert(service.name.clone(), service);
                        }
                    }
                    Err(e) => {
                        warn!("⚠️ IoTCraft mDNS receiver error: {}", e);
                        // Break on receiver error to prevent blocking
                        break;
                    }
                }
            }

            // Handle generic MQTT services (lower priority)
            event_result = mqtt_receiver.recv_async() => {
                match event_result {
                    Ok(event) => {
                        if let Some(service) = handle_service_event(event, 0).await {
                            // Only add if we don't already have a higher-priority service with the same name
                            if !services.contains_key(&service.name) {
                                debug!("🔍 Found MQTT service: {}", service.name);
                                services.insert(service.name.clone(), service);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("⚠️ MQTT mDNS receiver error: {}", e);
                        // Break on receiver error to prevent blocking
                        break;
                    }
                }
            }

            // Timeout
            _ = tokio::time::sleep(remaining_time) => {
                info!("🕐 mDNS discovery timeout reached ({} seconds)", timeout_secs);
                break;
            }
        }
    }

    // Properly stop browsing and cleanup resources
    info!("🧹 Cleaning up mDNS resources...");
    if let Err(e) = mdns.stop_browse("_iotcraft._tcp.local.") {
        debug!("⚠️ Error stopping IoTCraft mDNS browse: {}", e);
    }
    if let Err(e) = mdns.stop_browse("_mqtt._tcp.local.") {
        debug!("⚠️ Error stopping MQTT mDNS browse: {}", e);
    }

    // Explicit shutdown of the mDNS daemon with timeout
    info!("🔄 Shutting down mDNS daemon...");
    match tokio::time::timeout(
        Duration::from_secs(2), // Give 2 seconds for graceful shutdown
        tokio::task::spawn_blocking(move || {
            if let Err(e) = mdns.shutdown() {
                debug!("⚠️ Error shutting down mDNS daemon: {}", e);
            }
        }),
    )
    .await
    {
        Ok(_) => {
            info!("✅ mDNS daemon shutdown successfully");
        }
        Err(_) => {
            warn!("⚠️ mDNS daemon shutdown timed out, proceeding anyway");
        }
    }

    let discovered_services: Vec<DiscoveredMqttService> = services.into_values().collect();

    info!(
        "✅ Discovered {} MQTT service(s)",
        discovered_services.len()
    );
    for service in &discovered_services {
        info!(
            "  📍 {}: {} ({})",
            service.name,
            service.broker_address(),
            if service.is_iotcraft_service() {
                "IoTCraft"
            } else {
                "Generic MQTT"
            }
        );
    }

    Ok(discovered_services)
}

#[cfg(not(target_arch = "wasm32"))]
async fn handle_service_event(event: ServiceEvent, priority: u8) -> Option<DiscoveredMqttService> {
    match event {
        ServiceEvent::ServiceResolved(info) => {
            debug!("🔍 Found service: {}", info.get_fullname());

            // Parse TXT records for additional metadata
            let mut version = None;
            let mut service_type = None;
            let mut description = None;
            let mut features = None;

            let properties = info.get_properties();
            for property in properties.iter() {
                let property_string = format!("{}", property);
                if let Some((key, value)) = property_string.split_once('=') {
                    match key {
                        "version" => version = Some(value.to_string()),
                        "service" => service_type = Some(value.to_string()),
                        "description" => description = Some(value.to_string()),
                        "features" => features = Some(value.to_string()),
                        _ => {}
                    }
                }
            }

            // Get the first available IP address
            let addresses = info.get_addresses();
            if let Some(scoped_ip) = addresses.iter().next() {
                // Extract IP address from ScopedIp - ScopedIp wraps an IpAddr
                let ip = match scoped_ip {
                    mdns_sd::ScopedIp::V4(scoped_ipv4) => std::net::IpAddr::V4(*scoped_ipv4.addr()),
                    mdns_sd::ScopedIp::V6(scoped_ipv6) => std::net::IpAddr::V6(*scoped_ipv6.addr()),
                    _ => {
                        warn!("⚠️ Unsupported IP address type: {:?}", scoped_ip);
                        return None; // Skip this service
                    }
                };
                // Clean hostname by removing trailing dot (mDNS spec requires it but it breaks network connections)
                let raw_hostname = info.get_hostname().to_string();
                let clean_hostname = raw_hostname.trim_end_matches('.').to_string();

                let service = DiscoveredMqttService {
                    name: clean_hostname.clone(),
                    hostname: clean_hostname,
                    ip,
                    port: info.get_port(),
                    version,
                    service_type,
                    description,
                    features,
                    priority,
                };

                debug!(
                    "✅ Resolved service: {} at {}:{}",
                    service.name, service.ip, service.port
                );
                Some(service)
            } else {
                warn!("⚠️ Service {} has no IP addresses", info.get_hostname());
                None
            }
        }
        ServiceEvent::SearchStarted(_) => {
            debug!("🔍 mDNS search started");
            None
        }
        ServiceEvent::SearchStopped(_) => {
            debug!("🔍 mDNS search stopped");
            None
        }
        _ => None,
    }
}

/// Discover MQTT services with intelligent service selection
pub async fn discover_best_mqtt_service(
    timeout_secs: u64,
) -> Result<Option<DiscoveredMqttService>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let services = discover_mqtt_services(timeout_secs).await?;
        Ok(select_best_service(services))
    }

    #[cfg(target_arch = "wasm32")]
    {
        warn!("🌐 mDNS discovery not supported in web browsers");
        Ok(None)
    }
}

/// Select the best available MQTT service based on priority and features
fn select_best_service(mut services: Vec<DiscoveredMqttService>) -> Option<DiscoveredMqttService> {
    if services.is_empty() {
        return None;
    }

    // Sort services by priority (higher is better), then by name for stability
    services.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then_with(|| a.name.cmp(&b.name))
    });

    // Prefer IoTCraft MQTT server over IoTCraft gateway
    let best_service = services
        .iter()
        .find(|service| service.service_type.as_deref() == Some("iotcraft-mqtt-server"))
        .or_else(|| {
            // Fallback to any IoTCraft service
            services
                .iter()
                .find(|service| service.is_iotcraft_service())
        })
        .or_else(|| {
            // Final fallback to any MQTT service
            services.first()
        })
        .cloned();

    if let Some(ref service) = best_service {
        info!(
            "🎯 Selected MQTT service: {} at {}",
            service.name,
            service.broker_address()
        );
        if let Some(ref service_type) = service.service_type {
            info!("   Service type: {}", service_type);
        }
        if let Some(ref features) = service.features {
            info!("   Features: {}", features);
        }
    }

    best_service
}

/// Test if a discovered MQTT service is actually reachable
#[cfg(not(target_arch = "wasm32"))]
async fn test_service_connectivity(service: &DiscoveredMqttService, timeout_secs: u64) -> bool {
    debug!("🔌 Testing connectivity to {}", service.broker_address());

    let address = format!("{}:{}", service.ip, service.port);
    let connect_timeout = Duration::from_secs(timeout_secs);

    match timeout(connect_timeout, TcpStream::connect(&address)).await {
        Ok(Ok(_)) => {
            debug!("✅ Successfully connected to {}", service.broker_address());
            true
        }
        Ok(Err(e)) => {
            debug!(
                "❌ Failed to connect to {}: {}",
                service.broker_address(),
                e
            );
            false
        }
        Err(_) => {
            debug!(
                "⏰ Connection timeout to {} after {}s",
                service.broker_address(),
                timeout_secs
            );
            false
        }
    }
}

/// Discover MQTT services and test their connectivity
#[cfg(not(target_arch = "wasm32"))]
pub async fn discover_mqtt_services_with_connectivity_test(
    discovery_timeout_secs: u64,
    connect_timeout_secs: u64,
) -> Result<Vec<DiscoveredMqttService>> {
    let services = discover_mqtt_services(discovery_timeout_secs).await?;
    let mut reachable_services = Vec::new();

    for service in services {
        if test_service_connectivity(&service, connect_timeout_secs).await {
            reachable_services.push(service);
        } else {
            warn!(
                "🚫 Skipping unreachable service: {} at {}",
                service.name,
                service.broker_address()
            );
        }
    }

    Ok(reachable_services)
}

/// Discover MQTT services with connectivity test and return the best one
#[cfg(not(target_arch = "wasm32"))]
pub async fn discover_best_mqtt_service_with_connectivity_test(
    discovery_timeout_secs: u64,
    connect_timeout_secs: u64,
) -> Result<Option<DiscoveredMqttService>> {
    let services =
        discover_mqtt_services_with_connectivity_test(discovery_timeout_secs, connect_timeout_secs)
            .await?;
    Ok(select_best_service(services))
}
