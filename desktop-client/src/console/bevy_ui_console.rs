use bevy::prelude::*;
use std::collections::VecDeque;

use crate::console::command_parser::CommandParser;
use crate::console::console_trait::{Console, ConsoleRenderData, ConsoleResult};

/// Simple console implementation using Bevy's built-in UI system
/// Works on both desktop and WASM
pub struct BevyUiConsole {
    command_parser: CommandParser,
    output_lines: VecDeque<String>,
    max_output_lines: usize,
    visible: bool,
    input_text: String,
    cursor_position: usize,
    command_history: Vec<String>,
    history_index: Option<usize>,
}

impl Default for BevyUiConsole {
    fn default() -> Self {
        Self::new()
    }
}

impl BevyUiConsole {
    pub fn new() -> Self {
        Self {
            command_parser: CommandParser::new(),
            output_lines: VecDeque::new(),
            max_output_lines: 50,
            visible: false,
            input_text: String::new(),
            cursor_position: 0,
            command_history: Vec::new(),
            history_index: None,
        }
    }

    fn add_output_line(&mut self, line: String) {
        // Split multi-line output
        for single_line in line.split('\n') {
            self.output_lines.push_back(single_line.to_string());

            // Limit output lines to prevent memory issues
            while self.output_lines.len() > self.max_output_lines {
                self.output_lines.pop_front();
            }
        }
    }

    fn execute_command(&mut self, command: &str, world: &mut World) {
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

    fn handle_input(&mut self, character: char) {
        match character {
            '\x08' => {
                // Backspace
                if self.cursor_position > 0 {
                    self.input_text.remove(self.cursor_position - 1);
                    self.cursor_position -= 1;
                }
            }
            '\x7f' => {
                // Delete
                if self.cursor_position < self.input_text.len() {
                    self.input_text.remove(self.cursor_position);
                }
            }
            c if c.is_control() => {
                // Ignore other control characters
            }
            c => {
                // Insert character at cursor position
                self.input_text.insert(self.cursor_position, c);
                self.cursor_position += 1;
            }
        }
    }

    fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::ArrowLeft => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
            }
            KeyCode::ArrowRight => {
                if self.cursor_position < self.input_text.len() {
                    self.cursor_position += 1;
                }
            }
            KeyCode::ArrowUp => {
                // Navigate command history up
                if !self.command_history.is_empty() {
                    let new_index = match self.history_index {
                        None => Some(self.command_history.len() - 1),
                        Some(i) if i > 0 => Some(i - 1),
                        Some(_) => Some(0),
                    };

                    if let Some(index) = new_index {
                        self.history_index = new_index;
                        self.input_text = self.command_history[index].clone();
                        self.cursor_position = self.input_text.len();
                    }
                }
            }
            KeyCode::ArrowDown => {
                // Navigate command history down
                match self.history_index {
                    Some(i) if i < self.command_history.len() - 1 => {
                        self.history_index = Some(i + 1);
                        self.input_text = self.command_history[i + 1].clone();
                        self.cursor_position = self.input_text.len();
                    }
                    Some(_) => {
                        self.history_index = None;
                        self.input_text.clear();
                        self.cursor_position = 0;
                    }
                    None => {}
                }
            }
            KeyCode::Home => {
                self.cursor_position = 0;
            }
            KeyCode::End => {
                self.cursor_position = self.input_text.len();
            }
            KeyCode::Backspace => {
                self.handle_input('\x08');
            }
            KeyCode::Delete => {
                self.handle_input('\x7f');
            }
            _ => {}
        }
    }
}

