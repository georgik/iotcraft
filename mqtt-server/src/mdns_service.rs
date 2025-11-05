//! mDNS service registration for IoTCraft MQTT server discovery

use anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use tracing::{error, info, warn};

/// mDNS service for MQTT broker discovery
pub struct MdnsService {
    daemon: ServiceDaemon,
    service_name: String,
    mqtt_service_type: String,
    iotcraft_service_type: String,
}

impl MdnsService {
    /// Create a new mDNS service
    pub fn new() -> Result<Self> {
        let daemon = ServiceDaemon::new()
            .map_err(|e| anyhow::anyhow!("Failed to create mDNS daemon: {}", e))?;

        // Use a unique service name that doesn't conflict with system hostname
        // This prevents macOS from thinking there's another computer on the network
        let service_name = "iotcraft-mqtt-server".to_string();

        let mqtt_service_type = "_mqtt._tcp.local.".to_string();
        let iotcraft_service_type = "_iotcraft._tcp.local.".to_string();

        Ok(Self {
            daemon,
            service_name,
            mqtt_service_type,
            iotcraft_service_type,
        })
    }

    /// Register the MQTT broker for discovery
    pub async fn register(&self, port: u16, enable_mdns: bool) -> Result<()> {
        if !enable_mdns {
            info!("📡 mDNS service announcement disabled");
            return Ok(());
        }

        // Get server version
        let version = env!("CARGO_PKG_VERSION");

        // Get system hostname for mDNS registration (not for service name)
        // We use the actual system hostname to avoid conflicts
        let hostname = {
            let host = hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| "localhost".to_string());

            // Ensure hostname ends with .local. as required by mDNS spec
            let base_host = host.trim_end_matches(".local").trim_end_matches(".");
            format!("{}.local.", base_host)
        };

        // Get local IP addresses for all interfaces
        let addresses = self.get_local_addresses();
        if addresses.is_empty() {
            return Err(anyhow::anyhow!(
                "No network interfaces found for mDNS registration"
            ));
        }

        info!(
            "🔍 Found {} network addresses for mDNS: {:?}",
            addresses.len(),
            addresses
        );

        // Register standard MQTT service
        match self
            .register_mqtt_service(&hostname, &addresses, port, version)
            .await
        {
            Ok(_) => info!("📡 Standard MQTT service registered successfully"),
            Err(e) => warn!("⚠️ Failed to register MQTT service: {}", e),
        }

        // Register IoTCraft-specific service with additional metadata
        match self
            .register_iotcraft_service(&hostname, &addresses, port, version)
            .await
        {
            Ok(_) => info!("📡 IoTCraft-specific service registered successfully"),
            Err(e) => warn!("⚠️ Failed to register IoTCraft service: {}", e),
        }

        // Log clean hostname (without trailing dot) for clarity
        let clean_hostname = hostname.trim_end_matches('.');
        info!(
            "🔍 MQTT broker discoverable at: {}:{}",
            clean_hostname, port
        );
        info!("📡 mDNS services registered for IoTCraft MQTT server");

        Ok(())
    }

    /// Register standard MQTT service
    async fn register_mqtt_service(
        &self,
        hostname: &str,
        addresses: &[std::net::IpAddr],
        port: u16,
        version: &str,
    ) -> Result<()> {
        let service_info = ServiceInfo::new(
            &self.mqtt_service_type,
            &self.service_name,
            hostname,
            addresses,
            port,
            &[
                ("version", version),
                ("service", "iotcraft-mqtt-server"),
                ("protocol", "MQTT"),
            ][..],
        )
        .map_err(|e| anyhow::anyhow!("Failed to create MQTT service info: {}", e))?;

        self.daemon
            .register(service_info)
            .map_err(|e| anyhow::anyhow!("Failed to register MQTT service: {}", e))?;

        Ok(())
    }

    /// Register IoTCraft-specific service with additional metadata
    async fn register_iotcraft_service(
        &self,
        hostname: &str,
        addresses: &[std::net::IpAddr],
        port: u16,
        version: &str,
    ) -> Result<()> {
        let service_info = ServiceInfo::new(
            &self.iotcraft_service_type,
            &self.service_name,
            hostname,
            addresses,
            port,
            &[
                ("version", version),
                ("service", "iotcraft-mqtt-server"),
                ("description", "IoTCraft MQTT Broker"),
                ("features", "mqtt,desktop,voxel-world"),
                ("protocol", "MQTT"),
                ("type", "mqtt-broker"),
            ][..],
        )
        .map_err(|e| anyhow::anyhow!("Failed to create IoTCraft service info: {}", e))?;

        self.daemon
            .register(service_info)
            .map_err(|e| anyhow::anyhow!("Failed to register IoTCraft service: {}", e))?;

        Ok(())
    }

    /// Unregister the mDNS services
    pub fn unregister(&self) -> Result<()> {
        // Create the full service names for unregistration
        let mqtt_service_name = format!("{}.{}", self.service_name, self.mqtt_service_type);
        let iotcraft_service_name = format!("{}.{}", self.service_name, self.iotcraft_service_type);

        // Unregister MQTT service
        if let Err(e) = self.daemon.unregister(&mqtt_service_name) {
            warn!("⚠️ Failed to unregister MQTT service: {}", e);
        } else {
            info!("📡 MQTT service unregistered");
        }

        // Unregister IoTCraft service
        if let Err(e) = self.daemon.unregister(&iotcraft_service_name) {
            warn!("⚠️ Failed to unregister IoTCraft service: {}", e);
        } else {
            info!("📡 IoTCraft service unregistered");
        }

        Ok(())
    }

    /// Get local IP addresses for mDNS registration
    fn get_local_addresses(&self) -> Vec<std::net::IpAddr> {
        match if_addrs::get_if_addrs() {
            Ok(interfaces) => {
                interfaces
                    .into_iter()
                    .filter_map(|iface| {
                        // Skip loopback and down interfaces
                        if iface.is_loopback() {
                            return None;
                        }

                        // Include both IPv4 and IPv6 addresses
                        match iface.addr.ip() {
                            ip @ (std::net::IpAddr::V4(_) | std::net::IpAddr::V6(_)) => {
                                info!("🔍 Network interface {}: {}", iface.name, ip);
                                Some(ip)
                            }
                        }
                    })
                    .collect()
            }
            Err(e) => {
                error!("⚠️ Failed to get network interfaces: {}", e);
                // Fallback to localhost if we can't get interfaces
                vec![std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))]
            }
        }
    }

    /// Shutdown the mDNS daemon
    pub fn shutdown(self) -> Result<()> {
        // Unregister services first
        let _ = self.unregister();

        // Shutdown the daemon
        self.daemon
            .shutdown()
            .map_err(|e| anyhow::anyhow!("Failed to shutdown mDNS daemon: {}", e))?;

        info!("🛑 mDNS daemon shut down");
        Ok(())
    }
}
