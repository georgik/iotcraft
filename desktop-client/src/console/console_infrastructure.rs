use bevy::prelude::*;
use std::marker::PhantomData;

/// Resource to track if the console is open
#[derive(Resource, Default)]
pub struct ConsoleOpen {
    pub open: bool,
}

/// Event for printing lines to the console output
#[derive(Event, BufferedEvent)]
pub struct PrintConsoleLine {
    pub line: String,
}

impl PrintConsoleLine {
    pub fn new(line: String) -> Self {
        Self { line }
    }
}

/// Generic command struct that holds console command data
pub struct ConsoleCommand<T: Clone> {
    pub data: Option<Result<T, String>>,
}

impl<T: Clone> ConsoleCommand<T> {
    pub fn new(data: T) -> Self {
        Self {
            data: Some(Ok(data)),
        }
    }

    pub fn new_error(error: String) -> Self {
        Self {
            data: Some(Err(error)),
        }
    }

    pub fn empty() -> Self {
        Self { data: None }
    }

    pub fn take(&mut self) -> Option<Result<T, String>> {
        self.data.take()
    }
}

/// Console set for system ordering
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct ConsoleSet;

impl ConsoleSet {
    pub const Commands: ConsoleSet = ConsoleSet;
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

/// Console command trait for parsing commands from strings
pub trait ConsoleCommandParser: Clone + Send + Sync + 'static {
    fn parse(input: &str) -> Result<Self, String>;
}
