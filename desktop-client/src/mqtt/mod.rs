pub mod mqtt_helpers;
pub mod mqtt_types;

// Platform-specific MQTT implementations
#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(target_arch = "wasm32")]
pub mod web;

// Export the appropriate plugin based on target
#[cfg(not(target_arch = "wasm32"))]
pub use native::NativeMqttPlugin as MqttPlugin;
#[cfg(target_arch = "wasm32")]
pub use web::WebMqttPlugin as MqttPlugin;

pub use mqtt_types::*;

// Web-specific timestamp function for WASM
#[cfg(target_arch = "wasm32")]
pub fn now_ts_web() -> u64 {
    js_sys::Date::now() as u64
}
