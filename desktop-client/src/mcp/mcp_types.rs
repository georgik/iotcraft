use bevy::prelude::*;
use rumqttc::Client;
use serde::{Deserialize, Serialize};

/// MCP Protocol version
pub const MCP_VERSION: &str = "2024-11-05";

/// Resource to manage MCP server state
#[derive(Resource)]
pub struct McpServerState {
    pub active_connections: usize,
    pub server_task: Option<bevy::tasks::Task<()>>,
}

impl Default for McpServerState {
    fn default() -> Self {
        Self {
            active_connections: 0,
            server_task: None,
        }
    }
}

/// Channel for receiving MCP requests in Bevy systems
#[derive(Resource)]
pub struct McpRequestChannel {
    pub receiver: async_channel::Receiver<McpRequest>,
    pub sender: async_channel::Sender<McpRequest>,
}

/// Channel for sending MCP responses back to clients
#[derive(Resource)]
pub struct McpResponseChannel {
    pub sender: tokio::sync::mpsc::UnboundedSender<McpResponse>,
    pub receiver: tokio::sync::mpsc::UnboundedReceiver<McpResponse>,
}

/// MCP Request message
#[derive(Debug)]
pub struct McpRequest {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
    pub response_sender: tokio::sync::oneshot::Sender<serde_json::Value>,
}

/// MCP Response message
#[derive(Debug, Clone)]
pub struct McpResponse {
    pub id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<McpError>,
}

/// MCP Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// MCP Tool Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// MCP Resource Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// MCP Initialize Request
#[derive(Debug, Serialize, Deserialize)]
pub struct McpInitializeRequest {
    pub protocol_version: String,
    pub capabilities: McpClientCapabilities,
    pub client_info: McpClientInfo,
}

/// MCP Client Capabilities
#[derive(Debug, Serialize, Deserialize)]
pub struct McpClientCapabilities {
    pub roots: Option<McpRootsCapability>,
    pub sampling: Option<McpSamplingCapability>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpRootsCapability {
    pub list_changed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpSamplingCapability {
    // Add sampling-specific capabilities if needed
}

/// MCP Client Info
#[derive(Debug, Serialize, Deserialize)]
pub struct McpClientInfo {
    pub name: String,
    pub version: String,
}

/// MCP Initialize Response
#[derive(Debug, Serialize, Deserialize)]
pub struct McpInitializeResponse {
    pub protocol_version: String,
    pub capabilities: McpServerCapabilities,
    pub server_info: McpServerInfo,
}

/// MCP Server Capabilities
#[derive(Debug, Serialize, Deserialize)]
pub struct McpServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<McpToolsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<McpResourcesCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<McpPromptsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<McpLoggingCapability>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolsCapability {
    pub list_changed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpResourcesCapability {
    pub subscribe: Option<bool>,
    pub list_changed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpPromptsCapability {
    pub list_changed: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct McpLoggingCapability {}

/// MCP Server Info
#[derive(Debug, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub version: String,
}

/// Tool call request
#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolCall {
    pub name: String,
    pub arguments: Option<serde_json::Value>,
}

/// Tool call result
#[derive(Debug, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    pub is_error: Option<bool>,
}

/// Content types for MCP responses
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { resource: McpResource },
}

/// Event for MCP tool execution requests
#[derive(Event)]
pub struct McpToolExecutionEvent {
    pub request_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub response_sender: tokio::sync::oneshot::Sender<Result<McpToolResult, McpError>>,
}

/// Resource to hold MQTT client for MCP tools
#[derive(Resource)]
pub struct McpMqttClient {
    pub client: Option<Client>,
}

/// Pending MCP tool execution request
#[derive(Debug)]
pub struct PendingToolExecution {
    pub request_id: String,
    pub tool_name: String,
    pub command: String,
    pub response_sender: tokio::sync::oneshot::Sender<serde_json::Value>,
}

/// Resource to track pending tool executions
#[derive(Resource, Default)]
pub struct PendingToolExecutions {
    pub executions: std::collections::HashMap<String, PendingToolExecution>,
}

/// Event to signal that a command has been executed and results are available
#[derive(Event)]
pub struct CommandExecutedEvent {
    pub request_id: String,
    pub result: String,
}
