// IoTCraft Desktop Client - Web Version (Enhanced Gradual Build)
#[cfg(target_arch = "wasm32")]
mod lib_gradual;

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

// Note: Multiplayer and full device functionality requires desktop-only dependencies
// For now, MQTT device announcements are handled in the web MQTT plugin

#[cfg(target_arch = "wasm32")]
pub use lib_gradual::*;
