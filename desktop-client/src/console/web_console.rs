use bevy::prelude::*;

/// Web console plugin for WASM builds
/// This is a simplified version for web compatibility
pub struct WebConsolePlugin;

/// Web-specific BlinkState that doesn't require console feature
#[derive(Resource, Default)]
pub struct BlinkState {
    pub blinking: bool,
    pub light_state: bool,
    pub last_sent: bool,
}

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

/// BlinkCube component marker for devices that support blinking
#[derive(Component)]
pub struct BlinkCube;

impl Plugin for WebConsolePlugin {
    fn build(&self, app: &mut App) {
        // Add the BlinkState resource for web
        app.insert_resource(BlinkState::default());

        info!("WebConsolePlugin initialized (simplified for WASM)");

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&"📝 WebConsolePlugin initialized (simplified for WASM)".into());
    }
}
