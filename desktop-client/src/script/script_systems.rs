use bevy::prelude::*;
use super::{script_types::*, script_helpers::*};
use crate::console::{PrintConsoleLine, BlinkState};
use crate::mqtt::TemperatureResource;

pub struct ScriptPlugin;

impl Plugin for ScriptPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ScriptExecutor::default())
            .insert_resource(PendingCommands { commands: Vec::new() })
            .add_systems(Update, script_execution_system)
            .add_systems(Update, execute_pending_commands);
    }
}

pub fn script_execution_system(
    mut script_executor: ResMut<ScriptExecutor>,
    time: Res<Time>,
    mut pending_commands: ResMut<PendingCommands>,
) {
    if script_executor.execute_startup {
        if let Some(ref startup_script) = script_executor.startup_script.clone() {
            if let Ok(content) = std::fs::read_to_string(startup_script) {
                let commands = execute_script(&content);
                script_executor.commands = commands;
                script_executor.current_index = 0;
                info!("Loaded startup script: {}", startup_script);
            }
        }
        script_executor.execute_startup = false;
    }

    if !script_executor.commands.is_empty() && script_executor.current_index < script_executor.commands.len() {
        script_executor.delay_timer.tick(time.delta());

        if script_executor.delay_timer.just_finished() {
            let command = &script_executor.commands[script_executor.current_index];
            info!("Executing script command: {}", command);
            pending_commands.commands.push(command.clone());

            script_executor.current_index += 1;

            if script_executor.current_index >= script_executor.commands.len() {
                script_executor.commands.clear();
                script_executor.current_index = 0;
                info!("Script execution completed");
            }
        }
    }
}

pub fn execute_pending_commands(
    mut pending_commands: ResMut<PendingCommands>,
    mut print_console_line: EventWriter<super::console::PrintConsoleLine>,
    mut blink_state: ResMut<super::console::BlinkState>,
    temperature: Res<crate::mqtt::TemperatureResource>,
) {
    for command in pending_commands.commands.drain(..) {
        info!("Executing queued command: {}", command);
        // Additional command execution logic here.
    }
}

