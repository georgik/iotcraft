#[cfg(test)]
mod tests {
    use crate::environment::BlockType;
    use crate::inventory::PlayerInventory;
    use crate::multiplayer::shared_world::*;
    use crate::world::{VoxelBlockData, WorldMetadata, WorldSaveData};
    use bevy::prelude::*;

    fn create_sample_world_save_data() -> WorldSaveData {
        let metadata = WorldMetadata {
            name: "Test World".to_string(),
            description: "A test world for unit tests".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            last_played: "2024-01-01T12:00:00Z".to_string(),
            version: "1.0".to_string(),
        };

        let blocks = vec![
            VoxelBlockData {
                x: 0,
                y: 0,
                z: 0,
                block_type: BlockType::Grass,
            },
            VoxelBlockData {
                x: 1,
                y: 1,
                z: 1,
                block_type: BlockType::Stone,
            },
            VoxelBlockData {
                x: -1,
                y: 2,
                z: -1,
                block_type: BlockType::Dirt,
            },
        ];

        WorldSaveData {
            metadata,
            blocks,
            player_position: Vec3::new(0.5, 2.0, 0.5),
            player_rotation: Quat::IDENTITY,
            inventory: PlayerInventory::default(),
        }
    }

    fn create_sample_shared_world_info() -> SharedWorldInfo {
        SharedWorldInfo {
            world_id: "test_world_123".to_string(),
            world_name: "Test Multiplayer World".to_string(),
            description: "A test world for multiplayer testing".to_string(),
            host_player: "player_001".to_string(),
            host_name: "TestHost".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            last_updated: "2024-01-01T12:00:00Z".to_string(),
            player_count: 1,
            max_players: 4,
            is_public: true,
            version: "1.0".to_string(),
        }
    }

    #[test]
    fn test_shared_world_info_serialization() {
        let world_info = create_sample_shared_world_info();

        // Test JSON serialization
        let json_str = serde_json::to_string(&world_info).expect("Should serialize to JSON");
        assert!(json_str.contains("\"world_id\":\"test_world_123\""));
        assert!(json_str.contains("\"world_name\":\"Test Multiplayer World\""));
        assert!(json_str.contains("\"host_name\":\"TestHost\""));
        assert!(json_str.contains("\"is_public\":true"));
        assert!(json_str.contains("\"player_count\":1"));
        assert!(json_str.contains("\"max_players\":4"));

        // Test deserialization
        let deserialized: SharedWorldInfo =
            serde_json::from_str(&json_str).expect("Should deserialize from JSON");
        assert_eq!(deserialized.world_id, world_info.world_id);
        assert_eq!(deserialized.world_name, world_info.world_name);
        assert_eq!(deserialized.host_name, world_info.host_name);
        assert_eq!(deserialized.is_public, world_info.is_public);
        assert_eq!(deserialized.player_count, world_info.player_count);
        assert_eq!(deserialized.max_players, world_info.max_players);
    }

    #[test]
    fn test_world_save_data_serialization() {
        let world_data = create_sample_world_save_data();

        // Test JSON serialization
        let json_str = serde_json::to_string(&world_data).expect("Should serialize to JSON");
        assert!(json_str.contains("\"Test World\""));
        assert!(json_str.contains("\"blocks\":["));

        // Test deserialization
        let deserialized: WorldSaveData =
            serde_json::from_str(&json_str).expect("Should deserialize from JSON");
        assert_eq!(deserialized.metadata.name, "Test World");
        assert_eq!(deserialized.blocks.len(), 3);
        assert_eq!(deserialized.blocks[0].block_type, BlockType::Grass);
        assert_eq!(deserialized.player_position, Vec3::new(0.5, 2.0, 0.5));
    }

    #[test]
    fn test_shared_world_data_serialization() {
        let world_info = create_sample_shared_world_info();
        let world_data = create_sample_world_save_data();

        let shared_data = SharedWorldData {
            info: world_info,
            world_data,
        };

        // Test JSON serialization
        let json_str = serde_json::to_string(&shared_data).expect("Should serialize to JSON");
        assert!(json_str.contains("\"info\""));
        assert!(json_str.contains("\"world_data\""));

        // Test deserialization
        let deserialized: SharedWorldData =
            serde_json::from_str(&json_str).expect("Should deserialize from JSON");
        assert_eq!(deserialized.info.world_id, "test_world_123");
        assert_eq!(deserialized.world_data.metadata.name, "Test World");
    }

