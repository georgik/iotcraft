use bevy::prelude::*;

// Web-compatible console commands without clap dependency
// These will be parsed manually from console input

#[derive(Debug, Clone)]
pub struct BlinkCommand {
    pub action: String,
}

#[derive(Debug, Clone)]
pub struct MqttCommand {
    pub action: String,
}

#[derive(Debug, Clone)]
pub struct SpawnCommand {
    pub device_id: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone)]
pub struct SpawnDoorCommand {
    pub device_id: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone)]
pub struct MoveCommand {
    pub device_id: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone)]
pub struct PlaceBlockCommand {
    pub block_type: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone)]
pub struct RemoveBlockCommand {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone)]
pub struct WallCommand {
    pub block_type: String,
    pub x1: i32,
    pub y1: i32,
    pub z1: i32,
    pub x2: i32,
    pub y2: i32,
    pub z2: i32,
}

#[derive(Debug, Clone)]
pub struct SaveMapCommand {
    pub filename: String,
}

#[derive(Debug, Clone)]
pub struct LoadMapCommand {
    pub filename: String,
}

#[derive(Debug, Clone)]
pub struct GiveCommand {
    pub item_type: String,
    pub count: u32,
}

#[derive(Debug, Clone)]
pub struct TeleportCommand {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone)]
pub struct LookCommand {
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone)]
pub struct ListCommand {
    // No parameters - lists all connected devices
}

#[derive(Resource)]
pub struct BlinkState {
    pub blinking: bool,
    pub timer: Timer,
    pub light_state: bool,
    pub last_sent: bool,
}

impl Default for BlinkState {
    fn default() -> Self {
        Self {
            blinking: false,
            light_state: false,
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            last_sent: false,
        }
    }
}

#[derive(Component)]
pub struct BlinkCube;

// Simple command parser for web console
pub fn parse_console_command(input: &str) -> Option<ConsoleCommandType> {
    let parts: Vec<&str> = input.trim().split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    match parts[0] {
        "blink" => {
            if parts.len() >= 2 {
                Some(ConsoleCommandType::Blink(BlinkCommand {
                    action: parts[1].to_string(),
                }))
            } else {
                None
            }
        }
        "mqtt" => {
            if parts.len() >= 2 {
                Some(ConsoleCommandType::Mqtt(MqttCommand {
                    action: parts[1].to_string(),
                }))
            } else {
                None
            }
        }
        "spawn" => {
            if parts.len() >= 5 {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[2].parse::<f32>(),
                    parts[3].parse::<f32>(),
                    parts[4].parse::<f32>(),
                ) {
                    Some(ConsoleCommandType::Spawn(SpawnCommand {
                        device_id: parts[1].to_string(),
                        x,
                        y,
                        z,
                    }))
                } else {
                    None
                }
            } else {
                None
            }
        }
        "place" => {
            if parts.len() >= 5 {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[2].parse::<i32>(),
                    parts[3].parse::<i32>(),
                    parts[4].parse::<i32>(),
                ) {
                    Some(ConsoleCommandType::PlaceBlock(PlaceBlockCommand {
                        block_type: parts[1].to_string(),
                        x,
                        y,
                        z,
                    }))
                } else {
                    None
                }
            } else {
                None
            }
        }
        "remove" => {
            if parts.len() >= 4 {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[1].parse::<i32>(),
                    parts[2].parse::<i32>(),
                    parts[3].parse::<i32>(),
                ) {
                    Some(ConsoleCommandType::RemoveBlock(RemoveBlockCommand { x, y, z }))
                } else {
                    None
                }
            } else {
                None
            }
        }
        "wall" => {
            if parts.len() >= 8 {
                if let (Ok(x1), Ok(y1), Ok(z1), Ok(x2), Ok(y2), Ok(z2)) = (
                    parts[2].parse::<i32>(),
                    parts[3].parse::<i32>(),
                    parts[4].parse::<i32>(),
                    parts[5].parse::<i32>(),
                    parts[6].parse::<i32>(),
                    parts[7].parse::<i32>(),
                ) {
                    Some(ConsoleCommandType::Wall(WallCommand {
                        block_type: parts[1].to_string(),
                        x1,
                        y1,
                        z1,
                        x2,
                        y2,
                        z2,
                    }))
                } else {
                    None
                }
            } else {
                None
            }
        }
        "tp" => {
            if parts.len() >= 4 {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[1].parse::<f32>(),
                    parts[2].parse::<f32>(),
                    parts[3].parse::<f32>(),
                ) {
                    Some(ConsoleCommandType::Teleport(TeleportCommand { x, y, z }))
                } else {
                    None
                }
            } else {
                None
            }
        }
        "look" => {
            if parts.len() >= 3 {
                if let (Ok(yaw), Ok(pitch)) = (
                    parts[1].parse::<f32>(),
                    parts[2].parse::<f32>(),
                ) {
                    Some(ConsoleCommandType::Look(LookCommand { yaw, pitch }))
                } else {
                    None
                }
            } else {
                None
            }
        }
        "give" => {
            if parts.len() >= 2 {
                let count = if parts.len() >= 3 {
                    parts[2].parse::<u32>().unwrap_or(64)
                } else {
                    64
                };
                Some(ConsoleCommandType::Give(GiveCommand {
                    item_type: parts[1].to_string(),
                    count,
                }))
            } else {
                None
            }
        }
        "save_map" => {
            if parts.len() >= 2 {
                Some(ConsoleCommandType::SaveMap(SaveMapCommand {
                    filename: parts[1].to_string(),
                }))
            } else {
                None
            }
        }
        "load_map" => {
            if parts.len() >= 2 {
                Some(ConsoleCommandType::LoadMap(LoadMapCommand {
                    filename: parts[1].to_string(),
                }))
            } else {
                None
            }
        }
        "list" => Some(ConsoleCommandType::List(ListCommand {})),
        _ => None,
    }
}

#[derive(Debug)]
pub enum ConsoleCommandType {
    Blink(BlinkCommand),
    Mqtt(MqttCommand),
    Spawn(SpawnCommand),
    PlaceBlock(PlaceBlockCommand),
    RemoveBlock(RemoveBlockCommand),
    Wall(WallCommand),
    Teleport(TeleportCommand),
    Look(LookCommand),
    Give(GiveCommand),
    SaveMap(SaveMapCommand),
    LoadMap(LoadMapCommand),
    List(ListCommand),
}
