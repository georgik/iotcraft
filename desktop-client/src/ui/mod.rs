pub mod crosshair;
pub mod error_indicator;
pub mod inventory_ui;

// Use different main menu implementations for desktop vs web
#[cfg(not(target_arch = "wasm32"))]
pub mod main_menu;
#[cfg(target_arch = "wasm32")]
pub mod web_main_menu;

pub use crosshair::*;
pub use error_indicator::*;
pub use inventory_ui::*;

// Re-export the appropriate main menu implementation
#[cfg(not(target_arch = "wasm32"))]
pub use main_menu::*;
#[cfg(target_arch = "wasm32")]
pub use web_main_menu::*;
