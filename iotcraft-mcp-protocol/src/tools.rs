//! MCP tool definitions organized by category

use crate::types::{BlockType, DeviceType, GameState, McpTool};

#[cfg(feature = "serde")]
use serde_json::json;

/// Tool categories for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCategory {
    System,
    WorldBuilding,
    DeviceManagement,
    WorldManagement,
    GameControl,
}

impl ToolCategory {
    /// Get all tool categories
    pub fn all() -> Vec<ToolCategory> {
        vec![
            ToolCategory::System,
            ToolCategory::WorldBuilding,
            ToolCategory::DeviceManagement,
            ToolCategory::WorldManagement,
            ToolCategory::GameControl,
        ]
    }

    /// Get tools for this category
    pub fn tools(&self) -> Vec<McpTool> {
        match self {
            ToolCategory::System => system_tools(),
            ToolCategory::WorldBuilding => world_building_tools(),
            ToolCategory::DeviceManagement => device_management_tools(),
            ToolCategory::WorldManagement => world_management_tools(),
            ToolCategory::GameControl => game_control_tools(),
        }
    }
}

/// Get all available MCP tools
pub fn get_all_tools() -> Vec<McpTool> {
    let mut tools = Vec::new();
    for category in ToolCategory::all() {
        tools.extend(category.tools());
    }
    tools
}

/// Get tool by name
pub fn get_tool_by_name(name: &str) -> Option<McpTool> {
    get_all_tools().into_iter().find(|tool| tool.name == name)
}

/// System/Health commands
#[cfg(feature = "serde")]
fn system_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "ping".to_string(),
            description: "Test connectivity with the server - returns a simple pong response"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "get_client_info".to_string(),
            description: "Get basic information about the desktop client".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "get_game_state".to_string(),
            description: "Get current game state from the desktop client".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "health_check".to_string(),
            description: "Perform a health check on the desktop client".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "get_system_info".to_string(),
            description: "Get system information from the desktop client".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "get_mqtt_status".to_string(),
            description: "Get MQTT connection status and health information".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
    ]
}

#[cfg(not(feature = "serde"))]
fn system_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "ping".to_string(),
            description: "Test connectivity with the server - returns a simple pong response"
                .to_string(),
            input_schema: r#"{"type": "object", "properties": {}, "required": []}"#.to_string(),
        },
        // Add other system tools without serde dependency...
    ]
}

/// World building commands
#[cfg(feature = "serde")]
fn world_building_tools() -> Vec<McpTool> {
    let block_types: Vec<String> = BlockType::all()
        .iter()
        .map(|b| b.as_str().to_string())
        .collect();

    vec![
        McpTool {
            name: "place_block".to_string(),
            description: "Place a single block at specified coordinates".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "block_type": {
                        "type": "string",
                        "enum": block_types,
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
            description: "Create a wall/rectangular structure between two 3D coordinates"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "block_type": {
                        "type": "string",
                        "enum": block_types,
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

#[cfg(not(feature = "serde"))]
fn world_building_tools() -> Vec<McpTool> {
    vec![] // Placeholder for non-serde implementation
}

/// Device management commands  
#[cfg(feature = "serde")]
fn device_management_tools() -> Vec<McpTool> {
    let device_types: Vec<String> = DeviceType::all()
        .iter()
        .map(|d| d.as_str().to_string())
        .collect();

    vec![
        McpTool {
            name: "list_devices".to_string(),
            description: "List all IoT devices in the world with their positions and types"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
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
                        "enum": device_types,
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
            name: "move_device".to_string(),
            description: "Move a device to new coordinates".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "device_id": {
                        "type": "string",
                        "description": "The unique identifier of the device to move"
                    },
                    "x": {
                        "type": "number",
                        "description": "New X coordinate"
                    },
                    "y": {
                        "type": "number",
                        "description": "New Y coordinate"
                    },
                    "z": {
                        "type": "number",
                        "description": "New Z coordinate"
                    }
                },
                "required": ["device_id", "x", "y", "z"]
            }),
        },
    ]
}

#[cfg(not(feature = "serde"))]
fn device_management_tools() -> Vec<McpTool> {
    vec![] // Placeholder for non-serde implementation
}

/// World management commands
#[cfg(feature = "serde")]
fn world_management_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "publish_world".to_string(),
            description: "Publish current world to be discoverable by other clients".to_string(),
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
    ]
}

#[cfg(not(feature = "serde"))]
fn world_management_tools() -> Vec<McpTool> {
    vec![] // Placeholder for non-serde implementation
}

/// Game control commands
#[cfg(feature = "serde")]
fn game_control_tools() -> Vec<McpTool> {
    let game_states: Vec<String> = GameState::all()
        .iter()
        .map(|g| g.as_str().to_string())
        .collect();

    vec![
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
            description: "Wait for a specific condition to be met (useful for test scenarios)".to_string(),
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
        McpTool {
            name: "list_world_templates".to_string(),
            description: "List available world templates that can be used for world creation".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        McpTool {
            name: "create_world".to_string(),
            description: "Create a new world and set the game state to InGame to transition from menu to gameplay".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "world_name": {
                        "type": "string",
                        "description": "Name for the new world"
                    },
                    "description": {
                        "type": "string",
                        "description": "Description for the new world (default: 'A new world created via MCP')"
                    },
                    "template": {
                        "type": "string",
                        "description": "Template to use for world generation (default: 'default'). Available: default, medieval, modern, creative",
                        "enum": ["default", "medieval", "modern", "creative"]
                    }
                },
                "required": ["world_name"]
            }),
        },
        McpTool {
            name: "load_world_from_fs".to_string(),
            description: "Load an existing world by name from filesystem (single-player/host mode) and set the game state to InGame".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "world_name": {
                        "type": "string",
                        "description": "Name of the world to load from filesystem"
                    }
                },
                "required": ["world_name"]
            }),
        },
        McpTool {
            name: "load_world_from_mqtt".to_string(),
            description: "Load a shared world by reconstructing it from MQTT sticky topic data (multiplayer join mode)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "world_name": {
                        "type": "string",
                        "description": "Name of the world to reconstruct from MQTT shared data"
                    }
                },
                "required": ["world_name"]
            }),
        },
        McpTool {
            name: "set_game_state".to_string(),
            description: "Set the current game state for UI transitions (MainMenu, WorldSelection, InGame, etc.)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "state": {
                        "type": "string",
                        "enum": game_states,
                        "description": "The game state to transition to"
                    }
                },
                "required": ["state"]
            }),
        },
    ]
}

#[cfg(not(feature = "serde"))]
fn game_control_tools() -> Vec<McpTool> {
    vec![] // Placeholder for non-serde implementation
}
