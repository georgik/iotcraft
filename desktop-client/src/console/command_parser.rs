use crate::console::console_trait::ConsoleResult;
use crate::console::console_types::*;
use bevy::prelude::*;

/// Unified command parser that works with any console implementation
#[derive(Default)]
pub struct CommandParser {
    command_history: Vec<String>,
    max_history: usize,
}

impl CommandParser {
    pub fn new() -> Self {
        Self {
            command_history: Vec::new(),
            max_history: 50,
        }
    }

    /// Parse and execute a command string
    pub fn parse_command(&mut self, input: &str, world: &mut World) -> ConsoleResult {
        let input = input.trim();
        if input.is_empty() {
            return ConsoleResult::Success("".to_string());
        }

        // Add to history
        self.add_to_history(input.to_string());

        // Split command and arguments
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return ConsoleResult::Error("Empty command".to_string());
        }

        let command = parts[0].to_lowercase();
        let args = &parts[1..];

        // Execute the command
        match command.as_str() {
            "help" => self.handle_help_command(args),
            "clear" => self.handle_clear_command(args),
            "history" => self.handle_history_command(args),
            "blink" => self.handle_blink_command(args, world),
            "mqtt" => self.handle_mqtt_command(args, world),
            "spawn" => self.handle_spawn_command(args, world),
            "spawn_door" => self.handle_spawn_door_command(args, world),
            "place" => self.handle_place_command(args, world),
            "remove" => self.handle_remove_command(args, world),
            "wall" => self.handle_wall_command(args, world),
            "save" => self.handle_save_command(args, world),
            "load" => self.handle_load_command(args, world),
            "give" => self.handle_give_command(args, world),
            "test_error" => self.handle_test_error_command(args, world),
            "tp" | "teleport" => self.handle_teleport_command(args, world),
            "look" => self.handle_look_command(args, world),
            "move" => self.handle_move_command(args, world),
            "list" => self.handle_list_command(args, world),
            _ => ConsoleResult::CommandNotFound(format!("Unknown command: {}", command)),
        }
    }

    fn add_to_history(&mut self, command: String) {
        self.command_history.push(command);
        if self.command_history.len() > self.max_history {
            self.command_history.remove(0);
        }
    }

    pub fn get_history(&self) -> &[String] {
        &self.command_history
    }

    // Command handlers
    fn handle_help_command(&self, _args: &[&str]) -> ConsoleResult {
        let help_text = vec![
            "Available commands:",
            "  help - Show this help message",
            "  clear - Clear console output",
            "  history - Show command history",
            "  blink [start|stop] - Control device blinking",
            "  mqtt [status|temp] - MQTT broker status and temperature",
            "  spawn <device_id> <x> <y> <z> - Spawn a lamp device",
            "  spawn_door <device_id> <x> <y> <z> - Spawn a door device",
            "  place <block_type> <x> <y> <z> - Place a block",
            "  remove <x> <y> <z> - Remove a block",
            "  wall <block_type> <x1> <y1> <z1> <x2> <y2> <z2> - Create a wall",
            "  save <filename> - Save the world",
            "  load <filename> - Load a world",
            "  give <item_type> <count> - Give items to inventory",
            "  tp <x> <y> <z> - Teleport to coordinates",
            "  look <yaw> <pitch> - Set camera rotation",
            "  move <device_id> <x> <y> <z> - Move a device",
            "  list - List all connected devices",
            "  test_error <message> - Test error indicator",
        ];
        ConsoleResult::Success(help_text.join("\\n"))
    }

    fn handle_clear_command(&self, _args: &[&str]) -> ConsoleResult {
        ConsoleResult::Success("CLEAR_OUTPUT".to_string()) // Special marker
    }

    fn handle_history_command(&self, _args: &[&str]) -> ConsoleResult {
        if self.command_history.is_empty() {
            return ConsoleResult::Success("No command history".to_string());
        }

        let history = self
            .command_history
            .iter()
            .enumerate()
            .map(|(i, cmd)| format!("  {}: {}", i + 1, cmd))
            .collect::<Vec<_>>()
            .join("\\n");

        ConsoleResult::Success(format!("Command history:\\n{}", history))
    }

    // Placeholder implementations for other commands - these will delegate to the existing systems
    fn handle_blink_command(&self, args: &[&str], _world: &mut World) -> ConsoleResult {
        if args.is_empty() {
            return ConsoleResult::InvalidArgs("Usage: blink [start|stop]".to_string());
        }

        match args[0] {
            "start" | "stop" => ConsoleResult::Success(format!("Blink command: {}", args[0])),
            _ => ConsoleResult::InvalidArgs("Usage: blink [start|stop]".to_string()),
        }
    }

    fn handle_mqtt_command(&self, args: &[&str], _world: &mut World) -> ConsoleResult {
        if args.is_empty() {
            return ConsoleResult::InvalidArgs("Usage: mqtt [status|temp]".to_string());
        }

        match args[0] {
            "status" | "temp" => ConsoleResult::Success(format!("MQTT command: {}", args[0])),
            _ => ConsoleResult::InvalidArgs("Usage: mqtt [status|temp]".to_string()),
        }
    }

    fn handle_spawn_command(&self, args: &[&str], _world: &mut World) -> ConsoleResult {
        if args.len() != 4 {
            return ConsoleResult::InvalidArgs("Usage: spawn <device_id> <x> <y> <z>".to_string());
        }
        ConsoleResult::Success(format!(
            "Spawn command: {} {} {} {}",
            args[0], args[1], args[2], args[3]
        ))
    }

    fn handle_spawn_door_command(&self, args: &[&str], _world: &mut World) -> ConsoleResult {
        if args.len() != 4 {
            return ConsoleResult::InvalidArgs(
                "Usage: spawn_door <device_id> <x> <y> <z>".to_string(),
            );
        }
        ConsoleResult::Success(format!(
            "Spawn door command: {} {} {} {}",
            args[0], args[1], args[2], args[3]
        ))
    }

    fn handle_place_command(&self, args: &[&str], _world: &mut World) -> ConsoleResult {
        if args.len() != 4 {
            return ConsoleResult::InvalidArgs("Usage: place <block_type> <x> <y> <z>".to_string());
        }
        ConsoleResult::Success(format!(
            "Place command: {} {} {} {}",
            args[0], args[1], args[2], args[3]
        ))
    }

    fn handle_remove_command(&self, args: &[&str], _world: &mut World) -> ConsoleResult {
        if args.len() != 3 {
            return ConsoleResult::InvalidArgs("Usage: remove <x> <y> <z>".to_string());
        }
        ConsoleResult::Success(format!(
            "Remove command: {} {} {}",
            args[0], args[1], args[2]
        ))
    }

    fn handle_wall_command(&self, args: &[&str], _world: &mut World) -> ConsoleResult {
        if args.len() != 7 {
            return ConsoleResult::InvalidArgs(
                "Usage: wall <block_type> <x1> <y1> <z1> <x2> <y2> <z2>".to_string(),
            );
        }
        ConsoleResult::Success(format!(
            "Wall command: {} {} {} {} {} {} {}",
            args[0], args[1], args[2], args[3], args[4], args[5], args[6]
        ))
    }

    fn handle_save_command(&self, args: &[&str], _world: &mut World) -> ConsoleResult {
        if args.len() != 1 {
            return ConsoleResult::InvalidArgs("Usage: save <filename>".to_string());
        }
        ConsoleResult::Success(format!("Save command: {}", args[0]))
    }

    fn handle_load_command(&self, args: &[&str], _world: &mut World) -> ConsoleResult {
        if args.len() != 1 {
            return ConsoleResult::InvalidArgs("Usage: load <filename>".to_string());
        }
        ConsoleResult::Success(format!("Load command: {}", args[0]))
    }

    fn handle_give_command(&self, args: &[&str], _world: &mut World) -> ConsoleResult {
        if args.len() != 2 {
            return ConsoleResult::InvalidArgs("Usage: give <item_type> <count>".to_string());
        }
        ConsoleResult::Success(format!("Give command: {} {}", args[0], args[1]))
    }

    fn handle_test_error_command(&self, args: &[&str], _world: &mut World) -> ConsoleResult {
        if args.is_empty() {
            return ConsoleResult::InvalidArgs("Usage: test_error <message>".to_string());
        }
        let message = args.join(" ");
        ConsoleResult::Error(format!("TEST ERROR: {}", message))
    }

    fn handle_teleport_command(&self, args: &[&str], _world: &mut World) -> ConsoleResult {
        if args.len() != 3 {
            return ConsoleResult::InvalidArgs("Usage: tp <x> <y> <z>".to_string());
        }
        ConsoleResult::Success(format!(
            "Teleport command: {} {} {}",
            args[0], args[1], args[2]
        ))
    }

    fn handle_look_command(&self, args: &[&str], _world: &mut World) -> ConsoleResult {
        if args.len() != 2 {
            return ConsoleResult::InvalidArgs("Usage: look <yaw> <pitch>".to_string());
        }
        ConsoleResult::Success(format!("Look command: {} {}", args[0], args[1]))
    }

    fn handle_move_command(&self, args: &[&str], _world: &mut World) -> ConsoleResult {
        if args.len() != 4 {
            return ConsoleResult::InvalidArgs("Usage: move <device_id> <x> <y> <z>".to_string());
        }
        ConsoleResult::Success(format!(
            "Move command: {} {} {} {}",
            args[0], args[1], args[2], args[3]
        ))
    }

    fn handle_list_command(&self, _args: &[&str], _world: &mut World) -> ConsoleResult {
        ConsoleResult::Success("List command executed".to_string())
    }
}
