// Web-compatible script system for IoTCraft
use bevy::prelude::*;
use log::{error, info};

// Re-export script types for compatibility
pub mod script_types {
    use bevy::prelude::*;

    #[derive(Resource, Default)]
    pub struct PendingCommands {
        pub commands: Vec<String>,
    }
}

pub use script_types::*;

pub struct ScriptPlugin;

impl Plugin for ScriptPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingCommands>()
            .add_systems(Update, execute_background_world_script.run_if(run_once()));
    }
}

/// Resource to ensure background world setup only runs once
#[derive(Resource, Default)]
struct BackgroundWorldSetupComplete(bool);

/// Web-compatible background world script execution
/// Uses hardcoded script content since we can't read files in WASM
fn execute_background_world_script(
    mut commands: Commands,
    mut pending_commands: ResMut<PendingCommands>,
) {
    info!("Executing background world script for web version");

    // Background world script content (from scripts/background_world.txt)
    let background_script = vec![
        "# Background World Script".to_string(),
        "# Creates a scenic background for menu screens".to_string(),
        "".to_string(),
        "# Create a smaller grass base".to_string(),
        "wall grass -15 0 -15 15 0 15".to_string(),
        "".to_string(),
        "# Create rolling hills".to_string(),
        "wall dirt -10 1 -10 -5 2 -5".to_string(),
        "wall grass -10 3 -10 -5 3 -5".to_string(),
        "".to_string(),
        "wall dirt 5 1 5 10 3 10".to_string(),
        "wall grass 5 4 5 10 4 10".to_string(),
        "".to_string(),
        "# Add some variety blocks for visual interest".to_string(),
        "place stone -8 1 8".to_string(),
        "place quartz_block 8 1 -8".to_string(),
        "place glass_pane 0 1 12".to_string(),
        "place cyan_terracotta 12 1 0".to_string(),
        "".to_string(),
        "# Create a small tower for interest".to_string(),
        "wall stone 0 1 0 0 5 0".to_string(),
        "place quartz_block 0 6 0".to_string(),
    ];

    let script_commands = background_script
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect::<Vec<String>>();

    info!("Executing background world script with {} commands", script_commands.len());
    pending_commands.commands.extend(script_commands);

    // Add background setup complete resource to prevent re-execution
    commands.insert_resource(BackgroundWorldSetupComplete(true));
}

/// Get the new world script content for web
pub fn get_new_world_script() -> Vec<String> {
    // New world script content (from scripts/new_world.txt)
    vec![
        "# New World Initialization Script".to_string(),
        "# Creates a basic world with grass plains and hills".to_string(),
        "".to_string(),
        "# Set initial camera position and orientation for better view".to_string(),
        "tp -8 3 15".to_string(),
        "look -34 -3".to_string(),
        "".to_string(),
        "# Create a large grass plain (80x80 area)".to_string(),
        "wall grass -40 0 -40 40 0 40".to_string(),
        "".to_string(),
        "# Create some hills around the plain".to_string(),
        "# Hill 1 - Southeast".to_string(),
        "wall dirt 15 1 15 20 3 20".to_string(),
        "wall grass 15 4 15 20 4 20".to_string(),
        "".to_string(),
        "# Hill 2 - Northwest".to_string(),
        "wall dirt -20 1 -20 -15 4 -15".to_string(),
        "wall grass -20 5 -20 -15 5 -15".to_string(),
        "".to_string(),
        "# Hill 3 - Northeast".to_string(),
        "wall dirt 18 1 -18 22 2 -14".to_string(),
        "wall grass 18 3 -18 22 3 -14".to_string(),
        "".to_string(),
        "# Hill 4 - Southwest".to_string(),
        "wall dirt -22 1 18 -18 3 22".to_string(),
        "wall grass -22 4 18 -18 4 22".to_string(),
        "".to_string(),
        "# Create a small structure away from spawn".to_string(),
        "wall stone 3 1 3 7 3 7".to_string(),
        "wall stone 4 4 4 6 4 6".to_string(),
        "".to_string(),
        "# Place some decorative blocks".to_string(),
        "place quartz_block 5 5 5".to_string(),
        "place glass_pane 10 1 10".to_string(),
        "place glass_pane -10 1 -10".to_string(),
        "place cyan_terracotta 5 1 -5".to_string(),
        "place cyan_terracotta -5 1 5".to_string(),
        "".to_string(),
        "# Create a contained water pond in the corner of the map".to_string(),
        "# North wall (z=-26)".to_string(),
        "wall stone 21 1 -26 26 1 -26".to_string(),
        "# South wall (z=-21)".to_string(),
        "wall stone 21 1 -21 26 1 -21".to_string(),
        "# West wall (x=21)".to_string(),
        "wall stone 21 1 -26 21 1 -21".to_string(),
        "# East wall (x=26)".to_string(),
        "wall stone 26 1 -26 26 1 -21".to_string(),
        "".to_string(),
        "# Fill the inside with water at level 1".to_string(),
        "wall water 22 1 -25 25 1 -22".to_string(),
        "".to_string(),
        "# Give the player some starting items for building".to_string(),
        "give grass 64".to_string(),
        "give dirt 32".to_string(),
        "give stone 32".to_string(),
        "give quartz_block 16".to_string(),
        "give glass_pane 8".to_string(),
        "give cyan_terracotta 8".to_string(),
        "give water 8".to_string(),
    ]
}

/// Execute new world script commands
pub fn execute_new_world_script(pending_commands: &mut ResMut<PendingCommands>) {
    let script_content = get_new_world_script();

    let script_commands = script_content
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect::<Vec<String>>();

    info!("Executing new world script with {} commands", script_commands.len());
    pending_commands.commands.extend(script_commands);
}
