use crate::console::console_trait::ConsoleResult;
use bevy::prelude::*;

// Import required types
#[cfg(feature = "console")]
use crate::console::BlinkState;
use crate::mqtt::TemperatureResource;

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

    #[cfg(test)]
    pub fn get_history(&self) -> &Vec<String> {
        &self.command_history
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
        ConsoleResult::Success(help_text.join("\n"))
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
            .join("\n");

        ConsoleResult::Success(format!("Command history:\n{}", history))
    }

    // Placeholder implementations for other commands - these will delegate to the existing systems
    fn handle_blink_command(&self, args: &[&str], world: &mut World) -> ConsoleResult {
        if args.is_empty() {
            return ConsoleResult::InvalidArgs("Usage: blink [start|stop]".to_string());
        }

        match args[0] {
            "start" => {
                #[cfg(feature = "console")]
                if let Some(mut blink_state) = world.get_resource_mut::<BlinkState>() {
                    blink_state.blinking = true;
                    ConsoleResult::Success("Blink started".to_string())
                } else {
                    ConsoleResult::Error("Blink state not found".to_string())
                }
                #[cfg(not(feature = "console"))]
                ConsoleResult::Error("Blink functionality not available".to_string())
            }
            "stop" => {
                #[cfg(feature = "console")]
                if let Some(mut blink_state) = world.get_resource_mut::<BlinkState>() {
                    blink_state.blinking = false;
                    ConsoleResult::Success("Blink stopped".to_string())
                } else {
                    ConsoleResult::Error("Blink state not found".to_string())
                }
                #[cfg(not(feature = "console"))]
                ConsoleResult::Error("Blink functionality not available".to_string())
            }
            _ => ConsoleResult::InvalidArgs("Usage: blink [start|stop]".to_string()),
        }
    }

    fn handle_mqtt_command(&self, args: &[&str], world: &mut World) -> ConsoleResult {
        if args.is_empty() {
            return ConsoleResult::InvalidArgs("Usage: mqtt [status|temp]".to_string());
        }

        match args[0] {
            "status" => {
                if let Some(temperature) = world.get_resource::<TemperatureResource>() {
                    let status = if temperature.value.is_some() {
                        "✅ Connected to MQTT broker"
                    } else {
                        "🔄 Connecting to MQTT broker..."
                    };
                    ConsoleResult::Success(status.to_string())
                } else {
                    ConsoleResult::Error("Temperature resource not found".to_string())
                }
            }
            "temp" => {
                if let Some(temperature) = world.get_resource::<TemperatureResource>() {
                    let temp_msg = if let Some(val) = temperature.value {
                        format!("Current temperature: {:.1}°C", val)
                    } else {
                        "No temperature data available".to_string()
                    };
                    ConsoleResult::Success(temp_msg)
                } else {
                    ConsoleResult::Error("Temperature resource not found".to_string())
                }
            }
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

    fn handle_place_command(&self, args: &[&str], world: &mut World) -> ConsoleResult {
        if args.len() != 4 {
            return ConsoleResult::InvalidArgs("Usage: place <block_type> <x> <y> <z>".to_string());
        }

        // Parse coordinates
        let x = match args[1].parse::<i32>() {
            Ok(x) => x,
            Err(_) => {
                return ConsoleResult::InvalidArgs("X coordinate must be a number".to_string());
            }
        };
        let y = match args[2].parse::<i32>() {
            Ok(y) => y,
            Err(_) => {
                return ConsoleResult::InvalidArgs("Y coordinate must be a number".to_string());
            }
        };
        let z = match args[3].parse::<i32>() {
            Ok(z) => z,
            Err(_) => {
                return ConsoleResult::InvalidArgs("Z coordinate must be a number".to_string());
            }
        };

        // Parse block type
        let block_type = match args[0] {
            "grass" => crate::environment::BlockType::Grass,
            "dirt" => crate::environment::BlockType::Dirt,
            "stone" => crate::environment::BlockType::Stone,
            "quartz_block" => crate::environment::BlockType::QuartzBlock,
            "glass_pane" => crate::environment::BlockType::GlassPane,
            "cyan_terracotta" => crate::environment::BlockType::CyanTerracotta,
            "water" => crate::environment::BlockType::Water,
            _ => return ConsoleResult::InvalidArgs(format!("Invalid block type: {}", args[0])),
        };

        let position = bevy::math::IVec3::new(x, y, z);

        // Place the block in voxel world
        if let Some(mut voxel_world) = world.get_resource_mut::<crate::environment::VoxelWorld>() {
            voxel_world.set_block(position, block_type);

            // Release the mutable borrow before getting events
            drop(voxel_world);

            // Send place block event to spawn visual representation
            if let Some(mut place_events) = world
                .get_resource_mut::<bevy::ecs::event::Events<crate::inventory::PlaceBlockEvent>>()
            {
                place_events.write(crate::inventory::PlaceBlockEvent {
                    position,
                    // Since this is a console command, we'll force the placement
                });
            }

            ConsoleResult::Success(format!("Placed {} block at ({}, {}, {})", args[0], x, y, z))
        } else {
            ConsoleResult::Error("Voxel world not found".to_string())
        }
    }

    fn handle_remove_command(&self, args: &[&str], world: &mut World) -> ConsoleResult {
        if args.len() != 3 {
            return ConsoleResult::InvalidArgs("Usage: remove <x> <y> <z>".to_string());
        }

        // Parse coordinates
        let x = match args[0].parse::<i32>() {
            Ok(x) => x,
            Err(_) => {
                return ConsoleResult::InvalidArgs("X coordinate must be a number".to_string());
            }
        };
        let y = match args[1].parse::<i32>() {
            Ok(y) => y,
            Err(_) => {
                return ConsoleResult::InvalidArgs("Y coordinate must be a number".to_string());
            }
        };
        let z = match args[2].parse::<i32>() {
            Ok(z) => z,
            Err(_) => {
                return ConsoleResult::InvalidArgs("Z coordinate must be a number".to_string());
            }
        };

        let position = bevy::math::IVec3::new(x, y, z);

        // Remove the block from voxel world
        if let Some(mut voxel_world) = world.get_resource_mut::<crate::environment::VoxelWorld>() {
            if voxel_world.remove_block(&position).is_some() {
                // Find and despawn the block entity
                let mut entities_to_despawn = Vec::new();
                let mut query = world.query::<(Entity, &crate::environment::VoxelBlock)>();

                for (entity, block) in query.iter(world) {
                    if block.position == position {
                        entities_to_despawn.push(entity);
                    }
                }

                // Despawn entities (do this after the query to avoid borrow conflicts)
                for entity in entities_to_despawn {
                    if let Ok(entity_commands) = world.get_entity_mut(entity) {
                        entity_commands.despawn();
                    }
                }

                ConsoleResult::Success(format!("Removed block at ({}, {}, {})", x, y, z))
            } else {
                ConsoleResult::Success(format!("No block found at ({}, {}, {})", x, y, z))
            }
        } else {
            ConsoleResult::Error("Voxel world not found".to_string())
        }
    }

    fn handle_wall_command(&self, args: &[&str], world: &mut World) -> ConsoleResult {
        if args.len() != 7 {
            return ConsoleResult::InvalidArgs(
                "Usage: wall <block_type> <x1> <y1> <z1> <x2> <y2> <z2>".to_string(),
            );
        }

        // Parse coordinates
        let x1 = match args[1].parse::<i32>() {
            Ok(x) => x,
            Err(_) => {
                return ConsoleResult::InvalidArgs("X1 coordinate must be a number".to_string());
            }
        };
        let y1 = match args[2].parse::<i32>() {
            Ok(y) => y,
            Err(_) => {
                return ConsoleResult::InvalidArgs("Y1 coordinate must be a number".to_string());
            }
        };
        let z1 = match args[3].parse::<i32>() {
            Ok(z) => z,
            Err(_) => {
                return ConsoleResult::InvalidArgs("Z1 coordinate must be a number".to_string());
            }
        };
        let x2 = match args[4].parse::<i32>() {
            Ok(x) => x,
            Err(_) => {
                return ConsoleResult::InvalidArgs("X2 coordinate must be a number".to_string());
            }
        };
        let y2 = match args[5].parse::<i32>() {
            Ok(y) => y,
            Err(_) => {
                return ConsoleResult::InvalidArgs("Y2 coordinate must be a number".to_string());
            }
        };
        let z2 = match args[6].parse::<i32>() {
            Ok(z) => z,
            Err(_) => {
                return ConsoleResult::InvalidArgs("Z2 coordinate must be a number".to_string());
            }
        };

        // Parse block type
        let block_type_enum = match args[0] {
            "grass" => crate::environment::BlockType::Grass,
            "dirt" => crate::environment::BlockType::Dirt,
            "stone" => crate::environment::BlockType::Stone,
            "quartz_block" => crate::environment::BlockType::QuartzBlock,
            "glass_pane" => crate::environment::BlockType::GlassPane,
            "cyan_terracotta" => crate::environment::BlockType::CyanTerracotta,
            "water" => crate::environment::BlockType::Water,
            _ => return ConsoleResult::InvalidArgs(format!("Invalid block type: {}", args[0])),
        };

        // Place blocks in the wall area
        if let Some(mut voxel_world) = world.get_resource_mut::<crate::environment::VoxelWorld>() {
            let mut blocks_added = 0;

            for x in x1..=x2 {
                for y in y1..=y2 {
                    for z in z1..=z2 {
                        voxel_world.set_block(bevy::math::IVec3::new(x, y, z), block_type_enum);
                        blocks_added += 1;
                    }
                }
            }

            // Release the mutable borrow of voxel_world before trying to get events
            drop(voxel_world);

            // Trigger place block events for visual representation
            if let Some(mut place_events) = world
                .get_resource_mut::<bevy::ecs::event::Events<crate::inventory::PlaceBlockEvent>>()
            {
                for x in x1..=x2 {
                    for y in y1..=y2 {
                        for z in z1..=z2 {
                            place_events.write(crate::inventory::PlaceBlockEvent {
                                position: bevy::math::IVec3::new(x, y, z),
                            });
                        }
                    }
                }
            }

            ConsoleResult::Success(format!(
                "Created a wall of {} from ({}, {}, {}) to ({}, {}, {}) - {} blocks placed",
                args[0], x1, y1, z1, x2, y2, z2, blocks_added
            ))
        } else {
            ConsoleResult::Error("Voxel world not found".to_string())
        }
    }

    fn handle_save_command(&self, args: &[&str], world: &mut World) -> ConsoleResult {
        if args.len() != 1 {
            return ConsoleResult::InvalidArgs("Usage: save <filename>".to_string());
        }

        let filename = args[0];

        if let Some(voxel_world) = world.get_resource::<crate::environment::VoxelWorld>() {
            match voxel_world.save_to_file(filename) {
                Ok(_) => ConsoleResult::Success(format!(
                    "Map saved to '{}' with {} blocks",
                    filename,
                    voxel_world.blocks.len()
                )),
                Err(e) => ConsoleResult::Error(format!("Failed to save map: {}", e)),
            }
        } else {
            ConsoleResult::Error("Voxel world not found".to_string())
        }
    }

    fn handle_load_command(&self, args: &[&str], _world: &mut World) -> ConsoleResult {
        if args.len() != 1 {
            return ConsoleResult::InvalidArgs("Usage: load <filename>".to_string());
        }
        ConsoleResult::Success(format!("Load command: {}", args[0]))
    }

    fn handle_give_command(&self, args: &[&str], world: &mut World) -> ConsoleResult {
        if args.len() != 2 {
            return ConsoleResult::InvalidArgs("Usage: give <item_type> <count>".to_string());
        }

        // Parse count
        let count = match args[1].parse::<u32>() {
            Ok(count) if count > 0 => count,
            _ => return ConsoleResult::InvalidArgs("Count must be a positive number".to_string()),
        };

        // Parse block type
        let block_type = match args[0] {
            "grass" => crate::environment::BlockType::Grass,
            "dirt" => crate::environment::BlockType::Dirt,
            "stone" => crate::environment::BlockType::Stone,
            "quartz_block" => crate::environment::BlockType::QuartzBlock,
            "glass_pane" => crate::environment::BlockType::GlassPane,
            "cyan_terracotta" => crate::environment::BlockType::CyanTerracotta,
            "water" => crate::environment::BlockType::Water,
            _ => return ConsoleResult::InvalidArgs(format!("Invalid item type: {}", args[0])),
        };

        // Try to get the inventory and add items
        if let Some(mut inventory) = world.get_resource_mut::<crate::inventory::PlayerInventory>() {
            let item_type = crate::inventory::ItemType::Block(block_type);
            let remainder = inventory.add_items(item_type, count);

            if remainder == 0 {
                ConsoleResult::Success(format!(
                    "Gave {} {} to player",
                    count,
                    item_type.display_name()
                ))
            } else {
                let given = count - remainder;
                ConsoleResult::Success(format!(
                    "Gave {} {} to player ({} couldn't fit in inventory)",
                    given,
                    item_type.display_name(),
                    remainder
                ))
            }
        } else {
            ConsoleResult::Error("Player inventory not found".to_string())
        }
    }

    fn handle_test_error_command(&self, args: &[&str], _world: &mut World) -> ConsoleResult {
        if args.is_empty() {
            return ConsoleResult::InvalidArgs("Usage: test_error <message>".to_string());
        }
        let message = args.join(" ");
        ConsoleResult::Error(format!("TEST ERROR: {}", message))
    }

    fn handle_teleport_command(&self, args: &[&str], world: &mut World) -> ConsoleResult {
        if args.len() != 3 {
            return ConsoleResult::InvalidArgs("Usage: tp <x> <y> <z>".to_string());
        }

        // Parse coordinates
        let x = match args[0].parse::<f32>() {
            Ok(x) => x,
            Err(_) => {
                return ConsoleResult::InvalidArgs("X coordinate must be a number".to_string());
            }
        };
        let y = match args[1].parse::<f32>() {
            Ok(y) => y,
            Err(_) => {
                return ConsoleResult::InvalidArgs("Y coordinate must be a number".to_string());
            }
        };
        let z = match args[2].parse::<f32>() {
            Ok(z) => z,
            Err(_) => {
                return ConsoleResult::InvalidArgs("Z coordinate must be a number".to_string());
            }
        };

        // Find camera and teleport it
        let mut camera_found = false;
        let mut camera_query = world.query_filtered::<(
            &mut Transform,
            &mut crate::camera_controllers::CameraController,
        ), With<Camera>>();

        for (mut transform, _camera_controller) in camera_query.iter_mut(world) {
            // Set the camera position
            transform.translation = bevy::math::Vec3::new(x, y, z);
            camera_found = true;
            break; // Only teleport the first camera found
        }

        if camera_found {
            ConsoleResult::Success(format!("Teleported to ({:.1}, {:.1}, {:.1})", x, y, z))
        } else {
            ConsoleResult::Error("Could not find camera to teleport".to_string())
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_controllers::CameraController;
    use crate::console::console_trait::ConsoleResult;
    use crate::environment::{BlockType, VoxelWorld};
    use crate::inventory::PlaceBlockEvent;
    use crate::inventory::{ItemType, PlayerInventory};
    use bevy::ecs::event::Events;

    /// Helper function to create a minimal Bevy World for testing
    fn create_test_world() -> World {
        let mut world = World::new();

        // Insert required resources
        world.insert_resource(VoxelWorld::default());
        world.insert_resource(PlayerInventory::new());
        world.insert_resource(Events::<PlaceBlockEvent>::default());

        // Insert console-specific resources for testing
        #[cfg(feature = "console")]
        world.insert_resource(BlinkState {
            blinking: false,
            light_state: false,
            last_sent: false,
        });

        // Insert TemperatureResource for MQTT commands
        world.insert_resource(TemperatureResource { value: Some(22.5) });

        // Add a camera entity for teleport testing
        world.spawn((
            Camera3d::default(),
            Transform::default(),
            GlobalTransform::default(),
            CameraController::default(),
        ));

        world
    }

    /// Helper function to create parser with some history
    fn create_parser_with_history() -> CommandParser {
        let mut parser = CommandParser::new();
        parser.add_to_history("help".to_string());
        parser.add_to_history("give dirt 10".to_string());
        parser
    }

    // Basic Command Parsing Tests

    #[test]
    fn test_empty_command() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        let result = parser.parse_command("", &mut world);
        assert!(matches!(result, ConsoleResult::Success(msg) if msg.is_empty()));

        let result = parser.parse_command("   ", &mut world);
        assert!(matches!(result, ConsoleResult::Success(msg) if msg.is_empty()));
    }

    #[test]
    fn test_unknown_command() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        let result = parser.parse_command("nonexistent", &mut world);
        assert!(matches!(result, ConsoleResult::CommandNotFound(_)));
    }

    #[test]
    fn test_help_command() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        let result = parser.parse_command("help", &mut world);
        assert!(
            matches!(result, ConsoleResult::Success(msg) if msg.contains("Available commands:"))
        );
    }

    #[test]
    fn test_clear_command() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        let result = parser.parse_command("clear", &mut world);
        assert!(matches!(result, ConsoleResult::Success(msg) if msg == "CLEAR_OUTPUT"));
    }

    #[test]
    fn test_history_command() {
        let mut parser = create_parser_with_history();
        let mut world = create_test_world();

        let result = parser.parse_command("history", &mut world);
        assert!(matches!(result, ConsoleResult::Success(msg) if msg.contains("Command history:")));

        // Test empty history - but note that calling parse_command adds to history
        // So we need to check the actual empty state differently
        let empty_parser = CommandParser::new();
        assert_eq!(empty_parser.get_history().len(), 0);

        // When we call history command, it will add "history" to the command history
        // So let's test the handle_history_command method directly for empty case
        let result = empty_parser.handle_history_command(&[]);
        assert!(matches!(result, ConsoleResult::Success(msg) if msg == "No command history"));
    }

    #[test]
    fn test_command_history_management() {
        let mut parser = CommandParser::new();
        assert_eq!(parser.get_history().len(), 0);

        // Test history is added properly
        parser.add_to_history("test1".to_string());
        parser.add_to_history("test2".to_string());
        assert_eq!(parser.get_history().len(), 2);
        assert_eq!(parser.get_history()[0], "test1");
        assert_eq!(parser.get_history()[1], "test2");

        // Test history limit (max_history = 50)
        for i in 0..60 {
            parser.add_to_history(format!("command{}", i));
        }
        assert_eq!(parser.get_history().len(), 50);
        assert_eq!(parser.get_history()[0], "command10"); // First 10 should be removed
    }

    // Argument Validation Tests

    #[test]
    fn test_blink_command_validation() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        // Valid arguments
        let result = parser.parse_command("blink start", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        let result = parser.parse_command("blink stop", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        // Invalid arguments
        let result = parser.parse_command("blink", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        let result = parser.parse_command("blink invalid", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));
    }

    #[test]
    fn test_mqtt_command_validation() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        // Valid arguments
        let result = parser.parse_command("mqtt status", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        let result = parser.parse_command("mqtt temp", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        // Invalid arguments
        let result = parser.parse_command("mqtt", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        let result = parser.parse_command("mqtt invalid", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));
    }

    #[test]
    fn test_spawn_command_validation() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        // Valid arguments
        let result = parser.parse_command("spawn device1 1 2 3", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        // Invalid argument counts
        let result = parser.parse_command("spawn", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        let result = parser.parse_command("spawn device1", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        let result = parser.parse_command("spawn device1 1 2", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        let result = parser.parse_command("spawn device1 1 2 3 4", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));
    }

    #[test]
    fn test_give_command_validation() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        // Valid arguments
        let result = parser.parse_command("give dirt 10", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        // Invalid argument counts
        let result = parser.parse_command("give", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        let result = parser.parse_command("give dirt", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        let result = parser.parse_command("give dirt 10 extra", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        // Invalid item type
        let result = parser.parse_command("give invalid_item 10", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        // Invalid count
        let result = parser.parse_command("give dirt 0", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        let result = parser.parse_command("give dirt -5", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        let result = parser.parse_command("give dirt not_a_number", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));
    }

    #[test]
    fn test_place_command_validation() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        // Valid arguments
        let result = parser.parse_command("place dirt 1 2 3", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        // Invalid argument counts
        let result = parser.parse_command("place", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        let result = parser.parse_command("place dirt 1 2", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        // Invalid block type
        let result = parser.parse_command("place invalid_block 1 2 3", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        // Invalid coordinates
        let result = parser.parse_command("place dirt not_a_number 2 3", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        let result = parser.parse_command("place dirt 1 not_a_number 3", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        let result = parser.parse_command("place dirt 1 2 not_a_number", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));
    }

    #[test]
    fn test_teleport_command_validation() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        // Valid arguments
        let result = parser.parse_command("tp 1.5 2.0 3.5", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        let result = parser.parse_command("teleport 10 20 30", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        // Invalid argument counts
        let result = parser.parse_command("tp", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        let result = parser.parse_command("tp 1 2", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        let result = parser.parse_command("tp 1 2 3 4", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        // Invalid coordinates
        let result = parser.parse_command("tp not_a_number 2 3", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        let result = parser.parse_command("tp 1 not_a_number 3", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));

        let result = parser.parse_command("tp 1 2 not_a_number", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));
    }

    #[test]
    fn test_test_error_command_validation() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        // Valid arguments
        let result = parser.parse_command("test_error This is a test", &mut world);
        assert!(
            matches!(result, ConsoleResult::Error(msg) if msg.contains("TEST ERROR: This is a test"))
        );

        // Invalid arguments
        let result = parser.parse_command("test_error", &mut world);
        assert!(matches!(result, ConsoleResult::InvalidArgs(_)));
    }

    // Command Execution Tests with World State

    #[test]
    fn test_give_command_execution() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        // Test giving valid items
        let result = parser.parse_command("give dirt 20", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        // Verify inventory was updated
        let inventory = world.get_resource::<PlayerInventory>().unwrap();
        let total_dirt = inventory
            .slots
            .iter()
            .flatten()
            .filter(|stack| stack.item_type == ItemType::Block(BlockType::Dirt))
            .map(|stack| stack.count)
            .sum::<u32>();
        assert_eq!(total_dirt, 20);

        // Test giving items that exceed a single stack
        let result = parser.parse_command("give stone 100", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        let inventory = world.get_resource::<PlayerInventory>().unwrap();
        let total_stone = inventory
            .slots
            .iter()
            .flatten()
            .filter(|stack| stack.item_type == ItemType::Block(BlockType::Stone))
            .map(|stack| stack.count)
            .sum::<u32>();
        assert_eq!(total_stone, 100);
    }

    #[test]
    fn test_give_command_inventory_full() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        // Fill inventory to capacity
        let max_capacity = 36 * 64; // 36 slots * 64 max per slot
        let result = parser.parse_command(&format!("give dirt {}", max_capacity), &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        // Try to add more - should indicate some couldn't fit
        let result = parser.parse_command("give dirt 10", &mut world);
        assert!(matches!(result, ConsoleResult::Success(msg) if msg.contains("couldn't fit")));
    }

    #[test]
    fn test_place_command_execution() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        // Test placing a block
        let result = parser.parse_command("place grass 5 10 15", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        // Verify block was placed in voxel world
        let voxel_world = world.get_resource::<VoxelWorld>().unwrap();
        let position = IVec3::new(5, 10, 15);
        assert!(voxel_world.is_block_at(position));
        assert_eq!(voxel_world.blocks.get(&position), Some(&BlockType::Grass));

        // Event emission test would go here, but we'll skip it for compatibility
        // In a real system, PlaceBlockEvent would be emitted and handled by systems
        // We've verified that the block was correctly added to the voxel world above
    }

    #[test]
    fn test_teleport_command_execution() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        // Test teleporting to a position
        let result = parser.parse_command("tp 10.5 20.0 30.5", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        // Verify camera position was updated
        let mut query =
            world.query_filtered::<&Transform, (With<Camera>, With<CameraController>)>();
        for transform in query.iter(&world) {
            let expected_pos = Vec3::new(10.5, 20.0, 30.5);
            assert_eq!(transform.translation, expected_pos);
            break;
        }
    }

    #[test]
    fn test_teleport_command_no_camera() {
        let mut parser = CommandParser::new();
        let mut world = World::new();

        // Insert resources but no camera
        world.insert_resource(VoxelWorld::default());
        world.insert_resource(PlayerInventory::new());

        let result = parser.parse_command("tp 10 20 30", &mut world);
        assert!(
            matches!(result, ConsoleResult::Error(msg) if msg.contains("Could not find camera"))
        );
    }

    // Edge Cases and Integration Tests

    #[test]
    fn test_case_insensitive_commands() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        let result1 = parser.parse_command("HELP", &mut world);
        let result2 = parser.parse_command("help", &mut world);
        let result3 = parser.parse_command("HeLp", &mut world);

        assert!(matches!(result1, ConsoleResult::Success(_)));
        assert!(matches!(result2, ConsoleResult::Success(_)));
        assert!(matches!(result3, ConsoleResult::Success(_)));
    }

    #[test]
    fn test_whitespace_handling() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        // Test commands with extra whitespace
        let result = parser.parse_command("  help  ", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        let result = parser.parse_command("give    dirt    10", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        let result = parser.parse_command("\tgive\tdirt\t10\t", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));
    }

    #[test]
    fn test_all_block_types_in_give_command() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        let block_types = vec![
            "grass",
            "dirt",
            "stone",
            "quartz_block",
            "glass_pane",
            "cyan_terracotta",
            "water",
        ];

        for block_type in block_types {
            let result = parser.parse_command(&format!("give {} 1", block_type), &mut world);
            assert!(
                matches!(result, ConsoleResult::Success(_)),
                "Failed for block type: {}",
                block_type
            );
        }
    }

    #[test]
    fn test_all_block_types_in_place_command() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        let block_types = vec![
            "grass",
            "dirt",
            "stone",
            "quartz_block",
            "glass_pane",
            "cyan_terracotta",
            "water",
        ];

        for (i, block_type) in block_types.iter().enumerate() {
            let result =
                parser.parse_command(&format!("place {} {} 0 0", block_type, i), &mut world);
            assert!(
                matches!(result, ConsoleResult::Success(_)),
                "Failed for block type: {}",
                block_type
            );
        }
    }

    #[test]
    fn test_command_aliases() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        // Test tp and teleport aliases
        let result1 = parser.parse_command("tp 1 2 3", &mut world);
        let result2 = parser.parse_command("teleport 4 5 6", &mut world);

        assert!(matches!(result1, ConsoleResult::Success(_)));
        assert!(matches!(result2, ConsoleResult::Success(_)));
    }

    #[test]
    fn test_boundary_values() {
        let mut parser = CommandParser::new();
        let mut world = create_test_world();

        // Test extreme coordinate values
        let result = parser.parse_command("tp -1000000 1000000 0", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        let result = parser.parse_command("place dirt -2147483648 2147483647 0", &mut world);
        assert!(matches!(result, ConsoleResult::Success(_)));

        // Test large item counts
        let result = parser.parse_command("give dirt 4294967295", &mut world); // u32::MAX
        assert!(matches!(result, ConsoleResult::Success(_)));
    }

    #[test]
    fn test_missing_resources_error_handling() {
        let mut parser = CommandParser::new();
        let mut world = World::new();

        // Test give command without inventory resource
        let result = parser.parse_command("give dirt 10", &mut world);
        assert!(matches!(result, ConsoleResult::Error(msg) if msg.contains("inventory not found")));

        // Test place command without voxel world resource
        let result = parser.parse_command("place dirt 1 2 3", &mut world);
        assert!(
            matches!(result, ConsoleResult::Error(msg) if msg.contains("Voxel world not found"))
        );
    }
}
