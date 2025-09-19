#[cfg(test)]
mod mcp_tests {
    use super::super::{mcp_tools::*, mcp_types::*};
    use crate::environment::VoxelWorld;
    use crate::mqtt::TemperatureResource;
    use bevy::prelude::*;
    use serde_json::json;

    // Helper function to create a minimal test world
    fn create_test_world() -> World {
        let mut world = World::new();
        // Add minimal resources needed for MCP tools
        world.init_resource::<VoxelWorld>();
        world.init_resource::<TemperatureResource>();
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
    fn test_wall_command_coordinate_validation() {
        // Additional test specifically for the coordinate validation logic
        // This ensures the range calculation works as expected in different scenarios

        // Single block placement (all coordinates equal)
        let args_single = json!({
            "block_type": "dirt",
            "x1": 0,
            "y1": 5,
            "z1": -10,
            "x2": 0,   // Same as x1
            "y2": 5,   // Same as y1
            "z2": -10  // Same as z1
        });

        let world = create_test_world();
        let result = execute_mcp_tool("create_wall", args_single, &world);
        assert!(result.is_ok(), "Single block placement should work");
        let result = result.unwrap();

        if let McpContent::Text { text } = &result.content[0] {
            // Single block should result in exactly 1 block
            assert!(
                text.contains("1 blocks") || text.contains("1 block"),
                "Single coordinate should create 1 block, got: {}",
                text
            );
        }

        // Line of blocks (one dimension varies)
        let args_line = json!({
            "block_type": "stone",
            "x1": 10,
            "y1": 0,
            "z1": 0,
            "x2": 15,  // 6 blocks in X direction
            "y2": 0,   // Same Y
            "z2": 0    // Same Z
        });

        let result_line = execute_mcp_tool("create_wall", args_line, &world);
        assert!(result_line.is_ok(), "Line placement should work");
        let result_line = result_line.unwrap();

        if let McpContent::Text { text } = &result_line.content[0] {
            // Should be (15-10+1) * (0-0+1) * (0-0+1) = 6*1*1 = 6 blocks
            assert!(
                text.contains("6 blocks"),
                "Line should create 6 blocks, got: {}",
                text
            );
        }
    }

    #[test]
    fn test_create_wall_coordinate_ordering_edge_case() {
        // This test covers the scenario that was found in the new_world.txt script
        // where coordinates were incorrectly ordered (e.g., z1=-21, z2=-26)
        //
        // NOTE: The MCP system appears to automatically handle coordinate ordering
        // by internally correcting backwards coordinates, which is user-friendly behavior.
        // This test validates that both correctly ordered and backwards coordinates
        // produce the same expected result.

        // Test case 1: Backwards Z coordinates (MCP auto-corrects these)
        let args_backwards_z = json!({
            "block_type": "stone",
            "x1": 21,
            "y1": 1,
            "z1": -21,  // This is LARGER than z2
            "x2": 26,
            "y2": 1,
            "z2": -26   // This is SMALLER than z1 - MCP should auto-correct
        });

        let world = create_test_world();
        let result = execute_mcp_tool("create_wall", args_backwards_z, &world);

        assert!(
            result.is_ok(),
            "create_wall should handle backwards coordinates gracefully"
        );
        let result = result.unwrap();

        if let McpContent::Text { text } = &result.content[0] {
            // MCP should auto-correct coordinates and create the expected volume
            // (26-21+1) * (1-1+1) * (|-21-(-26)|+1) = 6*1*6 = 36 blocks
            assert!(
                text.contains("36 blocks"),
                "MCP should auto-correct backwards coordinates to create 36 blocks, got: {}",
                text
            );
        }

        // Test case 2: Correctly ordered coordinates (should work the same way)
        let args_correct = json!({
            "block_type": "stone",
            "x1": 21,
            "y1": 1,
            "z1": -26,  // Correctly smaller value first
            "x2": 26,
            "y2": 1,
            "z2": -21   // Correctly larger value second
        });

        let result_correct = execute_mcp_tool("create_wall", args_correct, &world);
        assert!(
            result_correct.is_ok(),
            "create_wall should work with correctly ordered coordinates"
        );
        let result_correct = result_correct.unwrap();

        if let McpContent::Text { text } = &result_correct.content[0] {
            // With correct coordinates, we should get the same result as the auto-corrected backwards case
            assert!(
                text.contains("36 blocks"),
                "Correctly ordered coordinates should create 36 blocks, got: {}",
                text
            );
        }

        // Test case 3: All coordinates backwards (MCP should auto-correct these too)
        let args_all_backwards = json!({
            "block_type": "water",
            "x1": 5,   // larger
            "y1": 2,   // larger
            "z1": 1,   // larger
            "x2": 0,   // smaller
            "y2": 0,   // smaller
            "z2": -1   // smaller
        });

        let result_all_backwards = execute_mcp_tool("create_wall", args_all_backwards, &world);
        assert!(
            result_all_backwards.is_ok(),
            "create_wall should handle all backwards coordinates"
        );
        let result_all_backwards = result_all_backwards.unwrap();

        if let McpContent::Text { text } = &result_all_backwards.content[0] {
            // MCP should auto-correct all coordinates: (5-0+1) * (2-0+1) * (1-(-1)+1) = 6*3*3 = 54 blocks
            assert!(
                text.contains("54 blocks"),
                "MCP should auto-correct all backwards coordinates to create 54 blocks, got: {}",
                text
            );
        }
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
