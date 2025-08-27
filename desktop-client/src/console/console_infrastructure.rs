use bevy::prelude::*;

/// Console set for system ordering
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct ConsoleSet;

impl ConsoleSet {
    pub const COMMANDS: ConsoleSet = ConsoleSet;
}

/// Reply macro to send responses back to console
#[macro_export]
macro_rules! reply {
    ($log:expr, $($arg:tt)*) => {{
        let message = format!($($arg)*);
        // For now, just log the reply - in a full implementation this would send to console output
        info!("Console reply: {}", message);
    }};
}
