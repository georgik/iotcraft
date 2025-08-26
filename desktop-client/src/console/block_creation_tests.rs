use bevy::prelude::*;
use bevy_console::{ConsoleCommand, ConsoleSet, PrintConsoleLine};
use serde_json::json;
use std::time::Duration;

use crate::{
    console::{console_systems::*, console_types::*},
    environment::{BlockType, VoxelWorld, VoxelBlock},
    inventory::PlayerInventory,
    mcp::mcp_types::CommandExecutedEvent,
    mqtt::{MqttPlugin, MqttConfig},
    player_avatar::PlayerAvatarPlugin,
    world::WorldPlugin,
    devices::{DeviceEntity, DevicePositionUpdateEvent},
    multiplayer::{MultiplayerPlugin, SharedWorldPlugin, WorldDiscoveryPlugin, WorldPublisherPlugin},
    player_controller::PlayerControllerPlugin,
    camera_controllers::CameraControllerPlugin,
    interaction::InteractionPlugin as MyInteractionPlugin,
    inventory::InventoryPlugin,
    ui::{CrosshairPlugin, ErrorIndicatorPlugin, GameState, InventoryUiPlugin, MainMenuPlugin},
    minimap::MinimapPlugin,
    physics::PhysicsManagerPlugin,
    shared_materials::SharedMaterialsPlugin,
    fonts::FontPlugin,
    localization::LocalizationPlugin,
    config::MqttConfig as ConfigMqttConfig,
    profile::PlayerProfile,
    script::script_systems::ScriptPlugin,
    mcp::McpPlugin,
};

// Test for the place_block console command
#[test]
fn test_place_block_command() {
    // Setup a minimal Bevy app for testing
    let mut app = App::new();
    
    // Add required plugins and resources
    app.add_plugins(DefaultPlugins)
        .add_plugins(CameraControllerPlugin)
        .add_plugins(PlayerControllerPlugin)
        .add_plugins(InteractionPlugin)
        .add_plugins(MqttPlugin)
        .add_plugins(InventoryPlugin)
        .add_plugins(CrosshairPlugin)
        .add_plugins(ErrorIndicatorPlugin)
        .add_plugins(MainMenuPlugin)
        .add_plugins(MinimapPlugin)
        .add_plugins(WorldPlugin)
        .add_plugins(MultiplayerPlugin)
        .add_plugins(SharedWorldPlugin)
        .add_plugins(WorldDiscoveryPlugin)
        .add_plugins(WorldPublisherPlugin)
        .add_plugins(PlayerAvatarPlugin)
        .add_plugins(PhysicsManagerPlugin)
        .add_plugins(SharedMaterialsPlugin)
        .add_plugins(FontPlugin)
        .add_plugins(LocalizationPlugin)
        .add_plugins(ScriptPlugin)
        .add_plugins(McpPlugin)
        .insert_resource(ConfigMqttConfig::from_env_with_override(None))
        .insert_resource(PlayerProfile {
            player_id: "test_player".to_string(),
            player_name: "Test Player".to_string(),
        })
        .init_resource::<VoxelWorld>()
        .init_resource::<PlayerInventory>()
        .init_resource::<crate::console::BlinkState>()
        .init_resource::<crate::ui::error_indicator::ErrorResource>()
        .init_resource::<crate::environment::ThermometerMaterial>()
        .init_resource::<crate::script::script_types::PendingCommands>()
        .add_event::<CommandExecutedEvent>()
        .add_event::<DevicePositionUpdateEvent>()
        .add_console_command::<PlaceBlockCommand, _>(handle_place_block_command)
        .init_state::<GameState>();
    
    // Test placing a grass block at position (0, 0, 0)
    let mut console_command = ConsoleCommand::<PlaceBlockCommand>::new();
    console_command.set(PlaceBlockCommand {
        block_type: "grass".to_string(),
        x: 0,
        y: 0,
        z: 0,
    });
    
    // Process the command
    app.world_mut().send_event(console_command);
    
    // Run update systems to process the command
    app.update();
    
    // Check that the block was placed in the VoxelWorld resource
    let voxel_world = app.world().get_resource::<VoxelWorld>().unwrap();
    assert!(voxel_world.is_block_at(IVec3::new(0, 0, 0)));
    assert_eq!(
        voxel_world.blocks.get(&IVec3::new(0, 0, 0)).unwrap(),
        &BlockType::Grass
    );
    
    // Check that the block was added to the world scene (this is harder to test without more complex setup)
    // But we can at least verify the resource was updated
}

