use crate::mcp::McpTool;
use bevy::prelude::*;

// Import from shared protocol crate
use iotcraft_mcp_protocol::tools::get_all_tools;
// For now, only import what we directly use

/// Registry of available MCP tools
#[derive(Resource)]
pub struct McpToolRegistry {
    pub tools: Vec<McpTool>,
}

impl Default for McpToolRegistry {
    fn default() -> Self {
        Self {
            tools: create_default_tools(),
        }
    }
}

/// Mock function for testing MCP tool execution
#[cfg(test)]
pub fn execute_mcp_tool(
    tool_name: &str,
    args: serde_json::Value,
    _world: &bevy::prelude::World,
) -> Result<crate::mcp::McpToolResult, crate::mcp::McpError> {
    use crate::mcp::{McpContent, McpError, McpToolResult};

    match tool_name {
        "create_wall" => {
            // Mock detailed create_wall response based on args
            if let (Some(block_type), Some(x1), Some(y1), Some(z1), Some(x2), Some(y2), Some(z2)) = (
                args.get("block_type").and_then(|v| v.as_str()),
                args.get("x1").and_then(|v| v.as_f64()),
                args.get("y1").and_then(|v| v.as_f64()),
                args.get("z1").and_then(|v| v.as_f64()),
                args.get("x2").and_then(|v| v.as_f64()),
                args.get("y2").and_then(|v| v.as_f64()),
                args.get("z2").and_then(|v| v.as_f64()),
            ) {
                let volume =
                    ((x2 - x1).abs() + 1.0) * ((y2 - y1).abs() + 1.0) * ((z2 - z1).abs() + 1.0);
                Ok(McpToolResult {
                    content: vec![McpContent::Text {
                        text: format!(
                            "Created {} wall from ({}, {}, {}) to ({}, {}, {}) - {} blocks",
                            block_type, x1, y1, z1, x2, y2, z2, volume as i32
                        ),
                    }],
                    is_error: Some(false),
                })
            } else {
                Err(McpError {
                    code: -32602,
                    message: "x2 parameter is required".to_string(),
                    data: None,
                })
            }
        }
        "place_block" => {
            if let (Some(block_type), Some(x), Some(y), Some(z)) = (
                args.get("block_type").and_then(|v| v.as_str()),
                args.get("x").and_then(|v| v.as_f64()),
                args.get("y").and_then(|v| v.as_f64()),
                args.get("z").and_then(|v| v.as_f64()),
            ) {
                Ok(McpToolResult {
                    content: vec![McpContent::Text {
                        text: format!("Placed {} block at ({}, {}, {})", block_type, x, y, z),
                    }],
                    is_error: Some(false),
                })
            } else {
                Err(McpError {
                    code: -32602,
                    message: "Missing required parameters".to_string(),
                    data: None,
                })
            }
        }
        "spawn_device" => {
            if args.get("x").is_none() {
                Err(McpError {
                    code: -32602,
                    message: "x parameter is required".to_string(),
                    data: None,
                })
            } else {
                let device_id = args
                    .get("device_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let device_type = args
                    .get("device_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("lamp");
                let x = args.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let y = args.get("y").and_then(|v| v.as_f64()).unwrap_or(1.0);
                let z = args.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0);
                Ok(McpToolResult {
                    content: vec![McpContent::Text {
                        text: format!(
                            "Spawned {} {} at ({}, {}, {})",
                            device_id, device_type, x, y, z
                        ),
                    }],
                    is_error: Some(false),
                })
            }
        }
        "control_device" => {
            let device_id = args
                .get("device_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let command = args
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN");
            Ok(McpToolResult {
                content: vec![McpContent::Text {
                    text: format!("Sent {} command to {}", command, device_id),
                }],
                is_error: Some(false),
            })
        }
        "move_device" => {
            let device_id = args
                .get("device_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let x = args.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let y = args.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let z = args.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0);
            Ok(McpToolResult {
                content: vec![McpContent::Text {
                    text: format!("Moved {} to ({}, {}, {})", device_id, x, y, z),
                }],
                is_error: Some(false),
            })
        }
        "list_devices" => Ok(McpToolResult {
            content: vec![McpContent::Text {
                text: "No devices found".to_string(),
            }],
            is_error: Some(false),
        }),
        "get_world_status" => Ok(McpToolResult {
            content: vec![McpContent::Text {
                text: r#"{"blocks": 0, "devices": [], "uptime_seconds": 0}"#.to_string(),
            }],
            is_error: Some(false),
        }),
        "get_sensor_data" => Ok(McpToolResult {
            content: vec![McpContent::Text {
                text: r#"{"temperature": 22.5, "devices_online": 0}"#.to_string(),
            }],
            is_error: Some(false),
        }),
        "ping" => Ok(McpToolResult {
            content: vec![McpContent::Text {
                text: "pong".to_string(),
            }],
            is_error: Some(false),
        }),
        _ => Err(McpError {
            code: -32601,
            message: format!("Tool '{}' not found", tool_name),
            data: None,
        }),
    }
}

/// Create the default set of IoTCraft MCP tools from shared protocol
pub fn create_default_tools() -> Vec<McpTool> {
    // Get tools from shared protocol and convert to internal format
    get_all_tools()
        .into_iter()
        .map(|protocol_tool| McpTool {
            name: protocol_tool.name,
            description: protocol_tool.description,
            input_schema: protocol_tool.input_schema,
        })
        .collect()
}
