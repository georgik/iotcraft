// IoTCraft Desktop Client - Web Version (Enhanced Gradual Build)
#[cfg(target_arch = "wasm32")]
mod lib_gradual;

#[cfg(target_arch = "wasm32")]
mod lib_debug;

#[cfg(target_arch = "wasm32")]
mod web_menu;

// Required modules for MQTT and profile support in WASM
#[cfg(target_arch = "wasm32")]
mod config;

#[cfg(target_arch = "wasm32")]
mod profile;

#[cfg(target_arch = "wasm32")]
mod mqtt;

#[cfg(target_arch = "wasm32")]
mod player_avatar;

#[cfg(target_arch = "wasm32")]
mod web_player_controller;

// Note: Multiplayer and full device functionality requires desktop-only dependencies
// For now, MQTT device announcements are handled in the web MQTT plugin

// Desktop-only modules for tests and library usage
#[cfg(not(target_arch = "wasm32"))]
pub mod multiplayer;

#[cfg(not(target_arch = "wasm32"))]
pub mod config;

#[cfg(not(target_arch = "wasm32"))]
pub mod profile;

// Environment and inventory modules available for both desktop and web
pub mod environment;

// World module available for both desktop and web
pub mod world;

// Inventory module available for both desktop and web
pub mod inventory;

// Multiplayer - available for both desktop and web with shared types
#[cfg(target_arch = "wasm32")]
pub mod multiplayer_web;

// Camera controllers module available for both desktop and web
pub mod camera_controllers;

// Console module available for both desktop and web
pub mod console;

#[cfg(not(target_arch = "wasm32"))]
pub mod devices;

#[cfg(not(target_arch = "wasm32"))]
pub mod interaction;

// Fonts module available for both desktop and web
pub mod fonts;

// Localization module available for both desktop and web
pub mod localization;

// Script module available for both desktop and web
pub mod script;

// UI module available for both desktop and web
pub mod ui;

// Shared materials module available for both desktop and web
pub mod shared_materials;

#[cfg(not(target_arch = "wasm32"))]
pub mod rendering;

#[cfg(not(target_arch = "wasm32"))]
pub mod mcp;

#[cfg(not(target_arch = "wasm32"))]
pub mod scenario_types;

#[cfg(not(target_arch = "wasm32"))]
pub mod mqtt;

#[cfg(not(target_arch = "wasm32"))]
pub mod reply {}

// Player controller module available for both desktop and web
pub mod player_controller;

#[cfg(not(target_arch = "wasm32"))]
pub mod player_avatar;

#[cfg(target_arch = "wasm32")]
pub use lib_gradual::*;

#[cfg(target_arch = "wasm32")]
pub use lib_debug::{debug_set_panic_hook, debug_start};

// Cross-platform tests that work on both desktop and WASM
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        // Basic test to verify testing works on both platforms
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_config_creation() {
        // Test that we can create basic config structures on both platforms
        let _config = crate::config::MqttConfig::default();
        assert!(true);
    }

    #[test]
    fn test_profile_creation() {
        // Test that profile creation works on both platforms
        let profile = crate::profile::PlayerProfile::new("test_player".to_string());
        assert_eq!(profile.player_name, "test_player");
        assert!(profile.player_id.starts_with("player-"));
    }
}
