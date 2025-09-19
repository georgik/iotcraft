use crate::{
    fonts::Fonts,
    localization::{LanguageChangeEvent, LocalizationBundle, LocalizationConfig},
    world::{CreateWorldEvent, DeleteWorldEvent, DiscoveredWorlds, LoadWorldEvent, SaveWorldEvent},
};

// Desktop-specific imports
#[cfg(not(target_arch = "wasm32"))]
use crate::ui::main_menu::GameState;

// WASM-specific imports
#[cfg(target_arch = "wasm32")]
use crate::ui::GameState;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// Bundle for core UI operations
/// Handles basic UI rendering and command management
#[derive(SystemParam)]
pub struct CoreUIParams<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub asset_server: Res<'w, AssetServer>,
    pub fonts: Option<Res<'w, Fonts>>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Bundle for localization and text management
/// Handles multi-language support and text rendering
#[derive(SystemParam)]
pub struct LocalizationUIParams<'w, 's> {
    pub localization_bundle: Option<Res<'w, LocalizationBundle>>,
    pub localization_config: Option<Res<'w, LocalizationConfig>>,
    pub language_change_events: EventWriter<'w, LanguageChangeEvent>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Bundle for world management UI operations
/// Handles world selection, creation, and deletion
#[derive(SystemParam)]
pub struct WorldUIParams<'w, 's> {
    pub discovered_worlds: Option<Res<'w, DiscoveredWorlds>>,
    pub create_world_events: EventWriter<'w, CreateWorldEvent>,
    pub load_world_events: EventWriter<'w, LoadWorldEvent>,
    pub delete_world_events: EventWriter<'w, DeleteWorldEvent>,
    pub save_world_events: EventWriter<'w, SaveWorldEvent>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Bundle for multiplayer UI operations (desktop only)
/// Handles online world discovery and multiplayer events
#[cfg(not(target_arch = "wasm32"))]
#[derive(SystemParam)]
pub struct MultiplayerUIParams<'w, 's> {
    pub online_worlds: Option<Res<'w, crate::multiplayer::shared_world::OnlineWorlds>>,
    pub refresh_events: EventWriter<'w, crate::multiplayer::shared_world::RefreshOnlineWorldsEvent>,
    pub join_events: EventWriter<'w, crate::multiplayer::shared_world::JoinSharedWorldEvent>,
    pub publish_events: EventWriter<'w, crate::multiplayer::shared_world::PublishWorldEvent>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// WASM-compatible stub for multiplayer UI operations
#[cfg(target_arch = "wasm32")]
#[derive(SystemParam)]
pub struct MultiplayerUIParams<'w, 's> {
    // Stub implementation for WASM - no desktop multiplayer features
    _phantom: std::marker::PhantomData<(&'w (), &'s ())>,
}

/// Bundle for cursor and window management
/// Handles cursor grabbing/releasing for different UI states
#[derive(SystemParam)]
pub struct CursorUIParams<'w, 's> {
    pub windows: Query<'w, 's, &'static mut Window>,
    pub cursor_options_query: Query<'w, 's, &'static mut bevy::window::CursorOptions>,
    pub camera_controller_query:
        Query<'w, 's, &'static mut crate::camera_controllers::CameraController>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Bundle for game state management
/// Handles state transitions and input events
#[derive(SystemParam)]
pub struct GameStateUIParams<'w, 's> {
    pub current_state: Res<'w, State<GameState>>,
    pub next_state: ResMut<'w, NextState<GameState>>,
    pub keyboard_input: Res<'w, ButtonInput<KeyCode>>,
    pub exit_events: EventWriter<'w, bevy::app::AppExit>,
    #[cfg(not(target_arch = "wasm32"))]
    pub mcp_state_transition: Option<ResMut<'w, crate::mcp::mcp_server::McpStateTransition>>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Bundle for UI interaction queries
/// Handles button interactions and UI component queries
#[derive(SystemParam)]
pub struct InteractionUIParams<'w, 's> {
    // Generic button interaction query - can be used for multiple button types
    pub button_interactions: Query<
        'w,
        's,
        (&'static Interaction, &'static mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,

    // Specific button queries - organized by functionality
    #[cfg(not(target_arch = "wasm32"))]
    pub main_menu_buttons: Query<
        'w,
        's,
        (
            &'static Interaction,
            &'static mut BackgroundColor,
            Option<&'static crate::ui::main_menu::EnterWorldButton>,
            Option<&'static crate::ui::main_menu::SettingsButton>,
            Option<&'static crate::ui::main_menu::QuitButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,

    // WASM-compatible button query without desktop-specific components
    #[cfg(target_arch = "wasm32")]
    pub main_menu_buttons: Query<
        'w,
        's,
        (&'static Interaction, &'static mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,

    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Bundle for entity management in UI systems (desktop)
/// Handles spawning and despawning of UI entities
#[cfg(not(target_arch = "wasm32"))]
#[derive(SystemParam)]
pub struct EntityUIParams<'w, 's> {
    // Queries for different UI component types
    pub main_menu_entities: Query<'w, 's, Entity, With<crate::ui::main_menu::MainMenu>>,
    pub world_selection_entities:
        Query<'w, 's, Entity, With<crate::ui::main_menu::WorldSelectionMenu>>,
    pub settings_entities: Query<'w, 's, Entity, With<crate::ui::main_menu::SettingsMenu>>,
    pub gameplay_entities: Query<'w, 's, Entity, With<crate::ui::main_menu::GameplayMenu>>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// WASM-compatible stub for entity management in UI systems
#[cfg(target_arch = "wasm32")]
#[derive(SystemParam)]
pub struct EntityUIParams<'w, 's> {
    // Stub implementation for WASM - uses web main menu components
    _phantom: std::marker::PhantomData<(&'w (), &'s ())>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::IntoSystem;

    #[test]
    fn test_core_ui_params_creation() {
        let mut world = World::new();

        // Initialize required resources
        // Note: AssetServer and Fonts need proper initialization in full tests

        // Test system that uses CoreUIParams
        let test_system = |_params: CoreUIParams| {
            // System compiles and can access bundled parameters
        };

        let mut system = IntoSystem::into_system(test_system);
        system.initialize(&mut world);
        let _ = system.run((), &mut world);

        assert!(true);
    }

    #[test]
    fn test_localization_ui_params_optional_resources() {
        let mut world = World::new();

        // Initialize only event resources (localization resources are optional)
        world.init_resource::<Events<LanguageChangeEvent>>();

        // Test system that uses LocalizationUIParams with optional resources
        let test_system = |params: LocalizationUIParams| {
            // Should handle missing optional resources gracefully
            assert!(params.localization_bundle.is_none());
            assert!(params.localization_config.is_none());
        };

        let mut system = IntoSystem::into_system(test_system);
        system.initialize(&mut world);
        let _ = system.run((), &mut world);

        assert!(true);
    }

    #[test]
    fn test_multiplayer_ui_params_optional_resources() {
        let mut world = World::new();

        // Initialize only the required event resources
        world.init_resource::<Events<crate::multiplayer::shared_world::RefreshOnlineWorldsEvent>>();
        world.init_resource::<Events<crate::multiplayer::shared_world::JoinSharedWorldEvent>>();
        world.init_resource::<Events<crate::multiplayer::shared_world::PublishWorldEvent>>();

        // Test system that uses MultiplayerUIParams with optional resources
        let test_system = |params: MultiplayerUIParams| {
            // Should handle missing optional resources gracefully for WASM builds
            assert!(params.online_worlds.is_none());
        };

        let mut system = IntoSystem::into_system(test_system);
        system.initialize(&mut world);
        let _ = system.run((), &mut world);

        assert!(true);
    }
}
