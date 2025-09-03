//! MCP protocol constants and error codes

/// Standard JSON-RPC error codes
pub mod error_codes {
    /// Parse error - Invalid JSON
    pub const PARSE_ERROR: i32 = -32700;
    /// Invalid request - JSON is not a valid request
    pub const INVALID_REQUEST: i32 = -32600;
    /// Method not found
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// Invalid params
    pub const INVALID_PARAMS: i32 = -32602;
    /// Internal error
    pub const INTERNAL_ERROR: i32 = -32603;

    /// Application-specific error codes (start from -32000)
    /// Tool execution failed
    pub const TOOL_EXECUTION_ERROR: i32 = -32000;
    /// Tool not available
    pub const TOOL_NOT_AVAILABLE: i32 = -32001;
    /// Resource not found
    pub const RESOURCE_NOT_FOUND: i32 = -32002;
    /// Permission denied
    pub const PERMISSION_DENIED: i32 = -32003;
    /// Service unavailable
    pub const SERVICE_UNAVAILABLE: i32 = -32004;
}

/// Standard MCP methods
pub mod methods {
    /// Initialize the MCP connection
    pub const INITIALIZE: &str = "initialize";
    /// List available tools
    pub const TOOLS_LIST: &str = "tools/list";
    /// Call a specific tool
    pub const TOOLS_CALL: &str = "tools/call";
    /// List available resources
    pub const RESOURCES_LIST: &str = "resources/list";
    /// Read a specific resource
    pub const RESOURCES_READ: &str = "resources/read";
    /// Ping for connectivity testing
    pub const PING: &str = "ping";
}

/// MCP capability definitions
#[cfg(feature = "serde")]
pub mod capabilities {
    use serde_json::{json, Value};

    /// Server capabilities
    pub fn server_capabilities() -> Value {
        json!({
            "tools": {
                "listChanged": false
            },
            "resources": {
                "subscribe": false,
                "listChanged": false
            }
        })
    }

    /// Client capabilities
    pub fn client_capabilities() -> Value {
        json!({
            "roots": {
                "listChanged": false
            },
            "sampling": {}
        })
    }
}

/// Protocol version constraints
pub const MIN_SUPPORTED_VERSION: &str = "2024-11-05";
pub const MAX_SUPPORTED_VERSION: &str = "2024-11-05";

/// Default ports
pub const DEFAULT_MCP_PORT: u16 = 8080;
pub const DEFAULT_MQTT_PORT: u16 = 1883;

/// Timeout configurations (in seconds)
pub const DEFAULT_REQUEST_TIMEOUT: u64 = 30;
pub const DEFAULT_PING_TIMEOUT: u64 = 5;
pub const DEFAULT_TOOL_TIMEOUT: u64 = 60;

/// Maximum limits
pub const MAX_CONTENT_SIZE: usize = 10 * 1024 * 1024; // 10MB
pub const MAX_TOOLS_PER_REQUEST: usize = 100;
pub const MAX_CONCURRENT_REQUESTS: usize = 50;
