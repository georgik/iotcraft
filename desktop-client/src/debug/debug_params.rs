// Parameter bundles for debug and diagnostics systems
// This reduces system parameter counts for Bevy compliance

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

#[cfg(feature = "console")]
use crate::console::ConsoleManager;

use crate::camera_controllers::CameraController;
use crate::devices::DeviceEntity;
use crate::environment::VoxelWorld;
use crate::fonts::Fonts;
use crate::inventory::PlayerInventory;
use crate::mqtt::TemperatureResource;
use crate::multiplayer::{
    MultiplayerConnectionStatus, MultiplayerMode, RemotePlayer, WorldDiscovery,
};
use crate::player_avatar::PlayerAvatar;
use crate::profile::PlayerProfile;

/// Parameter bundle for core debug UI setup
#[derive(SystemParam)]
pub struct CoreDebugParams<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub fonts: Res<'w, Fonts>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Parameter bundle for debug visibility and toggle handling
#[derive(SystemParam)]
pub struct DebugToggleParams<'w, 's> {
    pub keyboard_input: Res<'w, ButtonInput<KeyCode>>,
    pub diagnostics_visible: ResMut<'w, DiagnosticsVisible>,
    pub diagnostics_query: Query<'w, 's, &'static mut Visibility, With<DiagnosticsOverlay>>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Parameter bundle for game state and world information
#[derive(SystemParam)]
pub struct GameStateDebugParams<'w, 's> {
    pub voxel_world: Res<'w, VoxelWorld>,
    pub inventory: Res<'w, PlayerInventory>,
    pub device_query: Query<'w, 's, &'static DeviceEntity>,
    pub time: Res<'w, Time>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Parameter bundle for player information (camera and avatars)
#[derive(SystemParam)]
pub struct PlayerDebugParams<'w, 's> {
    pub camera_query: Query<'w, 's, (&'static Transform, &'static CameraController), With<Camera>>,
    pub player_avatar_query:
        Query<'w, 's, (&'static Transform, &'static PlayerAvatar), With<RemotePlayer>>,
    pub local_profile: Res<'w, PlayerProfile>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Parameter bundle for multiplayer and network information
#[derive(SystemParam)]
pub struct MultiplayerDebugParams<'w, 's> {
    pub temperature: Res<'w, TemperatureResource>,
    pub multiplayer_mode: Res<'w, MultiplayerMode>,
    pub multiplayer_status: Res<'w, MultiplayerConnectionStatus>,
    pub world_discovery: Res<'w, WorldDiscovery>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Parameter bundle for diagnostic text display updates
#[derive(SystemParam)]
pub struct DiagnosticDisplayParams<'w, 's> {
    pub diagnostics_visible: Res<'w, DiagnosticsVisible>,
    pub diagnostics_text_query: Query<'w, 's, &'static mut Text, With<DiagnosticsText>>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Parameter bundle for console state checking (only when console feature enabled)
#[cfg(feature = "console")]
#[derive(SystemParam)]
pub struct ConsoleDebugParams<'w, 's> {
    pub console_manager: Option<Res<'w, ConsoleManager>>,
    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

/// Parameter bundle for all diagnostics systems - combines multiple param bundles
#[derive(SystemParam)]
pub struct ComprehensiveDebugParams<'w, 's> {
    pub display: DiagnosticDisplayParams<'w, 's>,
    pub game_state: GameStateDebugParams<'w, 's>,
    pub player: PlayerDebugParams<'w, 's>,
    pub multiplayer: MultiplayerDebugParams<'w, 's>,

    #[cfg(feature = "console")]
    pub console: ConsoleDebugParams<'w, 's>,

    // PhantomData to use the 's lifetime
    _phantom: std::marker::PhantomData<&'s ()>,
}

// Resource to track diagnostics visibility
#[derive(Resource)]
pub struct DiagnosticsVisible {
    pub visible: bool,
}

impl Default for DiagnosticsVisible {
    fn default() -> Self {
        Self { visible: false }
    }
}

// Components for diagnostics UI
#[derive(Component)]
pub struct DiagnosticsText;

#[derive(Component)]
pub struct DiagnosticsOverlay;

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{MinimalPlugins, app::App};

    #[test]
    fn test_core_debug_params_compiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        // Test that the parameter bundle compiles
        fn test_system(_params: CoreDebugParams) {}

        app.add_systems(Update, test_system);
        // No need to run, just compile
    }

    #[test]
    fn test_debug_toggle_params_compiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<DiagnosticsVisible>();

        fn test_system(_params: DebugToggleParams) {}

        app.add_systems(Update, test_system);
    }

    #[test]
    fn test_comprehensive_debug_params_compiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.init_resource::<DiagnosticsVisible>();

        fn test_system(_params: ComprehensiveDebugParams) {}

        app.add_systems(Update, test_system);
    }

    #[test]
    fn test_diagnostics_visible_default() {
        let diagnostics = DiagnosticsVisible::default();
        assert!(!diagnostics.visible);
    }
}