    #[test]
    fn test_world_change_serialization() {
        let world_change = WorldChange {
            change_id: "change_001".to_string(),
            world_id: "test_world_123".to_string(),
            player_id: "player_001".to_string(),
            player_name: "TestPlayer".to_string(),
            timestamp: 1704067200000, // 2024-01-01T00:00:00Z in milliseconds
            change_type: WorldChangeType::BlockPlaced {
                x: 5,
                y: 10,
                z: -3,
                block_type: BlockType::Stone,
            },
        };

        // Test JSON serialization
        let json_str = serde_json::to_string(&world_change).expect("Should serialize to JSON");
        assert!(json_str.contains("\"change_id\":\"change_001\""));
        assert!(json_str.contains("\"world_id\":\"test_world_123\""));
        assert!(json_str.contains("\"BlockPlaced\""));
        assert!(json_str.contains("\"Stone\""));

        // Test deserialization
        let deserialized: WorldChange =
            serde_json::from_str(&json_str).expect("Should deserialize from JSON");
        assert_eq!(deserialized.change_id, "change_001");
        assert_eq!(deserialized.world_id, "test_world_123");
        assert_eq!(deserialized.player_name, "TestPlayer");

        match deserialized.change_type {
            WorldChangeType::BlockPlaced {
                x,
                y,
                z,
                block_type,
            } => {
                assert_eq!(x, 5);
                assert_eq!(y, 10);
                assert_eq!(z, -3);
                assert_eq!(block_type, BlockType::Stone);
            }
            _ => panic!("Wrong change type deserialized"),
        }
    }

    #[test]
    fn test_world_change_block_removed() {
        let world_change = WorldChange {
            change_id: "change_002".to_string(),
            world_id: "test_world_123".to_string(),
            player_id: "player_001".to_string(),
            player_name: "TestPlayer".to_string(),
            timestamp: 1704067200000,
            change_type: WorldChangeType::BlockRemoved { x: -2, y: 5, z: 8 },
        };

        // Test JSON serialization
        let json_str = serde_json::to_string(&world_change).expect("Should serialize to JSON");
        assert!(json_str.contains("\"BlockRemoved\""));
        assert!(json_str.contains("\"x\":-2"));
        assert!(json_str.contains("\"y\":5"));
        assert!(json_str.contains("\"z\":8"));

        // Test deserialization
        let deserialized: WorldChange =
            serde_json::from_str(&json_str).expect("Should deserialize from JSON");

        match deserialized.change_type {
            WorldChangeType::BlockRemoved { x, y, z } => {
                assert_eq!(x, -2);
                assert_eq!(y, 5);
                assert_eq!(z, 8);
            }
            _ => panic!("Wrong change type deserialized"),
        }
    }

    #[test]
    fn test_world_change_player_events() {
        let join_change = WorldChange {
            change_id: "change_003".to_string(),
            world_id: "test_world_123".to_string(),
            player_id: "player_002".to_string(),
            player_name: "NewPlayer".to_string(),
            timestamp: 1704067200000,
            change_type: WorldChangeType::PlayerJoined {
                player_id: "player_002".to_string(),
                player_name: "NewPlayer".to_string(),
            },
        };

        let leave_change = WorldChange {
            change_id: "change_004".to_string(),
            world_id: "test_world_123".to_string(),
            player_id: "player_002".to_string(),
            player_name: "NewPlayer".to_string(),
            timestamp: 1704067300000,
            change_type: WorldChangeType::PlayerLeft {
                player_id: "player_002".to_string(),
                player_name: "NewPlayer".to_string(),
            },
        };

        // Test serialization of both events
        let join_json = serde_json::to_string(&join_change).expect("Should serialize join event");
        let leave_json =
            serde_json::to_string(&leave_change).expect("Should serialize leave event");

        assert!(join_json.contains("\"PlayerJoined\""));
        assert!(leave_json.contains("\"PlayerLeft\""));

        // Test deserialization
        let join_deserialized: WorldChange =
            serde_json::from_str(&join_json).expect("Should deserialize join event");
        let leave_deserialized: WorldChange =
            serde_json::from_str(&leave_json).expect("Should deserialize leave event");

        match join_deserialized.change_type {
            WorldChangeType::PlayerJoined {
                player_id,
                player_name,
            } => {
                assert_eq!(player_id, "player_002");
                assert_eq!(player_name, "NewPlayer");
            }
            _ => panic!("Wrong change type for join event"),
        }

        match leave_deserialized.change_type {
            WorldChangeType::PlayerLeft {
                player_id,
                player_name,
            } => {
                assert_eq!(player_id, "player_002");
                assert_eq!(player_name, "NewPlayer");
            }
            _ => panic!("Wrong change type for leave event"),
        }
    }

    #[test]
    fn test_multiplayer_mode_default() {
        let mode = MultiplayerMode::default();
        assert_eq!(mode, MultiplayerMode::SinglePlayer);
    }

    #[test]
    fn test_multiplayer_mode_equality() {
        let single_player = MultiplayerMode::SinglePlayer;
        let hosting = MultiplayerMode::HostingWorld {
            world_id: "world_123".to_string(),
            is_published: true,
        };
        let hosting_same = MultiplayerMode::HostingWorld {
            world_id: "world_123".to_string(),
            is_published: true,
        };
        let hosting_different = MultiplayerMode::HostingWorld {
            world_id: "world_456".to_string(),
            is_published: true,
        };
        let joined = MultiplayerMode::JoinedWorld {
            world_id: "world_123".to_string(),
            host_player: "host_001".to_string(),
        };

        assert_eq!(single_player, MultiplayerMode::SinglePlayer);
        assert_eq!(hosting, hosting_same);
        assert_ne!(hosting, hosting_different);
        assert_ne!(hosting, joined);
        assert_ne!(single_player, hosting);
    }

