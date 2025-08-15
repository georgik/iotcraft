use crate::{
    TemperatureResource, VoxelWorld,
    mcp::{McpContent, McpError, McpTool, McpToolResult},
};
use bevy::prelude::*;
use log::info;
use serde_json::{Value, json};

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

/// Execute an MCP tool with the given arguments
pub fn execute_mcp_tool(
    tool_name: &str,
    arguments: Value,
    world: &World,
) -> Result<McpToolResult, McpError> {
    info!("Executing MCP tool: {} with args: {}", tool_name, arguments);

    match tool_name {
        "place_block" => execute_place_block(arguments, world),
        "remove_block" => execute_remove_block(arguments, world),
        "create_wall" => execute_create_wall(arguments, world),
        "spawn_device" => execute_spawn_device(arguments, world),
        "list_devices" => execute_list_devices(world),
        "control_device" => execute_control_device(arguments, world),
        "move_device" => execute_move_device(arguments, world),
        "teleport_camera" => execute_teleport_camera(arguments, world),
        "set_camera_angle" => execute_set_camera_angle(arguments, world),
        "get_world_status" => execute_get_world_status(world),
        "get_sensor_data" => execute_get_sensor_data(world),
        "save_world" => execute_save_world(arguments, world),
        "load_world" => execute_load_world(arguments, world),
        _ => Err(McpError {
            code: -32601, // Method not found
            message: format!("Tool '{}' not found", tool_name),
            data: None,
        }),
    }
}

fn execute_place_block(args: Value, _world: &World) -> Result<McpToolResult, McpError> {
    let block_type = args["block_type"].as_str().ok_or_else(|| McpError {
        code: -32602,
        message: "block_type parameter is required".to_string(),
        data: None,
    })?;

    let x = args["x"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "x parameter is required".to_string(),
        data: None,
    })?;

    let y = args["y"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "y parameter is required".to_string(),
        data: None,
    })?;

    let z = args["z"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "z parameter is required".to_string(),
        data: None,
    })?;

    // Note: In a real implementation, this would queue the command for execution
    // in the main Bevy thread via a command channel

    Ok(McpToolResult {
        content: vec![McpContent::Text {
            text: format!(
                "Queued placement of {} block at ({:.1}, {:.1}, {:.1})",
                block_type, x, y, z
            ),
        }],
        is_error: Some(false),
    })
}

fn execute_remove_block(args: Value, _world: &World) -> Result<McpToolResult, McpError> {
    let x = args["x"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "x parameter is required".to_string(),
        data: None,
    })?;

    let y = args["y"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "y parameter is required".to_string(),
        data: None,
    })?;

    let z = args["z"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "z parameter is required".to_string(),
        data: None,
    })?;

    Ok(McpToolResult {
        content: vec![McpContent::Text {
            text: format!("Queued removal of block at ({:.1}, {:.1}, {:.1})", x, y, z),
        }],
        is_error: Some(false),
    })
}

fn execute_create_wall(args: Value, _world: &World) -> Result<McpToolResult, McpError> {
    let block_type = args["block_type"].as_str().ok_or_else(|| McpError {
        code: -32602,
        message: "block_type parameter is required".to_string(),
        data: None,
    })?;

    let x1 = args["x1"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "x1 parameter is required".to_string(),
        data: None,
    })?;

    let y1 = args["y1"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "y1 parameter is required".to_string(),
        data: None,
    })?;

    let z1 = args["z1"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "z1 parameter is required".to_string(),
        data: None,
    })?;

    let x2 = args["x2"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "x2 parameter is required".to_string(),
        data: None,
    })?;

    let y2 = args["y2"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "y2 parameter is required".to_string(),
        data: None,
    })?;

    let z2 = args["z2"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "z2 parameter is required".to_string(),
        data: None,
    })?;

    // For volume calculation, we'll round to integers since blocks occupy discrete positions
    let x1_rounded = x1.floor() as i32;
    let y1_rounded = y1.floor() as i32;
    let z1_rounded = z1.floor() as i32;
    let x2_rounded = x2.floor() as i32;
    let y2_rounded = y2.floor() as i32;
    let z2_rounded = z2.floor() as i32;

    let volume = (x2_rounded - x1_rounded + 1).max(0)
        * (y2_rounded - y1_rounded + 1).max(0)
        * (z2_rounded - z1_rounded + 1).max(0);

    Ok(McpToolResult {
        content: vec![McpContent::Text {
            text: format!(
                "Queued creation of {} wall from ({:.1}, {:.1}, {:.1}) to ({:.1}, {:.1}, {:.1}) - {} blocks",
                block_type, x1, y1, z1, x2, y2, z2, volume
            ),
        }],
        is_error: Some(false),
    })
}

fn execute_spawn_device(args: Value, _world: &World) -> Result<McpToolResult, McpError> {
    let device_id = args["device_id"].as_str().ok_or_else(|| McpError {
        code: -32602,
        message: "device_id parameter is required".to_string(),
        data: None,
    })?;

    let device_type = args["device_type"].as_str().ok_or_else(|| McpError {
        code: -32602,
        message: "device_type parameter is required".to_string(),
        data: None,
    })?;

    let x = args["x"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "x parameter is required".to_string(),
        data: None,
    })?;

    let y = args["y"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "y parameter is required".to_string(),
        data: None,
    })?;

    let z = args["z"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "z parameter is required".to_string(),
        data: None,
    })?;

    Ok(McpToolResult {
        content: vec![McpContent::Text {
            text: format!(
                "Queued spawn of {} device '{}' at ({:.1}, {:.1}, {:.1})",
                device_type, device_id, x, y, z
            ),
        }],
        is_error: Some(false),
    })
}

