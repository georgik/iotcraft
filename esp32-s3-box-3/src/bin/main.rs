#![no_std]
#![no_main]

use esp_hal::main;
slint::include_modules!();

#[main]
fn main() -> ! {
    // Initialize the MCU board (GPIO, I2C, etc.)
    mcu_board_support::init();

    // Create and show the ThermoWindow defined in thermo.slint
    let window = ThermoWindow::new().unwrap();
    window.run().unwrap();

    // Prevent the function from returning
    loop {}
}
