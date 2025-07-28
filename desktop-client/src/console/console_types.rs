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