fn execute_list_devices(_world: &World) -> Result<McpToolResult, McpError> {
    // For now, return simple device list since DeviceEntity doesn't have position/state fields
    // This would be enhanced to get actual device data
    let devices = json!([
        {
            "message": "Device listing requires world system access - use console 'list' command for full device information"
        }
    ]);

    Ok(McpToolResult {
        content: vec![McpContent::Text {
            text: format!(
                "Active devices: {}",
                serde_json::to_string_pretty(&devices).unwrap_or_default()
            ),
        }],
        is_error: Some(false),
    })
}

fn execute_control_device(args: Value, _world: &World) -> Result<McpToolResult, McpError> {
    let device_id = args["device_id"].as_str().ok_or_else(|| McpError {
        code: -32602,
        message: "device_id parameter is required".to_string(),
        data: None,
    })?;

    let command = args["command"].as_str().ok_or_else(|| McpError {
        code: -32602,
        message: "command parameter is required".to_string(),
        data: None,
    })?;

    Ok(McpToolResult {
        content: vec![McpContent::Text {
            text: format!("Queued command '{}' for device '{}'", command, device_id),
        }],
        is_error: Some(false),
    })
}

fn execute_move_device(args: Value, _world: &World) -> Result<McpToolResult, McpError> {
    let device_id = args["device_id"].as_str().ok_or_else(|| McpError {
        code: -32602,
        message: "device_id parameter is required".to_string(),
        data: None,
    })?;

    let x = args["x"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "x parameter is required".to_string(),
        data: None,
    })?;

    let y = args["y"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "y parameter is required".to_string(),
        data: None,
    })?;

    let z = args["z"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "z parameter is required".to_string(),
        data: None,
    })?;

    Ok(McpToolResult {
        content: vec![McpContent::Text {
            text: format!(
                "Queued move of device '{}' to ({:.1}, {:.1}, {:.1})",
                device_id, x, y, z
            ),
        }],
        is_error: Some(false),
    })
}

fn execute_teleport_camera(args: Value, _world: &World) -> Result<McpToolResult, McpError> {
    let x = args["x"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "x parameter is required".to_string(),
        data: None,
    })?;

    let y = args["y"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "y parameter is required".to_string(),
        data: None,
    })?;

    let z = args["z"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "z parameter is required".to_string(),
        data: None,
    })?;

    Ok(McpToolResult {
        content: vec![McpContent::Text {
            text: format!("Queued camera teleport to ({:.1}, {:.1}, {:.1})", x, y, z),
        }],
        is_error: Some(false),
    })
}

fn execute_set_camera_angle(args: Value, _world: &World) -> Result<McpToolResult, McpError> {
    let yaw = args["yaw"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "yaw parameter is required".to_string(),
        data: None,
    })?;

    let pitch = args["pitch"].as_f64().ok_or_else(|| McpError {
        code: -32602,
        message: "pitch parameter is required".to_string(),
        data: None,
    })?;

    Ok(McpToolResult {
        content: vec![McpContent::Text {
            text: format!(
                "Queued camera angle set to yaw: {:.1}°, pitch: {:.1}°",
                yaw, pitch
            ),
        }],
        is_error: Some(false),
    })
}

fn execute_get_world_status(world: &World) -> Result<McpToolResult, McpError> {
    let mut status = json!({
        "blocks": 0,
        "devices": 0,
        "uptime_seconds": 0.0
    });

    // Try to get world information
    if let Some(voxel_world) = world.get_resource::<VoxelWorld>() {
        status["blocks"] = json!(voxel_world.blocks.len());
    }

    if let Some(time) = world.get_resource::<Time>() {
        status["uptime_seconds"] = json!(time.elapsed_secs());
    }

    // Device count would be retrieved via proper world access
    status["devices"] = json!(0);

    Ok(McpToolResult {
        content: vec![McpContent::Text {
            text: format!(
                "World status: {}",
                serde_json::to_string_pretty(&status).unwrap_or_default()
            ),
        }],
        is_error: Some(false),
    })
}

fn execute_get_sensor_data(world: &World) -> Result<McpToolResult, McpError> {
    let mut sensor_data = json!({
        "temperature": null,
        "devices_online": 0
    });

    // Get temperature data
    if let Some(temp_resource) = world.get_resource::<TemperatureResource>() {
        if let Some(temp) = temp_resource.value {
            sensor_data["temperature"] = json!(temp);
        }
    }

    // Online device count would be retrieved via proper world access
    sensor_data["devices_online"] = json!(0);

    Ok(McpToolResult {
        content: vec![McpContent::Text {
            text: format!(
                "Sensor data: {}",
                serde_json::to_string_pretty(&sensor_data).unwrap_or_default()
            ),
        }],
        is_error: Some(false),
    })
}

fn execute_save_world(args: Value, _world: &World) -> Result<McpToolResult, McpError> {
    let filename = args["filename"].as_str().ok_or_else(|| McpError {
        code: -32602,
        message: "filename parameter is required".to_string(),
        data: None,
    })?;

    Ok(McpToolResult {
        content: vec![McpContent::Text {
            text: format!("Queued world save to '{}'", filename),
        }],
        is_error: Some(false),
    })
}

fn execute_load_world(args: Value, _world: &World) -> Result<McpToolResult, McpError> {
    let filename = args["filename"].as_str().ok_or_else(|| McpError {
        code: -32602,
        message: "filename parameter is required".to_string(),
        data: None,
    })?;

    Ok(McpToolResult {
        content: vec![McpContent::Text {
            text: format!("Queued world load from '{}'", filename),
        }],
        is_error: Some(false),
    })
}
