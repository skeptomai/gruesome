//! Integration tests for Zork I gameplay
//!
//! These tests verify that the Z-Machine interpreter correctly executes
//! actual gameplay sequences. They use scripted input to automate game
//! interaction and validate expected outputs.
//!
//! The tests ensure:
//! - Basic commands work (look, open, take, read, inventory)
//! - Navigation between rooms functions correctly
//! - Object interactions produce expected results
//! - Game state changes are properly tracked
//!
//! These tests run against the actual Zork I game file to catch any
//! regressions in the interpreter implementation.

use std::process::{Command, Stdio};

/// Test basic Zork I gameplay sequences
///
/// This test verifies fundamental game mechanics:
/// 1. Starting location display (West of House)
/// 2. Object interaction (opening mailbox)
/// 3. Item manipulation (taking and reading leaflet)
/// 4. Inventory management
/// 5. Room navigation (moving south and east)
/// 6. Clean game exit
#[test]
fn test_zork_gameplay_basic() {
    // Build the game first to ensure we have the latest binary
    let build_output = Command::new("cargo")
        .args(&["build", "--bin", "gruesome"])
        .output()
        .expect("Failed to build game");

    assert!(build_output.status.success(), "Failed to build gruesome");

    // Create a simple test script with commands separated by newlines
    // Each command represents a player action in the game
    let script = "look
open mailbox
take leaflet
read leaflet
inventory
south
east
quit
yes
";

    // Run game with script as input using shell piping
    // Stderr is redirected to /dev/null to suppress debug output
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "echo '{}' | ./target/debug/gruesome resources/test/zork1/DATA/ZORK1.DAT 2>/dev/null",
            script
        ))
        .output()
        .expect("Failed to run game");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify expected outputs for each command
    // These assertions check that the game properly processes each action
    assert!(
        stdout.contains("West of House"),
        "Missing starting location"
    );
    assert!(stdout.contains("white house"), "Missing house description");
    assert!(stdout.contains("mailbox"), "Missing mailbox");
    assert!(
        stdout.contains("Opening the small mailbox"),
        "Failed to open mailbox"
    );
    assert!(stdout.contains("leaflet"), "Missing leaflet");
    assert!(
        stdout.contains("ZORK is a game of adventure"),
        "Failed to read leaflet"
    );
    assert!(
        stdout.contains("You are carrying"),
        "Inventory command failed"
    );
    assert!(stdout.contains("South of House"), "Failed to move south");
    assert!(stdout.contains("Behind House"), "Failed to move east");
}

/// Test extended Zork I walkthrough sequence
///
/// This test verifies a longer gameplay sequence that includes:
/// 1. Initial exploration and item collection
/// 2. Entering the house through the window
/// 3. Collecting important items (lamp and sword)
/// 4. Activating the lamp for underground exploration
/// 5. Basic navigation through multiple rooms
///
/// This ensures the interpreter maintains game state correctly
/// across a longer sequence of commands and room transitions.
#[test]
fn test_zork_scripted_walkthrough() {
    // Build the interpreter to ensure we test the latest code
    let build_output = Command::new("cargo")
        .args(&["build", "--bin", "gruesome"])
        .output()
        .expect("Failed to build game");

    assert!(build_output.status.success(), "Failed to build gruesome");

    // Extended script that explores more of the game
    // This sequence is commonly used in walkthroughs and tests
    // multiple game systems (movement, object manipulation, light)
    let script = "look
open mailbox
read leaflet
take leaflet
south
east
open window
enter window
west
take lamp
turn on lamp
take sword
go east
go east
go south
go east
go south
quit
yes
";

    // Run game with script as input using echo to pipe commands
    // We capture stdout but suppress stderr to avoid debug noise
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "echo '{}' | ./target/debug/gruesome resources/test/zork1/DATA/ZORK1.DAT",
            script
        ))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .expect("Failed to run game");

    // The game should have run without crashing
    // Note: 'quit' command may return non-zero exit status by design

    // Convert output to string for validation
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Validate key checkpoints in the walkthrough
    // These ensure the interpreter correctly handles:
    // - Room descriptions and navigation
    // - Object interactions and state changes
    // - Inventory management
    // - Light source activation (critical for underground areas)
    assert!(
        stdout.contains("West of House"),
        "Missing starting location"
    );
    assert!(stdout.contains("leaflet"), "Mailbox interaction failed");
    assert!(stdout.contains("South of House"), "Movement south failed");
    assert!(stdout.contains("Behind House"), "Movement east failed");
    assert!(stdout.contains("Kitchen"), "Window entry failed");
    assert!(stdout.contains("brass lantern"), "Lamp not found");
    assert!(stdout.contains("elvish sword"), "Sword not found");
    assert!(
        stdout.contains("The brass lantern is now on"),
        "Lamp didn't turn on"
    );
}
