// Core console architecture
pub mod command_parser;
pub mod console_plugin;
pub mod console_trait;

// Console implementations
pub mod bevy_ui_console;
pub mod simple_console;

// Console infrastructure (replacement for bevy_console)
pub mod console_infrastructure;

#[cfg(feature = "console-slint")]
pub mod slint_console;

// Legacy console components (kept for compatibility during transition)
pub mod console_helpers;
pub mod console_systems;
pub mod console_types;
pub mod esc_handling;

use bevy::prelude::*;

// Re-export new console architecture
pub use command_parser::*;
pub use console_infrastructure::*;
pub use console_plugin::*;
pub use console_trait::*;

// BlinkState resource for console blink functionality
#[cfg(feature = "console")]
#[derive(Resource, Default)]
pub struct BlinkState {
    pub blinking: bool,
    pub light_state: bool,
    pub last_sent: bool,
}

#[cfg(feature = "console")]
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
#[cfg(feature = "console")]
#[derive(Component)]
pub struct BlinkCube;

#[cfg(feature = "console")]
pub use console_types::*;
