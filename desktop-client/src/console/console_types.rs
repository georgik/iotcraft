use bevy::prelude::*;
use bevy_console::ConsoleCommand;
use clap::Parser;

// Console commands for bevy_console
#[derive(Parser, ConsoleCommand)]
#[command(name = "blink")]
pub struct BlinkCommand {
    /// Action to perform: start or stop
    pub action: String,
}

#[derive(Parser, ConsoleCommand)]
#[command(name = "mqtt")]
pub struct MqttCommand {
    /// MQTT action: status or reconnect
    pub action: String,
}

#[derive(Parser, ConsoleCommand)]
#[command(name = "spawn")]
pub struct SpawnCommand {
    /// Device ID
    pub device_id: String,
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
    /// Z coordinate
    pub z: f32,
}

#[derive(Parser, ConsoleCommand)]
#[command(name = "spawn_door")]
pub struct SpawnDoorCommand {
    /// Device ID
    pub device_id: String,
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
    /// Z coordinate
    pub z: f32,
}

#[derive(Parser, ConsoleCommand)]
#[command(name = "load")]
pub struct LoadCommand {
    /// Script file to load and execute
    pub filename: String,
}

#[derive(Parser, ConsoleCommand)]
#[command(name = "move")]
pub struct MoveCommand {
    /// Device ID to move
    pub device_id: String,
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
    /// Z coordinate
    pub z: f32,
}

#[derive(Parser, ConsoleCommand)]
#[command(name = "place")]
pub struct PlaceBlockCommand {
    /// Block type: grass, dirt, or stone
    pub block_type: String,
    /// X coordinate
    pub x: i32,
    /// Y coordinate
    pub y: i32,
    /// Z coordinate
    pub z: i32,
}

#[derive(Parser, ConsoleCommand)]
#[command(name = "remove")]
pub struct RemoveBlockCommand {
    /// X coordinate
    pub x: i32,
    /// Y coordinate
    pub y: i32,
    /// Z coordinate
    pub z: i32,
}

#[derive(Parser, ConsoleCommand)]
#[command(name = "wall")]
pub struct WallCommand {
    /// Block type: grass, dirt, or stone
    pub block_type: String,
    /// X1 coordinate
    pub x1: i32,
    /// Y1 coordinate
    pub y1: i32,
    /// Z1 coordinate
    pub z1: i32,
    /// X2 coordinate
    pub x2: i32,
    /// Y2 coordinate
    pub y2: i32,
    /// Z2 coordinate
    pub z2: i32,
}

#[derive(Parser, ConsoleCommand)]
#[command(name = "save_map")]
pub struct SaveMapCommand {
    /// Filename to save the map to
    pub filename: String,
}

#[derive(Parser, ConsoleCommand)]
#[command(name = "load_map")]
pub struct LoadMapCommand {
    /// Filename to load the map from
    pub filename: String,
}

#[derive(Parser, ConsoleCommand)]
#[command(name = "give")]
pub struct GiveCommand {
    /// Item type to give (grass, dirt, stone, quartz_block, glass_pane, cyan_terracotta)
    pub item_type: String,
    /// Number of items to give (default: 64)
    #[arg(default_value_t = 64)]
    pub count: u32,
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
