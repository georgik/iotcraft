#[cfg(test)]
mod mcp_tests {
    use super::super::{mcp_tools::*, mcp_types::*};
    use bevy::prelude::*;
    use serde_json::json;

    // Helper function to create a minimal test world
    fn create_test_world() -> World {
        let mut world = World::new();
        // Add minimal resources needed for MCP tools
        world.init_resource::<crate::VoxelWorld>();
        world.init_resource::<crate::TemperatureResource>();
        world.init_resource::<Time>();
        world
    }

    #[test]
    fn test_create_wall_json_parsing() {
        let args = json!({
            "block_type": "stone",
            "x1": 0,
            "y1": 0,
            "z1": 0,
            "x2": 5,
            "y2": 3,
            "z2": 2
        });

        let world = create_test_world();
        let result = execute_mcp_tool("create_wall", args, &world);

        assert!(
            result.is_ok(),
            "create_wall should succeed with valid parameters"
        );
        let result = result.unwrap();
        assert_eq!(result.is_error, Some(false));
        assert!(!result.content.is_empty());

        if let McpContent::Text { text } = &result.content[0] {
            assert!(text.contains("stone"));
            assert!(text.contains("from (0, 0, 0) to (5, 3, 2)"));
            // Volume should be (5-0+1) * (3-0+1) * (2-0+1) = 6*4*3 = 72 blocks
            assert!(text.contains("72 blocks"));
        } else {
            panic!("Expected text content in create_wall response");
        }
    }

    #[test]
    fn test_create_wall_negative_coordinates() {
        let args = json!({
            "block_type": "grass",
            "x1": -3,
            "y1": -1,
            "z1": -2,
            "x2": 2,
            "y2": 1,
            "z2": 0
        });

        let world = create_test_world();
        let result = execute_mcp_tool("create_wall", args, &world);

        assert!(
            result.is_ok(),
            "create_wall should handle negative coordinates"
        );
        let result = result.unwrap();

        if let McpContent::Text { text } = &result.content[0] {
            // Volume should be (2-(-3)+1) * (1-(-1)+1) * (0-(-2)+1) = 6*3*3 = 54 blocks
            assert!(text.contains("54 blocks"));
        }
    }

    #[test]
    fn test_create_wall_missing_parameters() {
        let args = json!({
            "block_type": "stone",
            "x1": 0,
            "y1": 0,
            "z1": 0
            // Missing x2, y2, z2
        });

        let world = create_test_world();
        let result = execute_mcp_tool("create_wall", args, &world);

        assert!(
            result.is_err(),
            "create_wall should fail with missing parameters"
        );

        let error = result.unwrap_err();
        assert_eq!(error.code, -32602); // Invalid params error code
        assert!(error.message.contains("x2 parameter is required"));
    }

    #[test]
    fn test_create_wall_invalid_block_type() {
        let args = json!({
            "block_type": "invalid_block",
            "x1": 0,
            "y1": 0,
            "z1": 0,
            "x2": 1,
            "y2": 1,
            "z2": 1
        });

        let world = create_test_world();
        let result = execute_mcp_tool("create_wall", args, &world);

        // The function currently doesn't validate block types in execution,
        // it just passes them through as strings
        assert!(
            result.is_ok(),
            "create_wall currently accepts any block type string"
        );
    }

    #[test]
    fn test_place_block_valid_parameters() {
        let args = json!({
            "block_type": "dirt",
            "x": 10,
            "y": 5,
            "z": -3
        });

        let world = create_test_world();
        let result = execute_mcp_tool("place_block", args, &world);

        assert!(
            result.is_ok(),
            "place_block should succeed with valid parameters"
        );
        let result = result.unwrap();

        if let McpContent::Text { text } = &result.content[0] {
            assert!(text.contains("dirt"));
            assert!(text.contains("(10, 5, -3)"));
        }
    }

    #[test]
    fn test_place_block_floating_point_coordinates() {
        let args = json!({
            "block_type": "stone",
            "x": 5.5,  // Now properly handled as floating point
            "y": 2.9,
            "z": -1.1
        });

        let world = create_test_world();
        let result = execute_mcp_tool("place_block", args, &world);

        // This should now succeed since we accept floating point coordinates
        assert!(
            result.is_ok(),
            "place_block should now accept floating point coordinates"
        );
        let result = result.unwrap();
        if let McpContent::Text { text } = &result.content[0] {
            // Should display floating point coordinates as-is
            assert!(text.contains("(5.5, 2.9, -1.1)"));
        }
    }

    #[test]
    fn test_spawn_device_with_defaults() {
        let args = json!({
            "device_id": "test_device_123",
            "device_type": "lamp"
            // No coordinates provided - should use defaults
        });

        let world = create_test_world();
        let result = execute_mcp_tool("spawn_device", args, &world);

        // This will currently fail because coordinates are required in the implementation
        assert!(
            result.is_err(),
            "spawn_device currently requires all coordinates"
        );
        let error = result.unwrap_err();
        assert!(error.message.contains("x parameter is required"));
    }