    #[test]
    fn test_online_worlds_default() {
        let online_worlds = OnlineWorlds::default();
        assert_eq!(online_worlds.worlds.len(), 0);
        assert!(online_worlds.last_updated.is_none());
    }

    #[test]
    fn test_online_worlds_operations() {
        let mut online_worlds = OnlineWorlds::default();
        let world_info = create_sample_shared_world_info();

        // Add world
        online_worlds
            .worlds
            .insert(world_info.world_id.clone(), world_info.clone());
        assert_eq!(online_worlds.worlds.len(), 1);
        assert_eq!(
            online_worlds.worlds.get("test_world_123"),
            Some(&world_info)
        );

        // Update timestamp
        online_worlds.last_updated = Some(std::time::Instant::now());
        assert!(online_worlds.last_updated.is_some());

        // Remove world
        online_worlds.worlds.remove("test_world_123");
        assert_eq!(online_worlds.worlds.len(), 0);
    }

    #[test]
    fn test_block_change_type_serialization() {
        let placed = BlockChangeType::Placed {
            x: 10,
            y: 20,
            z: 30,
            block_type: BlockType::QuartzBlock,
        };

        let removed = BlockChangeType::Removed {
            x: -5,
            y: -10,
            z: -15,
        };

        // Test serialization
        let placed_json = serde_json::to_string(&placed).expect("Should serialize placed");
        let removed_json = serde_json::to_string(&removed).expect("Should serialize removed");

        assert!(placed_json.contains("\"Placed\""));
        assert!(placed_json.contains("\"QuartzBlock\""));
        assert!(removed_json.contains("\"Removed\""));

        // Test deserialization
        let placed_deserialized: BlockChangeType =
            serde_json::from_str(&placed_json).expect("Should deserialize placed");
        let removed_deserialized: BlockChangeType =
            serde_json::from_str(&removed_json).expect("Should deserialize removed");

        match placed_deserialized {
            BlockChangeType::Placed {
                x,
                y,
                z,
                block_type,
            } => {
                assert_eq!(x, 10);
                assert_eq!(y, 20);
                assert_eq!(z, 30);
                assert_eq!(block_type, BlockType::QuartzBlock);
            }
            _ => panic!("Wrong deserialization result for placed"),
        }

        match removed_deserialized {
            BlockChangeType::Removed { x, y, z } => {
                assert_eq!(x, -5);
                assert_eq!(y, -10);
                assert_eq!(z, -15);
            }
            _ => panic!("Wrong deserialization result for removed"),
        }
    }

    #[test]
    fn test_world_info_validation() {
        let mut world_info = create_sample_shared_world_info();

        // Test valid world info
        assert!(!world_info.world_id.is_empty());
        assert!(!world_info.world_name.is_empty());
        assert!(!world_info.host_name.is_empty());
        assert!(world_info.max_players > 0);
        assert!(world_info.player_count <= world_info.max_players);

        // Test edge cases
        world_info.player_count = world_info.max_players;
        assert_eq!(world_info.player_count, world_info.max_players);

        // Serialization should still work with edge values
        let json_str = serde_json::to_string(&world_info).expect("Should serialize edge case");
        let _deserialized: SharedWorldInfo =
            serde_json::from_str(&json_str).expect("Should deserialize edge case");
    }

    #[test]
    fn test_large_world_serialization() {
        // Create a world with many blocks to test serialization performance
        let metadata = WorldMetadata {
            name: "Large Test World".to_string(),
            description: "A large test world".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            last_played: "2024-01-01T12:00:00Z".to_string(),
            version: "1.0".to_string(),
        };

        let mut blocks = Vec::new();
        let block_types = [
            BlockType::Grass,
            BlockType::Stone,
            BlockType::Dirt,
            BlockType::QuartzBlock,
            BlockType::GlassPane,
            BlockType::CyanTerracotta,
            BlockType::Water,
        ];

        // Create a 10x10x10 grid of blocks
        for x in 0..10 {
            for y in 0..10 {
                for z in 0..10 {
                    blocks.push(VoxelBlockData {
                        x,
                        y,
                        z,
                        block_type: block_types[(x + y + z) as usize % block_types.len()],
                    });
                }
            }
        }

        let large_world_data = WorldSaveData {
            metadata,
            blocks,
            player_position: Vec3::new(5.0, 12.0, 5.0),
            player_rotation: Quat::from_rotation_y(std::f32::consts::PI / 4.0),
            inventory: PlayerInventory::default(),
        };

        // Test serialization of large world
        let json_str =
            serde_json::to_string(&large_world_data).expect("Should serialize large world");
        assert!(!json_str.is_empty());
        assert!(json_str.contains("Large Test World"));
        assert!(json_str.len() > 1000); // Should be a substantial JSON string

        // Test deserialization of large world
        let deserialized: WorldSaveData =
            serde_json::from_str(&json_str).expect("Should deserialize large world");
        assert_eq!(deserialized.metadata.name, "Large Test World");
        assert_eq!(deserialized.blocks.len(), 1000); // 10x10x10 = 1000 blocks
        assert!(deserialized.player_position.y > 10.0);
    }
}
