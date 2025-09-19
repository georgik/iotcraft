//! Shared console architecture for both desktop and WASM
//! This module provides common console functionality that can be used across platforms

use bevy::prelude::*;
use std::collections::VecDeque;

// ConsoleResult is defined in console_trait.rs to avoid duplication
use crate::console::console_trait::ConsoleResult;

/// Shared console trait for both desktop and WASM implementations
#[allow(dead_code)]
pub trait SharedConsole: Send + Sync {
    /// Execute a command and return the result
    fn execute_command(&mut self, command: &str, world: &mut World) -> ConsoleResult;

    /// Add a message to the console output
    fn add_output(&mut self, message: &str);

    /// Check if console is currently visible
    fn is_visible(&self) -> bool;

    /// Toggle console visibility
    fn toggle_visibility(&mut self);

    /// Clear the console output
    fn clear_output(&mut self);

    /// Get command history
    fn get_history(&self) -> &[String];

    /// Update console state (called each frame)
    fn update(&mut self, world: &mut World);
}

/// Shared console state that can be used by both desktop and WASM
#[allow(dead_code)]
#[derive(Resource)]
pub struct SharedConsoleState {
    pub visible: bool,
    pub output_lines: VecDeque<String>,
    pub command_history: Vec<String>,
    pub max_output_lines: usize,
    pub max_history_lines: usize,
}

impl Default for SharedConsoleState {
    fn default() -> Self {
        Self {
            visible: false,
            output_lines: VecDeque::new(),
            command_history: Vec::new(),
            max_output_lines: 100,
            max_history_lines: 50,
        }
    }
}

impl SharedConsoleState {
    #[allow(dead_code)]
    pub fn add_output_line(&mut self, line: String) {
        // Split multi-line output
        for single_line in line.split('\n') {
            self.output_lines.push_back(single_line.to_string());

            // Limit output lines to prevent memory issues
            while self.output_lines.len() > self.max_output_lines {
                self.output_lines.pop_front();
            }
        }
    }

    #[allow(dead_code)]
    pub fn add_to_history(&mut self, command: String) {
        if self.command_history.last() != Some(&command) {
            self.command_history.push(command);
            if self.command_history.len() > self.max_history_lines {
                self.command_history.remove(0);
            }
        }
    }

    #[allow(dead_code)]
    pub fn clear_output(&mut self) {
        self.output_lines.clear();
    }

    #[allow(dead_code)]
    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }
}

/// Console configuration resource
#[allow(dead_code)]
#[derive(Resource, Clone)]
pub struct SharedConsoleConfig {
    pub toggle_key: KeyCode,
    pub enabled: bool,
}

impl Default for SharedConsoleConfig {
    fn default() -> Self {
        Self {
            toggle_key: KeyCode::F12,
            enabled: true,
        }
    }
}

/// Event for sending messages to the console from other systems
#[allow(dead_code)]
#[derive(Debug, Clone, bevy::prelude::Event, bevy::prelude::BufferedEvent)]
pub struct ConsoleMessageEvent {
    pub message: String,
}

/// Basic shared console implementation using the command parser
use crate::console::command_parser::CommandParser;

#[allow(dead_code)]
pub struct BasicSharedConsole {
    command_parser: CommandParser,
    state: SharedConsoleState,
}

impl BasicSharedConsole {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            command_parser: CommandParser::new(),
            state: SharedConsoleState::default(),
        }
    }

    /// Get access to the output lines for rendering
    #[allow(dead_code)]
    pub fn get_output_lines(&self) -> &VecDeque<String> {
        &self.state.output_lines
    }

    /// Get mutable access to the state (for web console integration)
    #[allow(dead_code)]
    pub fn get_state_mut(&mut self) -> &mut SharedConsoleState {
        &mut self.state
    }
}

impl SharedConsole for BasicSharedConsole {
    fn execute_command(&mut self, command: &str, world: &mut World) -> ConsoleResult {
        if command.trim().is_empty() {
            return ConsoleResult::Success(String::new());
        }

        // Add to history
        self.state.add_to_history(command.to_string());

        // Add command to output
        self.state.add_output_line(format!("> {}", command));

        // Execute the command using the shared command parser
        let result = self.command_parser.parse_command(command, world);

        // Add result to output
        match &result {
            ConsoleResult::Success(msg) => {
                if msg == "CLEAR_OUTPUT" {
                    self.state.clear_output();
                } else if !msg.is_empty() {
                    self.state.add_output_line(msg.clone());
                }
            }
            ConsoleResult::Error(msg) => {
                self.state.add_output_line(format!("ERROR: {}", msg));
            }
            ConsoleResult::CommandNotFound(msg) => {
                self.state
                    .add_output_line(format!("Unknown command: {}", msg));
            }
            ConsoleResult::InvalidArgs(msg) => {
                self.state
                    .add_output_line(format!("Invalid arguments: {}", msg));
            }
        }

        // Return result directly (no conversion needed since we're using the same type)
        result
    }

    fn add_output(&mut self, message: &str) {
        self.state.add_output_line(message.to_string());
    }

    fn is_visible(&self) -> bool {
        self.state.visible
    }

    fn toggle_visibility(&mut self) {
        self.state.toggle_visibility();
    }

    fn clear_output(&mut self) {
        self.state.clear_output();
    }

    fn get_history(&self) -> &[String] {
        &self.state.command_history
    }

    fn update(&mut self, _world: &mut World) {
        // Basic implementation doesn't need frame updates
    }
}