impl Console for BevyUiConsole {
    fn initialize(&mut self, app: &mut App) {
        info!("Initializing Bevy UI Console");

        // Add console UI components and systems
        app.add_systems(Startup, setup_console_ui).add_systems(
            Update,
            (update_console_ui, handle_console_input).run_if(resource_exists::<ConsoleUiState>),
        );

        // Initialize UI state resource
        app.insert_resource(ConsoleUiState {
            visible: false,
            output_text: "IoTCraft Console initialized. Type 'help' for available commands.\n"
                .to_string(),
            input_text: String::new(),
            cursor_visible: true,
            cursor_timer: Timer::from_seconds(0.5, TimerMode::Repeating),
        });

        self.add_output_line(
            "IoTCraft Console initialized. Type 'help' for available commands.".to_string(),
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
        info!("Console {}", if self.visible { "opened" } else { "closed" });
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn update(&mut self, world: &mut World) {
        // Update UI state resource
        if let Some(mut ui_state) = world.get_resource_mut::<ConsoleUiState>() {
            ui_state.visible = self.visible;

            // Update output text
            ui_state.output_text = self
                .output_lines
                .iter()
                .map(|line| line.as_str())
                .collect::<Vec<_>>()
                .join("\n");

            // Update input text
            ui_state.input_text = self.input_text.clone();
        }
    }

    fn get_render_data(&self) -> Option<ConsoleRenderData> {
        Some(ConsoleRenderData {
            visible: self.visible,
            output_lines: self.output_lines.iter().cloned().collect(),
            input_text: self.input_text.clone(),
            cursor_position: self.cursor_position,
        })
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Resource for managing console UI state
#[derive(Resource)]
pub struct ConsoleUiState {
    pub visible: bool,
    pub output_text: String,
    pub input_text: String,
    pub cursor_visible: bool,
    pub cursor_timer: Timer,
}

/// Console UI components
#[derive(Component)]
pub struct ConsoleRoot;

#[derive(Component)]
pub struct ConsoleOutput;

#[derive(Component)]
pub struct ConsoleInput;

/// System to set up the console UI
fn setup_console_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(50.0),
                left: Val::Px(50.0),
                width: Val::Px(800.0),
                height: Val::Px(400.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.9)),
            BorderColor::all(Color::srgb(0.5, 0.5, 0.5)),
            BorderRadius::all(Val::Px(8.0)),
            Visibility::Hidden, // Initially hidden
            ConsoleRoot,
        ))
        .with_children(|parent| {
            // Output area
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(85.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        overflow: Overflow::clip_y(),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 1.0, 1.0)),
                        ConsoleOutput,
                    ));
                });

            // Input area
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(15.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8)),
                ))
                .with_children(|parent| {
                    // Prompt
                    parent.spawn((
                        Text::new("> "),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.0, 1.0, 0.0)),
                    ));

                    // Input text
                    parent.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 1.0, 1.0)),
                        ConsoleInput,
                    ));
                });
        });
}

/// System to update console UI based on console state
fn update_console_ui(
    ui_state: Res<ConsoleUiState>,
    mut console_query: Query<&mut Visibility, With<ConsoleRoot>>,
    mut output_query: Query<&mut Text, (With<ConsoleOutput>, Without<ConsoleInput>)>,
    mut input_query: Query<&mut Text, (With<ConsoleInput>, Without<ConsoleOutput>)>,
    time: Res<Time>,
) {
    // Update visibility
    if let Ok(mut visibility) = console_query.single_mut() {
        *visibility = if ui_state.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // Update output text
    if let Ok(mut text) = output_query.single_mut() {
        **text = ui_state.output_text.clone();
    }

    // Update input text with cursor
    if let Ok(mut text) = input_query.single_mut() {
        let mut input_display = ui_state.input_text.clone();

        // Add blinking cursor
        if ui_state.visible {
            let cursor_char = if (time.elapsed_secs() * 2.0) as u32 % 2 == 0 {
                "_"
            } else {
                " "
            };
            input_display.push_str(cursor_char);
        }

        **text = input_display;
    }
}

/// System to handle console input
fn handle_console_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut console_manager: ResMut<crate::console::ConsoleManager>,
    ui_state: ResMut<ConsoleUiState>,
) {
    if !console_manager.console.is_visible() {
        return;
    }

    // Handle Enter key for command execution
    if keyboard_input.just_pressed(KeyCode::Enter) {
        if let Some(bevy_ui_console) = console_manager
            .console
            .as_any_mut()
            .downcast_mut::<BevyUiConsole>()
        {
            let command = bevy_ui_console.input_text.clone();
            // TODO: Queue command for execution in update method that has world access
            bevy_ui_console.add_output_line(format!("Command queued: {}", command));
            bevy_ui_console.input_text.clear();
            bevy_ui_console.cursor_position = 0;
        }
    }

    // Handle special keys
    for key in keyboard_input.get_just_pressed() {
        if let Some(bevy_ui_console) = console_manager
            .console
            .as_any_mut()
            .downcast_mut::<BevyUiConsole>()
        {
            bevy_ui_console.handle_key(*key);
        }
    }
}
