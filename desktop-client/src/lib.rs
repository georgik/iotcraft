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

// Note: Multiplayer and full device functionality requires desktop-only dependencies
// For now, MQTT device announcements are handled in the web MQTT plugin

// Desktop-only modules for tests and library usage
#[cfg(not(target_arch = "wasm32"))]
pub mod multiplayer;

#[cfg(not(target_arch = "wasm32"))]
pub mod config;

#[cfg(not(target_arch = "wasm32"))]
pub mod profile;

#[cfg(not(target_arch = "wasm32"))]
pub mod environment;

#[cfg(not(target_arch = "wasm32"))]
pub mod world;

#[cfg(not(target_arch = "wasm32"))]
pub mod inventory;

#[cfg(not(target_arch = "wasm32"))]
pub mod camera_controllers;

// Console module available for both desktop and web
pub mod console;

#[cfg(not(target_arch = "wasm32"))]
pub mod devices;

#[cfg(not(target_arch = "wasm32"))]
pub mod interaction;

#[cfg(not(target_arch = "wasm32"))]
pub mod fonts;

#[cfg(not(target_arch = "wasm32"))]
pub mod localization;

#[cfg(not(target_arch = "wasm32"))]
pub mod script;

#[cfg(not(target_arch = "wasm32"))]
pub mod ui;

#[cfg(not(target_arch = "wasm32"))]
pub mod shared_materials;

#[cfg(not(target_arch = "wasm32"))]
pub mod rendering;

#[cfg(not(target_arch = "wasm32"))]
pub mod mcp;

#[cfg(not(target_arch = "wasm32"))]
pub mod mqtt;

#[cfg(not(target_arch = "wasm32"))]
pub mod reply {}

#[cfg(not(target_arch = "wasm32"))]
pub mod player_controller;

#[cfg(not(target_arch = "wasm32"))]
pub mod player_avatar;

#[cfg(target_arch = "wasm32")]
pub use lib_gradual::*;

#[cfg(target_arch = "wasm32")]
pub use lib_debug::{debug_set_panic_hook, debug_start};
