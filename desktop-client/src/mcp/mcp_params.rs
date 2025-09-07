use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
use crate::{
    config::MqttConfig,
    devices::device_types::DeviceEntity, 
    environment::VoxelWorld,
    mqtt::TemperatureResource,
    ui::main_menu::GameState,
    world::{CreateWorldEvent, LoadWorldEvent, world_types::{CurrentWorld, DiscoveredWorlds}},
};

// Import CommandExecutedEvent from the correct module
use crate::mcp::CommandExecutedEvent;

use super::mcp_types::PendingToolExecutions;

/// Bundle for core MCP command execution parameters
/// Handles basic MCP server operations and response management
#[derive(SystemParam)]
pub struct CoreMcpParams<'w, 's> {
    pub pending_executions: ResMut<'w, PendingToolExecutions>,
    pub command_executed_events: EventWriter<'w, CommandExecutedEvent>,
    pub temperature: Res<'w, TemperatureResource>,
    pub mqtt_config: Res<'w, MqttConfig>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Bundle for world management MCP operations
/// Handles world creation, loading, and block operations
#[derive(SystemParam)]
pub struct WorldMcpParams<'w, 's> {
    pub voxel_world: ResMut<'w, VoxelWorld>,
    pub create_world_events: EventWriter<'w, CreateWorldEvent>,
    pub load_world_events: EventWriter<'w, LoadWorldEvent>,
    pub current_world: Option<Res<'w, CurrentWorld>>,
    pub discovered_worlds: ResMut<'w, DiscoveredWorlds>,
    pub next_game_state: Option<ResMut<'w, NextState<GameState>>>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Bundle for multiplayer-related MCP commands
/// Optional resources to support both desktop and WASM builds
#[derive(SystemParam)]
pub struct MultiplayerMcpParams<'w, 's> {
    pub online_worlds: Option<Res<'w, crate::multiplayer::shared_world::OnlineWorlds>>,
    pub multiplayer_mode: Option<Res<'w, crate::multiplayer::shared_world::MultiplayerMode>>,
    pub refresh_events: EventWriter<'w, crate::multiplayer::shared_world::RefreshOnlineWorldsEvent>,
    pub join_events: EventWriter<'w, crate::multiplayer::shared_world::JoinSharedWorldEvent>, 
    pub leave_events: EventWriter<'w, crate::multiplayer::shared_world::LeaveSharedWorldEvent>,
    pub unpublish_events: EventWriter<'w, crate::multiplayer::shared_world::UnpublishWorldEvent>,
    pub publish_events: EventWriter<'w, crate::multiplayer::shared_world::PublishWorldEvent>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Bundle for rendering and entity management operations
/// Handles visual block creation and device queries
#[derive(SystemParam)]
pub struct EntityMcpParams<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub device_query: Query<'w, 's, (&'static DeviceEntity, &'static Transform), Without<Camera>>,
    pub camera_query: Query<'w, 's, (
        &'static mut Transform,
        &'static mut crate::camera_controllers::CameraController,
    ), With<Camera>>,
    pub existing_blocks_query: Query<'w, 's, Entity, With<crate::environment::VoxelBlock>>,
}

/// Bundle for MCP state management
/// Handles MCP-specific state transitions and flags
#[derive(SystemParam)]
pub struct McpStateMcpParams<'w, 's> {
    pub mcp_state_transition: ResMut<'w, super::mcp_server::McpStateTransition>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::IntoSystem;

    #[test]
    fn test_core_mcp_params_creation() {
        let mut world = World::new();
        
        // Initialize required resources
        world.insert_resource(PendingToolExecutions::default());
        world.init_resource::<Events<CommandExecutedEvent>>();
        world.insert_resource(TemperatureResource { value: Some(25.0) });
        world.insert_resource(MqttConfig::default());
        
        // Test system that uses CoreMcpParams
        let test_system = |_params: CoreMcpParams| {
            // System compiles and can access bundled parameters
        };
        
        let mut system = IntoSystem::into_system(test_system);
        system.initialize(&mut world);
        let _ = system.run((), &mut world);
        
        // If we reach here, parameter bundling works correctly
        assert!(true);
    }
    
    #[test]
    fn test_world_mcp_params_creation() {
        let mut world = World::new();
        
        // Initialize required resources
        world.insert_resource(VoxelWorld::default());
        world.init_resource::<Events<CreateWorldEvent>>();
        world.init_resource::<Events<LoadWorldEvent>>();
        world.insert_resource(DiscoveredWorlds::default());
        
        // Test system that uses WorldMcpParams  
        let test_system = |_params: WorldMcpParams| {
            // System compiles and can access bundled parameters
        };
        
        let mut system = IntoSystem::into_system(test_system);
        system.initialize(&mut world);
        let _ = system.run((), &mut world);
        
        assert!(true);
    }
    
    #[test]
    fn test_multiplayer_mcp_params_optional_resources() {
        let mut world = World::new();
        
        // Initialize only the required event resources (no optional Res<> resources)
        world.init_resource::<Events<crate::multiplayer::shared_world::RefreshOnlineWorldsEvent>>();
        world.init_resource::<Events<crate::multiplayer::shared_world::JoinSharedWorldEvent>>();
        world.init_resource::<Events<crate::multiplayer::shared_world::LeaveSharedWorldEvent>>();
        world.init_resource::<Events<crate::multiplayer::shared_world::UnpublishWorldEvent>>();
        world.init_resource::<Events<crate::multiplayer::shared_world::PublishWorldEvent>>();
        
        // Test system that uses MultiplayerMcpParams with optional resources
        let test_system = |params: MultiplayerMcpParams| {
            // Should handle missing optional resources gracefully
            assert!(params.online_worlds.is_none());
            assert!(params.multiplayer_mode.is_none());
        };
        
        let mut system = IntoSystem::into_system(test_system);
        system.initialize(&mut world);
        let _ = system.run((), &mut world);
        
        assert!(true);
    }
}
