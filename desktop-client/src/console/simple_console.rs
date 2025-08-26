use bevy::prelude::*;
use std::collections::VecDeque;

use crate::console::command_parser::CommandParser;
use crate::console::console_trait::{Console, ConsoleRenderData, ConsoleResult};

/// Simple console implementation using basic logging
/// Works reliably on both desktop and WASM
pub struct SimpleConsole {
    command_parser: CommandParser,
    output_lines: VecDeque<String>,
    max_output_lines: usize,
    visible: bool,
    input_text: String,
    command_history: Vec<String>,
    history_index: Option<usize>,
}

impl Default for SimpleConsole {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleConsole {
    pub fn new() -> Self {
        Self {
            command_parser: CommandParser::new(),
            output_lines: VecDeque::new(),
            max_output_lines: 50,
            visible: false,
            input_text: String::new(),
            command_history: Vec::new(),
            history_index: None,
        }
    }

    fn add_output_line(&mut self, line: String) {
        // Split multi-line output
        for single_line in line.split('\n') {
            self.output_lines.push_back(single_line.to_string());
            info!("[Console] {}", single_line); // Also log to console

            // Limit output lines to prevent memory issues
            while self.output_lines.len() > self.max_output_lines {
                self.output_lines.pop_front();
            }
        }
    }

    pub fn execute_command(&mut self, command: &str, world: &mut World) {
        if command.trim().is_empty() {
            return;
        }

        // Add to history
        if self.command_history.last() != Some(&command.to_string()) {
            self.command_history.push(command.to_string());
            if self.command_history.len() > 50 {
                self.command_history.remove(0);
            }
        }
        self.history_index = None;

        // Add command to output
        self.add_output_line(format!("> {}", command));

        // Execute the command
        let result = self.command_parser.parse_command(command, world);

        // Add result to output
        match result {
            ConsoleResult::Success(msg) => {
                if msg == "CLEAR_OUTPUT" {
                    self.output_lines.clear();
                } else if !msg.is_empty() {
                    self.add_output_line(msg);
                }
            }
            ConsoleResult::Error(msg) => {
                self.add_output_line(format!("ERROR: {}", msg));
            }
            ConsoleResult::CommandNotFound(msg) => {
                self.add_output_line(format!("Unknown command: {}", msg));
            }
            ConsoleResult::InvalidArgs(msg) => {
                self.add_output_line(format!("Invalid arguments: {}", msg));
            }
        }
    }
}

impl Console for SimpleConsole {
    fn initialize(&mut self, app: &mut App) {
        info!("Initializing Simple Console - commands will be logged to terminal");

        // Add console systems
        app.add_systems(Update, handle_simple_console_input)
            .insert_resource(SimpleConsoleState {
                pending_command: None,
                visible: false,
            });

        self.add_output_line(
            "IoTCraft Console initialized. Type 'help' for available commands.".to_string(),
        );
        self.add_output_line(
            "Note: Console output appears in the terminal. Press F12 to toggle visibility."
                .to_string(),
        );
    }

    fn process_input(&mut self, input: &str) -> ConsoleResult {
        // This will be handled in the update method with full world access
        ConsoleResult::Success("Command queued".to_string())
    }

    fn add_output(&mut self, message: &str) {
        self.add_output_line(message.to_string());
    }

    fn clear_output(&mut self) {
        self.output_lines.clear();
    }

    fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            info!("Console opened - commands are processed and output to terminal");
            info!(
                "Available commands: help, clear, blink, mqtt, spawn, place, remove, wall, save, load, give, tp, look, move, list, test_error"
            );
            info!("Example: Type 'help' to see detailed command help");
        } else {
            info!("Console closed");
        }
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn update(&mut self, world: &mut World) {
        // Check for pending commands
        if let Some(mut console_state) = world.get_resource_mut::<SimpleConsoleState>() {
            console_state.visible = self.visible;

            if let Some(command) = console_state.pending_command.take() {
                self.execute_command(&command, world);
            }
        }
    }

    fn get_render_data(&self) -> Option<ConsoleRenderData> {
        Some(ConsoleRenderData {
            visible: self.visible,
            output_lines: self.output_lines.iter().cloned().collect(),
            input_text: self.input_text.clone(),
            cursor_position: 0,
        })
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Resource for simple console state
#[derive(Resource)]
pub struct SimpleConsoleState {
    pub pending_command: Option<String>,
    pub visible: bool,
}

/// System to handle simple console input via keyboard shortcuts
fn handle_simple_console_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut console_state: ResMut<SimpleConsoleState>,
) {
    if !console_state.visible {
        return;
    }

    // Check for common command shortcuts when console is visible
    if keyboard_input.just_pressed(KeyCode::KeyH) && keyboard_input.pressed(KeyCode::ControlLeft) {
        console_state.pending_command = Some("help".to_string());
    } else if keyboard_input.just_pressed(KeyCode::KeyC)
        && keyboard_input.pressed(KeyCode::ControlLeft)
    {
        console_state.pending_command = Some("clear".to_string());
    }

    // Add more shortcuts as needed
}

/// Convenience function for external systems to execute console commands
pub fn execute_console_command(command: &str, world: &mut World) {
    if let Some(mut console_state) = world.get_resource_mut::<SimpleConsoleState>() {
        console_state.pending_command = Some(command.to_string());
    }
}
