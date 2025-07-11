use bevy::prelude::*;

#[derive(Resource)]
pub struct ScriptExecutor {
    pub commands: Vec<String>,
    pub current_index: usize,
    pub delay_timer: Timer,
    pub startup_script: Option<String>,
    pub execute_startup: bool,
}

impl Default for ScriptExecutor {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
            current_index: 0,
            delay_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            startup_script: None,
            execute_startup: false,
        }
    }
}

#[derive(Resource)]
pub struct PendingCommands {
    pub commands: Vec<String>,
}
