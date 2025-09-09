// Core console architecture
pub mod command_parser;
pub mod console_plugin;
pub mod console_trait;

// Shared console architecture (works on both desktop and WASM)
pub mod shared;

// Console implementations
pub mod bevy_ui_console;
pub mod simple_console;

// Web-specific console (for WASM builds)
#[cfg(target_arch = "wasm32")]
pub mod web_console;

// Console infrastructure (replacement for bevy_console)
pub mod console_infrastructure;

// Legacy console components (kept for compatibility during transition)
pub mod console_helpers;
pub mod console_systems;
pub mod console_types;
pub mod esc_handling;

// Re-export new console architecture
pub use console_infrastructure::*;
pub use console_plugin::*;
pub use console_trait::*;

// Import required Bevy types for derive macros
use bevy::prelude::{Component, Resource};

// BlinkState resource for console blink functionality
// Use web_console version for WASM, desktop version for non-WASM
#[cfg(all(target_arch = "wasm32", not(feature = "console")))]
pub use web_console::{BlinkCube, BlinkState};

#[cfg(all(target_arch = "wasm32", feature = "console"))]
pub use web_console::{BlinkCube, BlinkState};

#[cfg(all(not(target_arch = "wasm32"), feature = "console"))]
#[derive(Resource, Default)]
pub struct BlinkState {
    pub blinking: bool,
    pub light_state: bool,
    pub last_sent: bool,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "console"))]
impl BlinkState {
    pub fn update_state(&mut self, time: &bevy::prelude::Time) {
        if self.blinking {
            // Toggle every second
            self.light_state = (time.elapsed_secs() as u32 % 2) == 0;
        } else {
            self.light_state = false;
        }
    }
}

// BlinkCube component marker for devices that support blinking
#[cfg(all(not(target_arch = "wasm32"), feature = "console"))]
#[derive(Component)]
pub struct BlinkCube;

// Default BlinkState and BlinkCube when console feature is disabled on desktop
#[cfg(all(not(target_arch = "wasm32"), not(feature = "console")))]
#[derive(Resource, Default)]
pub struct BlinkState {
    pub blinking: bool,
    pub light_state: bool,
    pub last_sent: bool,
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "console")))]
impl BlinkState {
    pub fn update_state(&mut self, time: &bevy::prelude::Time) {
        if self.blinking {
            // Toggle every second
            self.light_state = (time.elapsed_secs() as u32 % 2) == 0;
        } else {
            self.light_state = false;
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(feature = "console")))]
#[derive(Component)]
pub struct BlinkCube;