    #[test]
    fn test_spawn_device_full_parameters() {
        let args = json!({
            "device_id": "lamp_kitchen_01",
            "device_type": "door",
            "x": 5.5,
            "y": 1.0,
            "z": -2.3
        });

        let world = create_test_world();
        let result = execute_mcp_tool("spawn_device", args, &world);

        assert!(
            result.is_ok(),
            "spawn_device should succeed with all parameters"
        );
        let result = result.unwrap();

        if let McpContent::Text { text } = &result.content[0] {
            assert!(text.contains("lamp_kitchen_01"));
            assert!(text.contains("door"));
            assert!(text.contains("(5.5, 1, -2.3)"));
        }
    }

    #[test]
    fn test_list_devices_no_parameters() {
        let args = json!({});
        let world = create_test_world();
        let result = execute_mcp_tool("list_devices", args, &world);

        assert!(
            result.is_ok(),
            "list_devices should succeed with no parameters"
        );
        let result = result.unwrap();
        assert!(!result.content.is_empty());
    }

    #[test]
    fn test_control_device_parameters() {
        let args = json!({
            "device_id": "lamp_01",
            "command": "ON"
        });

        let world = create_test_world();
        let result = execute_mcp_tool("control_device", args, &world);

        assert!(
            result.is_ok(),
            "control_device should succeed with valid parameters"
        );
        let result = result.unwrap();

        if let McpContent::Text { text } = &result.content[0] {
            assert!(text.contains("lamp_01"));
            assert!(text.contains("ON"));
        }
    }

    #[test]
    fn test_move_device_parameters() {
        let args = json!({
            "device_id": "door_main",
            "x": -5.0,
            "y": 0.0,
            "z": 10.5
        });

        let world = create_test_world();
        let result = execute_mcp_tool("move_device", args, &world);

        assert!(
            result.is_ok(),
            "move_device should succeed with valid parameters"
        );
        let result = result.unwrap();

        if let McpContent::Text { text } = &result.content[0] {
            assert!(text.contains("door_main"));
            assert!(text.contains("(-5, 0, 10.5)"));
        }
    }

    #[test]
    fn test_unknown_tool() {
        let args = json!({});
        let world = create_test_world();
        let result = execute_mcp_tool("unknown_tool", args, &world);

        assert!(result.is_err(), "Unknown tools should return error");
        let error = result.unwrap_err();
        assert_eq!(error.code, -32601); // Method not found
        assert!(error.message.contains("unknown_tool"));
    }

    #[test]
    fn test_get_world_status() {
        let world = create_test_world();
        let result = execute_mcp_tool("get_world_status", json!({}), &world);

        assert!(result.is_ok(), "get_world_status should succeed");
        let result = result.unwrap();

        if let McpContent::Text { text } = &result.content[0] {
            // Should contain JSON with world status
            assert!(text.contains("blocks"));
            assert!(text.contains("devices"));
            assert!(text.contains("uptime_seconds"));
        }
    }

    #[test]
    fn test_get_sensor_data() {
        let world = create_test_world();
        let result = execute_mcp_tool("get_sensor_data", json!({}), &world);

        assert!(result.is_ok(), "get_sensor_data should succeed");
        let result = result.unwrap();

        if let McpContent::Text { text } = &result.content[0] {
            // Should contain JSON with sensor data
            assert!(text.contains("temperature"));
            assert!(text.contains("devices_online"));
        }
    }
}

#[cfg(test)]
mod command_conversion_tests {
    use super::super::mcp_server::{convert_tool_call_to_command, should_queue_as_command};
    use serde_json::json;

    #[test]
    fn test_should_queue_as_command() {
        // Test all tools that should be queued
        assert!(should_queue_as_command("list_devices"));
        assert!(should_queue_as_command("control_device"));
        assert!(should_queue_as_command("spawn_device"));
        assert!(should_queue_as_command("place_block"));
        assert!(should_queue_as_command("remove_block"));
        assert!(should_queue_as_command("create_wall"));
        assert!(should_queue_as_command("move_device"));
        assert!(should_queue_as_command("teleport_camera"));
        assert!(should_queue_as_command("set_camera_angle"));
        assert!(should_queue_as_command("save_world"));
        assert!(should_queue_as_command("load_world"));

        // Test tools that shouldn't be queued
        assert!(!should_queue_as_command("unknown_tool"));
        assert!(!should_queue_as_command("get_world_status"));
        assert!(!should_queue_as_command("get_sensor_data"));
    }

    #[test]
    fn test_convert_list_devices() {
        let args = json!({});
        let result = convert_tool_call_to_command("list_devices", &args);

        assert_eq!(result, Some("list".to_string()));
    }

