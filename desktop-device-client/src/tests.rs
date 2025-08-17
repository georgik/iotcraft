use super::*;
use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_player_id() {
        let id1 = generate_player_id();
        let id2 = generate_player_id();

        assert!(id1.starts_with("player-"));
        assert!(id2.starts_with("player-"));
        assert_ne!(id1, id2); // Should generate unique IDs
        assert_eq!(id1.len(), 23); // "player-" (7) + hex (16)
    }

    #[test]
    fn test_now_ts() {
        let ts1 = now_ts();
        std::thread::sleep(Duration::from_millis(10));
        let ts2 = now_ts();

        assert!(ts2 > ts1);
        assert!(ts1 > 0);
    }

    #[test]
    fn test_player_state_default() {
        let state = PlayerState::default();

        assert_eq!(state.position, [0.0, 2.0, 0.0]);
        assert_eq!(state.yaw, 0.0);
        assert_eq!(state.pitch, 0.0);
        assert_eq!(state.movement_time, 0.0);
    }

    #[test]
    fn test_device_state_default() {
        let state = DeviceState::default();

        assert_eq!(state.properties.x, 1.0);
        assert_eq!(state.properties.y, 0.5);
        assert_eq!(state.properties.z, 2.0);
        assert!(!state.light_state);
    }

    #[test]
    fn test_update_player_position_static() {
        let mut state = PlayerState::default();
        let initial_position = state.position;
        let initial_yaw = state.yaw;

        update_player_position(&mut state, "static", 1.0);

        assert_eq!(state.position, initial_position);
        assert_eq!(state.yaw, initial_yaw);
        assert_eq!(state.movement_time, 1.0);
    }

    #[test]
    fn test_update_player_position_circle() {
        let mut state = PlayerState::default();

        update_player_position(&mut state, "circle", 1.0);

        assert_eq!(state.movement_time, 1.0);
        assert_ne!(state.position[0], 0.0); // Should have moved
        assert_ne!(state.position[2], 0.0); // Should have moved
        assert_eq!(state.position[1], 2.0); // Y should stay the same (initial Y from default)
        assert_ne!(state.yaw, 0.0); // Yaw should have changed
    }

    #[test]
    fn test_update_player_position_circle_movement() {
        let mut state = PlayerState::default();

        // Test movement over time
        update_player_position(&mut state, "circle", 0.5);
        let pos1 = state.position;
        let yaw1 = state.yaw;

        update_player_position(&mut state, "circle", 0.5);
        let pos2 = state.position;
        let yaw2 = state.yaw;

        assert_ne!(pos1, pos2);
        assert_ne!(yaw1, yaw2);
        assert_eq!(state.movement_time, 1.0);
    }

    #[test]
    fn test_pose_message_serialization() {
        let pose = PoseMessage {
            player_id: "test-player".to_string(),
            player_name: "TestPlayer".to_string(),
            pos: [1.0, 2.0, 3.0],
            yaw: 1.57,
            pitch: 0.0,
            ts: 1234567890,
        };

        let json = serde_json::to_string(&pose).unwrap();
        let deserialized: PoseMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(pose.player_id, deserialized.player_id);
        assert_eq!(pose.player_name, deserialized.player_name);
        assert_eq!(pose.pos, deserialized.pos);
        assert_eq!(pose.yaw, deserialized.yaw);
        assert_eq!(pose.pitch, deserialized.pitch);
        assert_eq!(pose.ts, deserialized.ts);
    }

    #[test]
    fn test_device_announcement_serialization() {
        let announcement = DeviceAnnouncement {
            device_id: "test-device".to_string(),
            device_type: "lamp".to_string(),
            state: "online".to_string(),
            location: DeviceLocation {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
        };

        let json = serde_json::to_string(&announcement).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["device_id"], "test-device");
        assert_eq!(parsed["device_type"], "lamp");
        assert_eq!(parsed["state"], "online");
        assert_eq!(parsed["location"]["x"], 1.0);
        assert_eq!(parsed["location"]["y"], 2.0);
        assert_eq!(parsed["location"]["z"], 3.0);
    }

    #[test]
    fn test_position_update_deserialization() {
        let json = r#"{"x": 5.5, "y": 2.5, "z": 10.0}"#;
        let update: PositionUpdate = serde_json::from_str(json).unwrap();

        assert_eq!(update.x, 5.5);
        assert_eq!(update.y, 2.5);
        assert_eq!(update.z, 10.0);
    }

    #[test]
    fn test_player_state_with_custom_position() {
        let initial_position = [10.0, 5.0, 15.0];
        let state = PlayerState {
            position: initial_position,
            ..Default::default()
        };

        assert_eq!(state.position, initial_position);
        assert_eq!(state.yaw, 0.0);
        assert_eq!(state.pitch, 0.0);
        assert_eq!(state.movement_time, 0.0);
    }

    #[test]
    fn test_random_movement_changes_over_time() {
        let mut state = PlayerState::default();
        let initial_position = state.position;

        // Simulate multiple small time steps that don't trigger random movement
        for _ in 0..10 {
            update_player_position(&mut state, "random", 0.1);
        }

        // Position should still be the same
        assert_eq!(state.position, initial_position);

        // Now trigger a random movement with a larger time step
        update_player_position(&mut state, "random", 3.5);

        // Note: Due to randomness, we can't test exact values, but we can test
        // that movement_time was updated correctly
        assert!(state.movement_time > 4.0);
    }

    #[test]
    fn test_circular_movement_maintains_radius() {
        let mut state = PlayerState::default();

        update_player_position(&mut state, "circle", std::f32::consts::PI);

        // After π seconds with speed 0.5, we should be at angle π/2
        let radius = (state.position[0].powi(2) + state.position[2].powi(2)).sqrt();
        assert!((radius - 5.0).abs() < 0.01); // Should maintain radius of 5.0
    }
}
