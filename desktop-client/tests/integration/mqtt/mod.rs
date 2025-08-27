//! MQTT Integration Tests
//! 
//! This module contains integration tests for MQTT functionality
//! that require a running MQTT server.

pub mod simple_test;
pub mod utils;
// Note: Other test files can be added as needed

// Re-export utilities
pub use utils::*;
