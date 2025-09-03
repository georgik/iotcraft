/// Unified scenario types for IoTCraft
///
/// This module provides comprehensive scenario definitions supporting both
/// mcplay binary and xtask infrastructure, with backward compatibility.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main scenario definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    /// Scenario name
    pub name: String,

    /// Scenario description (optional for backward compatibility)
    #[serde(default)]
    pub description: String,

    /// Scenario version (optional for backward compatibility)
    #[serde(default)]
    pub version: String,

    /// Infrastructure requirements
    pub infrastructure: InfrastructureConfig,

    /// Client definitions (unified to support both old and new formats)
    pub clients: Vec<ClientConfig>,

    /// Test steps to execute
    pub steps: Vec<Step>,

    /// Global configuration settings (optional)
    #[serde(default)]
    pub config: Option<ScenarioConfig>,
}

/// Infrastructure configuration (unified format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfrastructureConfig {
    /// MQTT server configuration
    pub mqtt_server: MqttServerConfig,

    /// MQTT observer configuration (optional)
    #[serde(default)]
    pub mqtt_observer: Option<MqttObserverConfig>,

    /// MCP server configuration (optional)
    #[serde(default)]
    pub mcp_server: Option<McpServerConfig>,

    /// Additional services (extensible for xtask)
    #[serde(default)]
    pub services: Option<HashMap<String, ServiceConfig>>,
}

/// MQTT server configuration (unified to support both formats)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttServerConfig {
    /// Whether the MQTT server is required/enabled
    #[serde(alias = "enabled")]
    pub required: bool,

    /// Port to run the server on
    pub port: u16,

    /// Custom configuration file path (xtask extension)
    #[serde(default)]
    pub config_file: Option<String>,

    /// Additional server options (xtask extension)
    #[serde(default)]
    pub options: Option<HashMap<String, serde_json::Value>>,
}

/// MQTT observer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttObserverConfig {
    /// Whether the observer is required/enabled
    #[serde(default)]
    pub required: bool,

    /// Topics to observe (xtask extension)
    #[serde(default)]
    pub topics: Option<Vec<String>>,

    /// Client ID for the observer (xtask extension)
    #[serde(default)]
    pub client_id: Option<String>,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Whether the MCP server is required
    pub required: bool,

    /// Port for the MCP server
    pub port: u16,
}

/// Generic service configuration (xtask extension)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Service type identifier
    pub service_type: String,

    /// Whether the service is enabled
    pub enabled: bool,

    /// Service-specific configuration
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

/// Client configuration (unified to support both formats)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Unique client identifier
    pub id: String,

    /// Player ID (mcplay compatibility)
    #[serde(default)]
    pub player_id: String,

    /// MCP port for this client (mcplay compatibility)
    #[serde(default)]
    pub mcp_port: u16,

    /// Client type (unified field)
    #[serde(rename = "type", alias = "client_type", default)]
    pub client_type: String,

    /// Display name for logging (xtask extension)
    #[serde(default)]
    pub name: Option<String>,

    /// Extended client configuration (xtask extension)
    #[serde(default)]
    pub config: Option<serde_json::Value>,
}

/// Extended client configuration for xtask scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedClientConfig {
    /// Starting position in the world
    #[serde(default)]
    pub spawn_position: Option<Position>,

    /// Inventory items to start with
    #[serde(default)]
    pub initial_inventory: Option<Vec<InventoryItem>>,

    /// Permissions and capabilities
    #[serde(default)]
    pub permissions: Option<Vec<String>>,

    /// Custom client settings
    #[serde(default)]
    pub settings: Option<HashMap<String, serde_json::Value>>,
}

/// 3D position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Inventory item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    /// Item type/ID
    pub item_type: String,

    /// Quantity
    pub quantity: u32,

    /// Item metadata
    #[serde(default)]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Test step definition (unified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// Step name for logging and identification
    pub name: String,

    /// Step description
    pub description: String,

    /// Client that should execute this step (mcplay compatibility)
    #[serde(default)]
    pub client: String,

    /// Action to perform (unified)
    pub action: Action,

    /// Wait time before executing (mcplay compatibility)
    #[serde(default)]
    pub wait_before: u64,

    /// Wait time after executing (mcplay compatibility)
    #[serde(default)]
    pub wait_after: u64,

    /// Timeout for the step (mcplay compatibility)
    #[serde(default)]
    pub timeout: u64,

    /// Success condition (mcplay compatibility)
    #[serde(default)]
    pub success_condition: Option<SuccessCondition>,

    /// Step dependencies (mcplay compatibility)
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Extended timing configuration (xtask extension)
    #[serde(default)]
    pub timing: Option<Timing>,

    /// Prerequisites/conditions (xtask extension)
    #[serde(default)]
    pub conditions: Option<Vec<Condition>>,

    /// Expected outcomes/assertions (xtask extension)
    #[serde(default)]
    pub expectations: Option<Vec<Expectation>>,
}

