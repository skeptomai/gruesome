use std::process::{Command, Stdio};

#[test]
fn test_zork_gameplay_basic() {
    // Build the game first
    let build_output = Command::new("cargo")
        .args(&["build", "--bin", "gruesome"])
        .output()
        .expect("Failed to build game");

    assert!(build_output.status.success(), "Failed to build gruesome");

    // Create a simple test script
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

    // Run game with script as input
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "echo '{}' | ./target/debug/gruesome resources/test/zork1/DATA/ZORK1.DAT 2>/dev/null",
            script
        ))
        .output()
        .expect("Failed to run game");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify expected outputs
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

#[test]
fn test_zork_scripted_walkthrough() {
    // This test runs a longer scripted walkthrough
    let build_output = Command::new("cargo")
        .args(&["build", "--bin", "gruesome"])
        .output()
        .expect("Failed to build game");

    assert!(build_output.status.success(), "Failed to build gruesome");

    // Create a script file with commands
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
    // We can't easily check the exit status as 'quit' might return non-zero

    // Convert output to string for validation
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check for expected text in various locations
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
