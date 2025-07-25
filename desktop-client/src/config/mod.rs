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

impl MqttConfig {
    /// Load configuration from environment variables or use defaults
    pub fn from_env() -> Self {
        let host = env::var("MQTT_BROKER_HOST").unwrap_or_else(|_| "localhost".to_string());
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
