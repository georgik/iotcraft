use bevy::prelude::*;
use std::any::Any;

/// Trait that defines the interface for console implementations
/// This allows us to swap between different console backends (bevy_console, custom UI, etc.)
pub trait Console: Send + Sync {
    /// Initialize the console (called during startup)
    fn initialize(&mut self, app: &mut App);

    /// Process console input and execute commands
    fn process_input(&mut self, input: &str) -> ConsoleResult;

    /// Add a message to the console output
    fn add_output(&mut self, message: &str);

    /// Clear the console output
    fn clear_output(&mut self);

    /// Toggle console visibility
    fn toggle_visibility(&mut self);

    /// Check if console is currently visible
    fn is_visible(&self) -> bool;

    /// Update the console (called each frame)
    fn update(&mut self, world: &mut World);

    /// Get the console's render data (for integration with rendering pipeline)
    fn get_render_data(&self) -> Option<ConsoleRenderData>;

    /// Support for downcasting to concrete types
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn as_any(&self) -> &dyn Any;
}

/// Result type for console operations
#[derive(Debug, Clone)]
pub enum ConsoleResult {
    Success(String),
    Error(String),
    CommandNotFound(String),
    InvalidArgs(String),
}

/// Data needed for rendering the console
#[derive(Debug, Clone)]
pub struct ConsoleRenderData {
    pub visible: bool,
    pub output_lines: Vec<String>,
    pub input_text: String,
    pub cursor_position: usize,
}

/// Resource that holds the current console implementation
#[derive(Resource)]
pub struct ConsoleManager {
    pub console: Box<dyn Console>,
    pub enabled: bool,
}

impl ConsoleManager {
    pub fn new(console: Box<dyn Console>) -> Self {
        Self {
            console,
            enabled: true,
        }
    }

    pub fn process_command(&mut self, command: &str) -> ConsoleResult {
        if !self.enabled {
            return ConsoleResult::Error("Console is disabled".to_string());
        }
        self.console.process_input(command)
    }

    pub fn add_message(&mut self, message: &str) {
        if self.enabled {
            self.console.add_output(message);
        }
    }

    pub fn toggle(&mut self) {
        if self.enabled {
            self.console.toggle_visibility();
        }
    }
}

/// Console configuration resource
#[derive(Resource, Clone)]
pub struct ConsoleConfig {
    pub toggle_key: KeyCode,
    pub max_output_lines: usize,
    pub font_size: f32,
    pub background_color: Color,
    pub text_color: Color,
    pub input_prompt: String,
}

impl Default for ConsoleConfig {
    fn default() -> Self {
        Self {
            toggle_key: KeyCode::F12,
            max_output_lines: 100,
            font_size: 14.0,
            background_color: Color::srgba(0.0, 0.0, 0.0, 0.8),
            text_color: Color::srgb(1.0, 1.0, 1.0),
            input_prompt: "> ".to_string(),
        }
    }
}
