use anyhow::{Result, anyhow};
use bytes::BytesMut;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::codec::{Decoder, Encoder, LinesCodec};

/// JSON-RPC 2.0 Request
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

/// JSON-RPC 2.0 Notification
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
}

/// MCP Message - can be request, response, or notification
#[derive(Debug, Clone)]
pub enum McpMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
}

/// Codec for MCP messages over stdio/TCP
pub struct McpCodec {
    lines: LinesCodec,
}

impl Default for McpCodec {
    fn default() -> Self {
        Self {
            lines: LinesCodec::new(),
        }
    }
}

impl Decoder for McpCodec {
    type Item = McpMessage;
    type Error = anyhow::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.lines.decode(src)? {
            Some(line) => {
                debug!("Received MCP message: {}", line);

                // Parse as JSON first to determine message type
                let json: Value =
                    serde_json::from_str(&line).map_err(|e| anyhow!("Invalid JSON: {}", e))?;

                // Determine message type based on presence of fields
                if json.get("id").is_some() && json.get("method").is_some() {
                    // Has both id and method - it's a request
                    let request: JsonRpcRequest = serde_json::from_value(json)
                        .map_err(|e| anyhow!("Invalid request: {}", e))?;
                    return Ok(Some(McpMessage::Request(request)));
                } else if json.get("id").is_some()
                    && (json.get("result").is_some() || json.get("error").is_some())
                {
                    // Has id and result/error - it's a response
                    let response: JsonRpcResponse = serde_json::from_value(json)
                        .map_err(|e| anyhow!("Invalid response: {}", e))?;
                    return Ok(Some(McpMessage::Response(response)));
                } else if json.get("method").is_some() && json.get("id").is_none() {
                    // Has method but no id - it's a notification
                    let notification: JsonRpcNotification = serde_json::from_value(json)
                        .map_err(|e| anyhow!("Invalid notification: {}", e))?;
                    return Ok(Some(McpMessage::Notification(notification)));
                }

                error!("Failed to parse MCP message: {}", line);
                Err(anyhow!("Invalid MCP message format"))
            }
            None => Ok(None),
        }
    }
}

impl Encoder<McpMessage> for McpCodec {
    type Error = anyhow::Error;

    fn encode(&mut self, item: McpMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let json_str = match item {
            McpMessage::Request(req) => serde_json::to_string(&req)?,
            McpMessage::Response(resp) => serde_json::to_string(&resp)?,
            McpMessage::Notification(notif) => serde_json::to_string(&notif)?,
        };

        debug!("Sending MCP message: {}", json_str);
        self.lines.encode(json_str, dst)?;
        Ok(())
    }
}

/// Helper functions for creating MCP protocol messages
impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, code: i32, message: String, data: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data,
            }),
        }
    }
}

impl JsonRpcRequest {
    pub fn new(id: Value, method: String, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            method,
            params,
        }
    }
}

impl JsonRpcNotification {
    pub fn new(method: String, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method,
            params,
        }
    }
}

/// MCP Error codes based on JSON-RPC 2.0 specification
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;

    // MCP-specific error codes (application-defined range)
    pub const TOOL_EXECUTION_ERROR: i32 = -32000;
    pub const RESOURCE_NOT_FOUND: i32 = -32001;
    pub const UNAUTHORIZED: i32 = -32002;
}
