use crate::mcp::McpTool;
use bevy::prelude::*;
use serde_json::json;

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
        _ => Err(McpError {
            code: -32601,
            message: format!("Tool '{}' not found", tool_name),
            data: None,
        }),
    }
}

/// Create the default set of IoTCraft MCP tools (minimal for testing)
pub fn create_default_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "list_devices".to_string(),
            description: "List all IoT devices in the world with their positions and types"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        McpTool {
            name: "control_device".to_string(),
            description: "Send a control command to an IoT device".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "device_id": {
                        "type": "string",
                        "description": "The unique identifier of the device to control"
                    },
                    "command": {
                        "type": "string",
                        "description": "The command to send to the device (e.g., 'ON', 'OFF', 'open', 'close')"
                    }
                },
                "required": ["device_id", "command"]
            }),
        },
        McpTool {
            name: "spawn_device".to_string(),
            description: "Create a new IoT device at specified coordinates".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "device_id": {
                        "type": "string",
                        "description": "Unique identifier for the new device"
                    },
                    "device_type": {
                        "type": "string",
                        "enum": ["lamp", "door"],
                        "description": "Type of device to spawn"
                    },
                    "x": {
                        "type": "number",
                        "description": "X coordinate (default: 0.0)"
                    },
                    "y": {
                        "type": "number",
                        "description": "Y coordinate (default: 1.0)"
                    },
                    "z": {
                        "type": "number",
                        "description": "Z coordinate (default: 0.0)"
                    }
                },
                "required": ["device_id", "device_type"]
            }),
        },
        McpTool {
            name: "place_block".to_string(),
            description: "Place a single block at specified coordinates".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "block_type": {
                        "type": "string",
                        "enum": ["grass", "dirt", "stone", "quartz_block", "glass_pane", "cyan_terracotta"],
                        "description": "Type of block to place"
                    },
                    "x": {
                        "type": "number",
                        "description": "X coordinate"
                    },
                    "y": {
                        "type": "number",
                        "description": "Y coordinate"
                    },
                    "z": {
                        "type": "number",
                        "description": "Z coordinate"
                    }
                },
                "required": ["block_type", "x", "y", "z"]
            }),
        },
        McpTool {
            name: "remove_block".to_string(),
            description: "Remove a block at specified coordinates".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "x": {
                        "type": "number",
                        "description": "X coordinate"
                    },
                    "y": {
                        "type": "number",
                        "description": "Y coordinate"
                    },
                    "z": {
                        "type": "number",
                        "description": "Z coordinate"
                    }
                },
                "required": ["x", "y", "z"]
            }),
        },
        McpTool {
            name: "create_wall".to_string(),
            description: "Create a wall or rectangular structure between two 3D coordinates"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "block_type": {
                        "type": "string",
                        "enum": ["grass", "dirt", "stone", "quartz_block", "glass_pane", "cyan_terracotta"],
                        "description": "Type of block to use for the wall"
                    },
                    "x1": {
                        "type": "number",
                        "description": "Starting X coordinate"
                    },
                    "y1": {
                        "type": "number",
                        "description": "Starting Y coordinate"
                    },
                    "z1": {
                        "type": "number",
                        "description": "Starting Z coordinate"
                    },
                    "x2": {
                        "type": "number",
                        "description": "Ending X coordinate"
                    },
                    "y2": {
                        "type": "number",
                        "description": "Ending Y coordinate"
                    },
                    "z2": {
                        "type": "number",
                        "description": "Ending Z coordinate"
                    }
                },
                "required": ["block_type", "x1", "y1", "z1", "x2", "y2", "z2"]
            }),
        },
        McpTool {
            name: "publish_world".to_string(),
            description: "Publish the current world to be discoverable by other clients"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "world_name": {
                        "type": "string",
                        "description": "Name for the shared world (defaults to current world name)"
                    },
                    "max_players": {
                        "type": "number",
                        "description": "Maximum number of players allowed (default: 4)"
                    },
                    "is_public": {
                        "type": "boolean",
                        "description": "Whether the world is publicly discoverable (default: true)"
                    }
                },
                "required": []
            }),
        },
        McpTool {
            name: "unpublish_world".to_string(),
            description: "Stop sharing the current world and return to single-player mode"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "join_world".to_string(),
            description: "Join a shared world by world ID".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "world_id": {
                        "type": "string",
                        "description": "Unique identifier of the world to join"
                    }
                },
                "required": ["world_id"]
            }),
        },
        McpTool {
            name: "leave_world".to_string(),
            description: "Leave the current shared world and return to single-player mode"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "list_online_worlds".to_string(),
            description: "List all discoverable shared worlds".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "get_multiplayer_status".to_string(),
            description: "Get current multiplayer mode and world information".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "player_move".to_string(),
            description: "Move the player to a specific position".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "x": {
                        "type": "number",
                        "description": "Target X coordinate"
                    },
                    "y": {
                        "type": "number",
                        "description": "Target Y coordinate"
                    },
                    "z": {
                        "type": "number",
                        "description": "Target Z coordinate"
                    }
                },
                "required": ["x", "y", "z"]
            }),
        },
        McpTool {
            name: "wait_for_condition".to_string(),
            description: "Wait for a specific condition to be met (useful for test scenarios)"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "condition": {
                        "type": "string",
                        "enum": ["world_published", "world_joined", "player_connected", "block_placed"],
                        "description": "Condition to wait for"
                    },
                    "timeout_seconds": {
                        "type": "number",
                        "description": "Maximum time to wait in seconds (default: 30)"
                    },
                    "expected_value": {
                        "type": "string",
                        "description": "Expected value for the condition (optional)"
                    }
                },
                "required": ["condition"]
            }),
        },
    ]
}
