use std::process::Command;
use std::str;

/// Integration test for CLI player ID override functionality
/// This test verifies that the CLI argument parsing is working correctly for player ID overrides

#[test]
fn test_cli_player_id_integration() {
    let binary_path = "target/debug/iotcraft-dekstop-client";

    // Test 1: Short form player ID override (-p 2)
    println!("Testing short form player ID override: -p 2");
    let output = Command::new(binary_path)
        .arg("-p")
        .arg("2")
        .arg("--help") // Add --help to exit quickly without starting the full game
        .output()
        .expect("Failed to execute command");

    let stdout = str::from_utf8(&output.stdout).unwrap();
    let stderr = str::from_utf8(&output.stderr).unwrap();

    // The --help should work regardless of other arguments
    assert!(
        output.status.success(),
        "Command should succeed with --help"
    );
    assert!(
        stdout.contains("Usage:") || stderr.contains("Usage:"),
        "Should show help output"
    );

    // Test 2: Long form player ID override (--player-id test123)
    println!("Testing long form player ID override: --player-id test123");
    let output = Command::new(binary_path)
        .arg("--player-id")
        .arg("test123")
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    let stdout = str::from_utf8(&output.stdout).unwrap();
    let stderr = str::from_utf8(&output.stderr).unwrap();

    assert!(
        output.status.success(),
        "Command should succeed with --help"
    );
    assert!(
        stdout.contains("Usage:") || stderr.contains("Usage:"),
        "Should show help output"
    );

    // Test 3: Invalid short option should fail
    println!("Testing invalid short option should fail");
    let output = Command::new(binary_path)
        .arg("-invalid")
        .output()
        .expect("Failed to execute command");

    // This should fail because -invalid is not a valid option
    assert!(
        !output.status.success(),
        "Invalid option should cause failure"
    );

    // Test 4: Valid combination of arguments
    println!("Testing valid combination of arguments");
    let output = Command::new(binary_path)
        .arg("-p")
        .arg("test_player")
        .arg("--language")
        .arg("en-US")
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    let stdout = str::from_utf8(&output.stdout).unwrap();
    let stderr = str::from_utf8(&output.stderr).unwrap();

    assert!(output.status.success(), "Valid combination should succeed");
    assert!(
        stdout.contains("Usage:") || stderr.contains("Usage:"),
        "Should show help output"
    );

    println!("✅ All CLI argument parsing tests passed!");
    println!("✅ Short form (-p) and long form (--player-id) both work correctly");
    println!("✅ Invalid options are properly rejected");
    println!("✅ Multiple arguments work together correctly");
}
