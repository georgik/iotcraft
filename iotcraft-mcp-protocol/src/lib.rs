//! IoTCraft MCP Protocol
//!
//! This crate provides shared protocol definitions for the Model Context Protocol (MCP)
//! implementation used between the IoTCraft desktop client and mcplay orchestrator.
//!
//! # Features
//!
//! - **Shared Tool Definitions**: All MCP tools defined in one place
//! - **Type Safety**: Shared types prevent API mismatches
//! - **Validation**: Input validation for tool parameters
//! - **Versioning**: Protocol version management
//!
//! # Example
//!
//! ```rust
//! use iotcraft_mcp_protocol::{McpTool, tools::get_all_tools, PROTOCOL_VERSION};
//!
//! // Get all available tools
//! let tools = get_all_tools();
//! println!("Protocol version: {}", PROTOCOL_VERSION);
//! ```

pub mod protocol;
pub mod tools;
pub mod types;
pub mod validation;

// Re-export commonly used types
pub use protocol::*;
pub use tools::{get_all_tools, ToolCategory};
pub use types::*;

#[cfg(feature = "serde")]
pub use serde_json;

/// Current MCP protocol version
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// IoTCraft MCP server information
pub const SERVER_NAME: &str = "iotcraft";
pub const SERVER_VERSION: &str = "1.0.0";

/// Client information for mcplay
pub const CLIENT_NAME: &str = "mcplay";
pub const CLIENT_VERSION: &str = "1.0.0";
