//! Integration Tests for IoTCraft Desktop Client
//!
//! This module contains integration tests that require external infrastructure
//! such as MQTT servers, file system access, or network connectivity.

pub mod mqtt_test_infrastructure;

// Re-export common test utilities
pub use mqtt_test_infrastructure::*;
