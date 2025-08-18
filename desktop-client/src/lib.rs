// IoTCraft Desktop Client - Web Version (Enhanced Gradual Build)
#[cfg(target_arch = "wasm32")]
mod lib_gradual;

#[cfg(target_arch = "wasm32")]
mod web_menu;

#[cfg(target_arch = "wasm32")]
pub use lib_gradual::*;