/// Action to perform in a step (unified enum supporting both formats)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    // mcplay-style actions
    #[serde(rename = "mcp_call")]
    McpCall {
        tool: String,
        arguments: serde_json::Value,
    },
    #[serde(rename = "wait_condition")]
    WaitCondition {
        condition: String,
        expected_value: Option<String>,
        timeout: u64,
    },
    #[serde(rename = "console_command")]
    ConsoleCommand { command: String },
    #[serde(rename = "delay")]
    Delay { duration: u64 },
    #[serde(rename = "validate_scenario")]
    ValidateScenario { checks: Vec<String> },

    // xtask-style actions
    #[serde(rename = "wait")]
    Wait { duration_ms: u64 },
    #[serde(rename = "mqtt_publish")]
    MqttPublish {
        topic: String,
        payload: String,
        qos: Option<u8>,
        retain: Option<bool>,
    },
    #[serde(rename = "mqtt_expect")]
    MqttExpect {
        topic: String,
        payload: Option<String>,
        timeout_ms: Option<u64>,
    },
    #[serde(rename = "client_action")]
    ClientAction {
        client_id: String,
        action_type: ClientActionType,
        parameters: Option<HashMap<String, serde_json::Value>>,
    },
    #[serde(rename = "parallel")]
    Parallel { actions: Vec<Action> },
    #[serde(rename = "sequence")]
    Sequence { actions: Vec<Action> },
    #[serde(rename = "custom")]
    Custom {
        action_type: String,
        parameters: HashMap<String, serde_json::Value>,
    },
}

/// Client action types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientActionType {
    /// Move to a position
    MoveTo,
    /// Place a block
    PlaceBlock,
    /// Break a block  
    BreakBlock,
    /// Use an item
    UseItem,
    /// Send a chat message
    Chat,
    /// Join the game
    Connect,
    /// Leave the game
    Disconnect,
    /// Create a new world
    CreateWorld,
    /// Join an existing world
    JoinWorld,
    /// Enter the game from menu/lobby state
    EnterGame,
    /// Navigate to main menu
    ReturnToMenu,
    /// Wait for client to be ready/initialized
    WaitForReady,
    /// Custom action
    Custom(String),
}

/// Success condition (mcplay compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SuccessCondition {
    #[serde(rename = "mcp_response")]
    McpResponse { expected: String },
    #[serde(rename = "world_state")]
    WorldState { check: String, expected: String },
    #[serde(rename = "client_count")]
    ClientCount { world_id: String, expected: u32 },
    #[serde(rename = "all_checks_passed")]
    AllChecksPassed,
}

/// Timing configuration for steps (xtask extension)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timing {
    /// Delay before starting the step
    #[serde(default)]
    pub delay_ms: Option<u64>,

    /// Maximum time to wait for completion
    #[serde(default)]
    pub timeout_ms: Option<u64>,

    /// Retry configuration
    #[serde(default)]
    pub retry: Option<RetryConfig>,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Delay between retry attempts
    pub delay_ms: u64,

    /// Backoff strategy
    #[serde(default)]
    pub backoff: Option<BackoffStrategy>,
}

/// Backoff strategies for retries
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackoffStrategy {
    /// Fixed delay
    Fixed,
    /// Linear increase
    Linear,
    /// Exponential backoff
    Exponential { base: f64 },
}

/// Condition that must be met before executing a step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    /// Wait for a client to be connected
    ClientConnected { client_id: String },
    /// Wait for an MQTT topic to have a specific value
    MqttTopicValue {
        topic: String,
        expected_value: String,
        timeout_ms: Option<u64>,
    },
    /// Custom condition
    Custom {
        condition_type: String,
        parameters: HashMap<String, serde_json::Value>,
    },
}