    #[test]
    fn test_convert_control_device() {
        let args = json!({
            "device_id": "lamp_01",
            "command": "OFF"
        });
        let result = convert_tool_call_to_command("control_device", &args);

        assert_eq!(result, Some("control lamp_01 OFF".to_string()));
    }

    #[test]
    fn test_convert_spawn_device_lamp() {
        let args = json!({
            "device_id": "kitchen_lamp",
            "device_type": "lamp",
            "x": 3.0,
            "y": 1.5,
            "z": -2.0
        });
        let result = convert_tool_call_to_command("spawn_device", &args);

        assert_eq!(result, Some("spawn kitchen_lamp 3 1.5 -2".to_string()));
    }

    #[test]
    fn test_convert_spawn_device_door() {
        let args = json!({
            "device_id": "main_door",
            "device_type": "door",
            "x": 0.0,
            "y": 1.0,
            "z": 5.0
        });
        let result = convert_tool_call_to_command("spawn_device", &args);

        assert_eq!(result, Some("spawn_door main_door 0 1 5".to_string()));
    }

    #[test]
    fn test_convert_spawn_device_with_defaults() {
        let args = json!({
            "device_id": "test_device",
            "device_type": "lamp"
            // No coordinates provided
        });
        let result = convert_tool_call_to_command("spawn_device", &args);

        // Should use defaults: x=0.0, y=1.0, z=0.0
        assert_eq!(result, Some("spawn test_device 0 1 0".to_string()));
    }

    #[test]
    fn test_convert_spawn_device_invalid_type() {
        let args = json!({
            "device_id": "invalid_device",
            "device_type": "invalid_type",
            "x": 1.0,
            "y": 1.0,
            "z": 1.0
        });
        let result = convert_tool_call_to_command("spawn_device", &args);

        assert_eq!(result, None); // Should return None for invalid device type
    }

    #[test]
    fn test_convert_place_block() {
        let args = json!({
            "block_type": "stone",
            "x": 10,
            "y": 5,
            "z": -3
        });
        let result = convert_tool_call_to_command("place_block", &args);

        assert_eq!(result, Some("place stone 10 5 -3".to_string()));
    }

    #[test]
    fn test_convert_remove_block() {
        let args = json!({
            "x": -5,
            "y": 0,
            "z": 8
        });
        let result = convert_tool_call_to_command("remove_block", &args);

        assert_eq!(result, Some("remove -5 0 8".to_string()));
    }

    #[test]
    fn test_convert_create_wall() {
        let args = json!({
            "block_type": "grass",
            "x1": 0,
            "y1": 0,
            "z1": 0,
            "x2": 5,
            "y2": 3,
            "z2": 2
        });
        let result = convert_tool_call_to_command("create_wall", &args);

        assert_eq!(result, Some("wall grass 0 0 0 5 3 2".to_string()));
    }

    #[test]
    fn test_convert_move_device() {
        let args = json!({
            "device_id": "sensor_01",
            "x": -2.5,
            "y": 1.2,
            "z": 4.8
        });
        let result = convert_tool_call_to_command("move_device", &args);

        assert_eq!(result, Some("move sensor_01 -2.5 1.2 4.8".to_string()));
    }

    #[test]
    fn test_convert_teleport_camera() {
        let args = json!({
            "x": 10.0,
            "y": 20.0,
            "z": -5.0
        });
        let result = convert_tool_call_to_command("teleport_camera", &args);

        assert_eq!(result, Some("tp 10 20 -5".to_string()));
    }

    #[test]
    fn test_convert_set_camera_angle() {
        let args = json!({
            "yaw": 45.0,
            "pitch": -30.0
        });
        let result = convert_tool_call_to_command("set_camera_angle", &args);

        assert_eq!(result, Some("look 45 -30".to_string()));
    }

    #[test]
    fn test_convert_save_world() {
        let args = json!({
            "filename": "my_world.json"
        });
        let result = convert_tool_call_to_command("save_world", &args);

        assert_eq!(result, Some("save_map my_world.json".to_string()));
    }

    #[test]
    fn test_convert_load_world() {
        let args = json!({
            "filename": "castle.json"
        });
        let result = convert_tool_call_to_command("load_world", &args);

        assert_eq!(result, Some("load_map castle.json".to_string()));
    }

    #[test]
    fn test_convert_missing_parameters() {
        // Test missing required parameters
        let args = json!({
            "block_type": "stone"
            // Missing coordinates
        });
        let result = convert_tool_call_to_command("place_block", &args);

        assert_eq!(result, None); // Should return None when required params are missing
    }

    #[test]
    fn test_convert_wrong_parameter_types() {
        // Test wrong parameter types (string instead of number)
        let args = json!({
            "block_type": "stone",
            "x": "not_a_number",
            "y": 1,
            "z": 1
        });
        let result = convert_tool_call_to_command("place_block", &args);

        assert_eq!(result, None); // Should return None when types are wrong
    }
}
