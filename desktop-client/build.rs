// Build script to set compile-time environment variables
use chrono::Utc;

fn main() {
    // Get current UTC timestamp in a readable format
    let build_timestamp = Utc::now().format("%Y-%m-%d-%H:%M-UTC").to_string();

    // Set the environment variable for use with env! macro
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", build_timestamp);

    // Tell cargo to rerun this build script if it changes
    println!("cargo:rerun-if-changed=build.rs");
}