/// Expected outcome/assertion for a step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expectation {
    /// Expect an MQTT message on a topic
    MqttMessage {
        topic: String,
        payload_pattern: Option<String>,
        within_ms: Option<u64>,
    },
    /// Expect client state change
    ClientState {
        client_id: String,
        expected_state: String,
        within_ms: Option<u64>,
    },
    /// Custom expectation
    Custom {
        expectation_type: String,
        parameters: HashMap<String, serde_json::Value>,
    },
}

/// Global scenario configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioConfig {
    /// Global timeout for the entire scenario
    #[serde(default)]
    pub timeout_ms: Option<u64>,

    /// Logging configuration
    #[serde(default)]
    pub logging: Option<LoggingConfig>,

    /// Environment variables
    #[serde(default)]
    pub environment: Option<HashMap<String, String>>,

    /// Custom settings
    #[serde(default)]
    pub settings: Option<HashMap<String, serde_json::Value>>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    #[serde(default)]
    pub level: Option<String>,

    /// Whether to log MQTT traffic
    #[serde(default)]
    pub log_mqtt: Option<bool>,

    /// Whether to log client actions
    #[serde(default)]
    pub log_client_actions: Option<bool>,

    /// Custom log filters
    #[serde(default)]
    pub filters: Option<Vec<String>>,
}

// Default implementations for backward compatibility
impl Default for InfrastructureConfig {
    fn default() -> Self {
        Self {
            mqtt_server: MqttServerConfig {
                required: true,
                port: 1883,
                config_file: None,
                options: None,
            },
            mqtt_observer: Some(MqttObserverConfig {
                required: true,
                topics: None,
                client_id: Some("scenario_observer".to_string()),
            }),
            mcp_server: None,
            services: None,
        }
    }
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            delay_ms: None,
            timeout_ms: Some(30000), // 30 second default timeout
            retry: None,
        }
    }
}

// Conversion helpers for backward compatibility
impl ClientConfig {
    /// Create a ClientConfig in mcplay format
    pub fn new_mcplay_style(
        id: String,
        player_id: String,
        mcp_port: u16,
        client_type: String,
    ) -> Self {
        Self {
            id,
            player_id,
            mcp_port,
            client_type,
            name: None,
            config: None,
        }
    }

    /// Create a ClientConfig in xtask format
    pub fn new_xtask_style(
        id: String,
        client_type: String,
        name: Option<String>,
        config: Option<serde_json::Value>,
    ) -> Self {
        Self {
            id,
            player_id: String::default(),
            mcp_port: 0,
            client_type,
            name,
            config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcplay_scenario_compatibility() {
        let json = r#"{
            "name": "test",
            "description": "test description", 
            "version": "1.0.0",
            "clients": [{
                "id": "alice",
                "player_id": "alice",
                "mcp_port": 3001,
                "type": "desktop"
            }],
            "infrastructure": {
                "mqtt_server": {
                    "required": true,
                    "port": 1883
                },
                "mqtt_observer": {
                    "required": false
                }
            },
            "steps": [{
                "name": "test_step",
                "description": "test step",
                "client": "alice",
                "action": {
                    "type": "mcp_call",
                    "tool": "place_block",
                    "arguments": {"block_type": "stone"}
                },
                "wait_before": 0,
                "wait_after": 0,
                "timeout": 5000,
                "success_condition": {
                    "type": "all_checks_passed"
                },
                "depends_on": []
            }]
        }"#;

        let scenario: Scenario = serde_json::from_str(json).unwrap();
        assert_eq!(scenario.name, "test");
        assert_eq!(scenario.clients.len(), 1);
        assert_eq!(scenario.clients[0].id, "alice");
        assert_eq!(scenario.clients[0].mcp_port, 3001);
    }

    #[test]
    fn test_xtask_scenario_compatibility() {
        // Test that the structures can be created
        let scenario = Scenario {
            name: "Test Scenario".to_string(),
            description: "A test scenario".to_string(),
            version: "1.0.0".to_string(),
            infrastructure: InfrastructureConfig::default(),
            clients: vec![ClientConfig::new_xtask_style(
                "player1".to_string(),
                "player".to_string(),
                Some("Test Player".to_string()),
                None,
            )],
            steps: vec![],
            config: None,
        };

        assert_eq!(scenario.name, "Test Scenario");
    }
}
