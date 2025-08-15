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
    ]
}
