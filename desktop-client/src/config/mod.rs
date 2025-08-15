use bevy::prelude::*;
use std::env;

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
}

impl MqttConfig {
    /// Load configuration from CLI args, environment variables, or defaults
    /// CLI args take precedence over environment variables
    pub fn from_env_with_override(mqtt_server_override: Option<String>) -> Self {
        let host = mqtt_server_override
            .or_else(|| env::var("MQTT_BROKER_HOST").ok())
            .unwrap_or_else(|| "localhost".to_string());
        let port = env::var("MQTT_BROKER_PORT")
            .unwrap_or_else(|_| "1883".to_string())
            .parse()
            .unwrap_or(1883);

        Self { host, port }
    }

    /// Get the broker address as a string for display purposes
    pub fn broker_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