// Test for the wall command with proper coordinate ordering
#[test]
fn test_wall_command_proper_ordering() {
    let mut app = App::new();
    
    // Add required plugins and resources
    app.add_plugins(DefaultPlugins)
        .add_plugins(CameraControllerPlugin)
        .add_plugins(PlayerControllerPlugin)
        .add_plugins(InteractionPlugin)
        .add_plugins(MqttPlugin)
        .add_plugins(InventoryPlugin)
        .add_plugins(CrosshairPlugin)
        .add_plugins(ErrorIndicatorPlugin)
        .add_plugins(MainMenuPlugin)
        .add_plugins(MinimapPlugin)
        .add_plugins(WorldPlugin)
        .add_plugins(MultiplayerPlugin)
        .add_plugins(SharedWorldPlugin)
        .add_plugins(WorldDiscoveryPlugin)
        .add_plugins(WorldPublisherPlugin)
        .add_plugins(PlayerAvatarPlugin)
        .add_plugins(PhysicsManagerPlugin)
        .add_plugins(SharedMaterialsPlugin)
        .add_plugins(FontPlugin)
        .add_plugins(LocalizationPlugin)
        .add_plugins(ScriptPlugin)
        .add_plugins(McpPlugin)
        .insert_resource(ConfigMqttConfig::from_env_with_override(None))
        .insert_resource(PlayerProfile {
            player_id: "test_player".to_string(),
            player_name: "Test Player".to_string(),
        })
        .init_resource::<VoxelWorld>()
        .init_resource::<PlayerInventory>()
        .init_resource::<crate::console::BlinkState>()
        .init_resource::<crate::ui::error_indicator::ErrorResource>()
        .init_resource::<crate::environment::ThermometerMaterial>()
        .init_resource::<crate::script::script_types::PendingCommands>()
        .add_event::<CommandExecutedEvent>()
        .add_event::<DevicePositionUpdateEvent>()
        .add_console_command::<WallCommand, _>(handle_wall_command)
        .init_state::<GameState>();
    
    // Test creating a wall with proper coordinate ordering
    let mut console_command = ConsoleCommand::<WallCommand>::new();
    console_command.set(WallCommand {
        block_type: "stone".to_string(),
        x1: 0,
        y1: 0,
        z1: 0,
        x2: 2,
        y2: 0,
        z2: 2,
    });
    
    // Process the command
    app.world_mut().send_event(console_command);
    
    // Run update systems to process the command
    app.update();
    
    // Check that the wall was created in the VoxelWorld resource
    let voxel_world = app.world().get_resource::<VoxelWorld>().unwrap();
    
    // Should have created 3x1x3 = 9 blocks
    assert_eq!(voxel_world.blocks.len(), 9);
    
    // Check that all blocks in the wall are placed correctly
    for x in 0..=2 {
        for z in 0..=2 {
            assert!(voxel_world.is_block_at(IVec3::new(x, 0, z)));
            assert_eq!(
                voxel_world.blocks.get(&IVec3::new(x, 0, z)).unwrap(),
                &BlockType::Stone
            );
        }
    }
}

// Test for the wall command with backwards coordinate ordering (regression test)
#[test]
fn test_wall_command_backward_ordering_regression() {
    let mut app = App::new();
    
    // Add required plugins and resources
    app.add_plugins(DefaultPlugins)
        .add_plugins(CameraControllerPlugin)
        .add_plugins(PlayerControllerPlugin)
        .add_plugins(InteractionPlugin)
        .add_plugins(MqttPlugin)
        .add_plugins(InventoryPlugin)
        .add_plugins(CrosshairPlugin)
        .add_plugins(ErrorIndicatorPlugin)
        .add_plugins(MainMenuPlugin)
        .add_plugins(MinimapPlugin)
        .add_plugins(WorldPlugin)
        .add_plugins(MultiplayerPlugin)
        .add_plugins(SharedWorldPlugin)
        .add_plugins(WorldDiscoveryPlugin)
        .add_plugins(WorldPublisherPlugin)
        .add_plugins(PlayerAvatarPlugin)
        .add_plugins(PhysicsManagerPlugin)
        .add_plugins(SharedMaterialsPlugin)
        .add_plugins(FontPlugin)
        .add_plugins(LocalizationPlugin)
        .add_plugins(ScriptPlugin)
        .add_plugins(McpPlugin)
        .insert_resource(ConfigMqttConfig::from_env_with_override(None))
        .insert_resource(PlayerProfile {
            player_id: "test_player".to_string(),
            player_name: "Test Player".to_string(),
        })
        .init_resource::<VoxelWorld>()
        .init_resource::<PlayerInventory>()
        .init_resource::<crate::console::BlinkState>()
        .init_resource::<crate::ui::error_indicator::ErrorResource>()
        .init_resource::<crate::environment::ThermometerMaterial>()
        .init_resource::<crate::script::script_types::PendingCommands>()
        .add_event::<CommandExecutedEvent>()
        .add_event::<DevicePositionUpdateEvent>()
        .add_console_command::<WallCommand, _>(handle_wall_command)
        .init_state::<GameState>();
    
    // Test creating a wall with backwards coordinate ordering (this was the bug)
    // This should create 0 blocks because -21 > -26, so the range is invalid
    let mut console_command = ConsoleCommand::<WallCommand>::new();
    console_command.set(WallCommand {
        block_type: "stone".to_string(),
        x1: 21,
        y1: 1,
        z1: -21,
        x2: 26,
        y2: 1,
        z2: -26, // This creates an invalid range since -21 > -26
    });
    
    // Process the command
    app.world_mut().send_event(console_command);
    
    // Run update systems to process the command
    app.update();
    
    // Check that no blocks were created (this was the bug - it would create 0 blocks but not properly)
    let voxel_world = app.world().get_resource::<VoxelWorld>().unwrap();
    
    // Should have created 0 blocks due to invalid coordinate range
    assert_eq!(voxel_world.blocks.len(), 0);
}

