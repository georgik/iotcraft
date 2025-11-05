use bevy::prelude::*;
use std::env;

#[cfg(not(target_arch = "wasm32"))]
use crate::discovery::{
    discover_best_mqtt_service, discover_best_mqtt_service_with_connectivity_test,
};

/// Configuration for MQTT broker connection
#[derive(Debug, Clone, Resource)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 1883,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mqtt_config_default() {
        let config = MqttConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 1883);
    }

    #[test]
    fn test_broker_address() {
        let config = MqttConfig {
            host: "brokerhost".to_string(),
            port: 8883,
        };
        assert_eq!(config.broker_address(), "brokerhost:8883");
    }

    #[test]
    fn test_mqtt_config_with_host_port_override() {
        let config = MqttConfig::from_env_with_override(Some("localhost:1883".to_string()));
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 1883);
        assert_eq!(config.broker_address(), "localhost:1883");
    }

    #[test]
    fn test_mqtt_config_with_host_only_override() {
        let config = MqttConfig::from_env_with_override(Some("testhost".to_string()));
        assert_eq!(config.host, "testhost");
        assert_eq!(config.port, 1883);
        assert_eq!(config.broker_address(), "testhost:1883");
    }

    #[test]
    fn test_mqtt_config_with_custom_port() {
        let config =
            MqttConfig::from_env_with_override(Some("broker.example.com:8883".to_string()));
        assert_eq!(config.host, "broker.example.com");
        assert_eq!(config.port, 8883);
        assert_eq!(config.broker_address(), "broker.example.com:8883");
    }

    #[test]
    fn test_mqtt_config_no_override() {
        let config = MqttConfig::from_env_with_override(None);
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 1883);
        assert_eq!(config.broker_address(), "localhost:1883");
    }
}

impl MqttConfig {
    /// Load configuration with mDNS discovery fallback
    /// Precedence: CLI args > Environment variables > mDNS discovery > localhost fallback
    pub async fn from_env_with_discovery(mqtt_server_override: Option<String>) -> Self {
        // If explicit override provided, use it
        if mqtt_server_override.is_some() {
            return Self::from_env_with_override(mqtt_server_override);
        }

        // Check environment variables first
        if env::var("MQTT_BROKER_HOST").is_ok() || env::var("MQTT_BROKER_PORT").is_ok() {
            return Self::from_env_with_override(None);
        }

        // Try mDNS discovery with connectivity testing
        #[cfg(not(target_arch = "wasm32"))]
        {
            info!(
                "🔍 No MQTT server specified, attempting mDNS discovery with connectivity test..."
            );
            match discover_best_mqtt_service_with_connectivity_test(3, 2).await {
                Ok(Some(service)) => {
                    info!(
                        "✅ Discovered reachable MQTT broker via mDNS: {}",
                        service.broker_address()
                    );
                    return MqttConfig {
                        host: service.ip.to_string(),
                        port: service.port,
                    };
                }
                Ok(None) => {
                    warn!("⚠️ No reachable MQTT services found via mDNS discovery");
                }
                Err(e) => {
                    warn!("⚠️ mDNS discovery with connectivity test failed: {}", e);
                }
            }
        }

        // Fallback to localhost
        info!("📍 Using localhost fallback for MQTT broker");
        Self::default()
    }

    /// Load configuration from CLI args, environment variables, or defaults
    /// CLI args take precedence over environment variables
    pub fn from_env_with_override(mqtt_server_override: Option<String>) -> Self {
        let (host, port) = if let Some(mqtt_server) = mqtt_server_override {
            // Parse host:port format if provided
            if let Some((host_part, port_part)) = mqtt_server.split_once(':') {
                let parsed_port = port_part.parse::<u16>().unwrap_or(1883);
                (host_part.to_string(), parsed_port)
            } else {
                // No port specified, use mqtt_server as host with default port
                (mqtt_server, 1883)
            }
        } else {
            // No override, use environment variables or defaults
            let host = env::var("MQTT_BROKER_HOST").unwrap_or_else(|_| "localhost".to_string());
            let port = env::var("MQTT_BROKER_PORT")
                .unwrap_or_else(|_| "1883".to_string())
                .parse()
                .unwrap_or(1883);
            (host, port)
        };

        Self { host, port }
    }

    /// Create web configuration using current host and URL parameters
    #[cfg(target_arch = "wasm32")]
    pub fn from_web_env() -> Self {
        use web_sys::window;

        let window = window().expect("Failed to get window object");
        let location = window.location();

        // Get current host from window.location.hostname
        let current_host = location
            .hostname()
            .unwrap_or_else(|_| "localhost".to_string());

        // Parse URL parameters manually from window.location.search
        let search_string = location.search().unwrap_or_default();
        let mut mqtt_server_override = None;

        if !search_string.is_empty() {
            // Remove leading '?' and split by '&'
            let params_str = search_string.trim_start_matches('?');
            for param in params_str.split('&') {
                if let Some((key, value)) = param.split_once('=') {
                    if key == "mqtt_server" {
                        // URL decode the value (basic implementation)
                        mqtt_server_override = Some(value.replace("%3A", ":").replace("%2F", "/"));
                        break;
                    }
                }
            }
        }

        let host = if let Some(mqtt_server) = mqtt_server_override {
            info!("🌐 Using MQTT server from URL parameter: {}", mqtt_server);
            mqtt_server
        } else {
            info!("🌐 Using current host for MQTT server: {}", current_host);
            current_host
        };

        // For web client, use WebSocket port (8083) instead of standard MQTT port (1883)
        let port = 8083;

        info!("🌐 Web MQTT Config: {}:{}", host, port);
        Self { host, port }
    }

    /// Get the broker address as a string for display purposes
    pub fn broker_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