// Test for the remove_block command
#[test]
fn test_remove_block_command() {
    let mut app = App::new();
    
    // Add required plugins and resources
    app.add_plugins(DefaultPlugins)
        .add_plugins(CameraControllerPlugin)
        .add_plugins(PlayerControllerPlugin)
        .add_plugins(InteractionPlugin)
        .add_plugins(MqttPlugin)
        .add_plugins(InventoryPlugin)
        .add_plugins(CrosshairPlugin)
        .add_plugins(ErrorIndicatorPlugin)
        .add_plugins(MainMenuPlugin)
        .add_plugins(MinimapPlugin)
        .add_plugins(WorldPlugin)
        .add_plugins(MultiplayerPlugin)
        .add_plugins(SharedWorldPlugin)
        .add_plugins(WorldDiscoveryPlugin)
        .add_plugins(WorldPublisherPlugin)
        .add_plugins(PlayerAvatarPlugin)
        .add_plugins(PhysicsManagerPlugin)
        .add_plugins(SharedMaterialsPlugin)
        .add_plugins(FontPlugin)
        .add_plugins(LocalizationPlugin)
        .add_plugins(ScriptPlugin)
        .add_plugins(McpPlugin)
        .insert_resource(ConfigMqttConfig::from_env_with_override(None))
        .insert_resource(PlayerProfile {
            player_id: "test_player".to_string(),
            player_name: "Test Player".to_string(),
        })
        .init_resource::<VoxelWorld>()
        .init_resource::<PlayerInventory>()
        .init_resource::<crate::console::BlinkState>()
        .init_resource::<crate::ui::error_indicator::ErrorResource>()
        .init_resource::<crate::environment::ThermometerMaterial>()
        .init_resource::<crate::script::script_types::PendingCommands>()
        .add_event::<CommandExecutedEvent>()
        .add_event::<DevicePositionUpdateEvent>()
        .add_console_command::<RemoveBlockCommand, _>(handle_remove_block_command)
        .init_state::<GameState>();
    
    // First place a block to remove it
    let mut place_command = ConsoleCommand::<PlaceBlockCommand>::new();
    place_command.set(PlaceBlockCommand {
        block_type: "dirt".to_string(),
        x: 5,
        y: 5,
        z: 5,
    });
    
    // Process the place command
    app.world_mut().send_event(place_command);
    app.update();
    
    // Verify the block was placed
    let voxel_world = app.world().get_resource::<VoxelWorld>().unwrap();
    assert!(voxel_world.is_block_at(IVec3::new(5, 5, 5)));
    
    // Now remove the block
    let mut remove_command = ConsoleCommand::<RemoveBlockCommand>::new();
    remove_command.set(RemoveBlockCommand {
        x: 5,
        y: 5,
        z: 5,
    });
    
    // Process the remove command
    app.world_mut().send_event(remove_command);
    app.update();
    
    // Verify the block was removed
    let voxel_world = app.world().get_resource::<VoxelWorld>().unwrap();
    assert!(!voxel_world.is_block_at(IVec3::new(5, 5, 5)));
    assert_eq!(voxel_world.blocks.len(), 0);
}