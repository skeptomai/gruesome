use std::process::Command;

#[test]
fn test_disassembler_routine_count() {
    // Build the disassembler
    let build_output = Command::new("cargo")
        .args(&["build", "--bin", "gruedasm-txd"])
        .output()
        .expect("Failed to build disassembler");

    assert!(
        build_output.status.success(),
        "Failed to build gruedasm-txd"
    );

    // Run disassembler on Zork I
    let output = Command::new("./target/debug/gruedasm-txd")
        .arg("resources/test/zork1/DATA/ZORK1.DAT")
        .output()
        .expect("Failed to run disassembler");

    assert!(output.status.success(), "Disassembler failed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Count routines - should find 449 for Zork I
    let routine_count = stdout.matches("Routine R").count();
    assert_eq!(
        routine_count, 449,
        "Expected 449 routines for Zork I, found {}",
        routine_count
    );

    // Check for expected header
    assert!(
        stdout.contains("Resident data ends at"),
        "Missing header info"
    );
    assert!(
        stdout.contains("program starts at"),
        "Missing program start info"
    );
    assert!(
        stdout.contains("[Start of code]"),
        "Missing code start marker"
    );
    assert!(stdout.contains("[End of code]"), "Missing code end marker");
}

#[test]
fn test_disassembler_v4_game() {
    // Test with AMFV (v4 game)
    let amfv_path = "resources/test/amfv/amfv-r79-s851122.z4";

    // Check if AMFV test file exists
    if !std::path::Path::new(amfv_path).exists() {
        eprintln!("Skipping AMFV test - test file not found");
        return;
    }

    let build_output = Command::new("cargo")
        .args(&["build", "--bin", "gruedasm-txd"])
        .output()
        .expect("Failed to build disassembler");

    assert!(
        build_output.status.success(),
        "Failed to build gruedasm-txd"
    );

    let output = Command::new("./target/debug/gruedasm-txd")
        .arg(amfv_path)
        .output()
        .expect("Failed to run disassembler");

    assert!(output.status.success(), "Disassembler failed on v4 game");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Count routines - should find 1025 for AMFV
    let routine_count = stdout.matches("Routine R").count();
    assert_eq!(
        routine_count, 1025,
        "Expected 1025 routines for AMFV, found {}",
        routine_count
    );
}

#[test]
fn test_disassembler_output_format() {
    let build_output = Command::new("cargo")
        .args(&["build", "--bin", "gruedasm-txd"])
        .output()
        .expect("Failed to build disassembler");

    assert!(
        build_output.status.success(),
        "Failed to build gruedasm-txd"
    );

    // Test default mode (with labels)
    let output = Command::new("./target/debug/gruedasm-txd")
        .arg("resources/test/zork1/DATA/ZORK1.DAT")
        .output()
        .expect("Failed to run disassembler");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check for label-based formatting
    assert!(stdout.contains("Routine R0001"), "Missing routine labels");
    assert!(
        stdout.contains("L00") || stdout.contains("L01"),
        "Missing local variable references"
    );

    // Check for common opcodes
    assert!(stdout.contains("CALL"), "Missing CALL instructions");
    assert!(
        stdout.contains("JE") || stdout.contains("JZ"),
        "Missing jump instructions"
    );
    assert!(stdout.contains("PRINT"), "Missing PRINT instructions");
    assert!(
        stdout.contains("RET") || stdout.contains("RTRUE") || stdout.contains("RFALSE"),
        "Missing return instructions"
    );

    // Check for proper formatting of a routine
    let lines: Vec<&str> = stdout.lines().collect();
    let mut found_routine = false;
    for i in 0..lines.len() {
        if lines[i].starts_with("Routine R") {
            found_routine = true;
            // Check that the routine has local count
            assert!(
                lines[i].contains("local"),
                "Routine header missing local count"
            );
            // Check next lines have proper indentation
            if i + 1 < lines.len() && !lines[i + 1].is_empty() {
                assert!(
                    lines[i + 1].starts_with("       ") || lines[i + 1].starts_with("L"),
                    "Instructions not properly indented"
                );
            }
            break;
        }
    }
    assert!(found_routine, "No properly formatted routine found");
}

#[test]
fn test_disassembler_address_mode() {
    let build_output = Command::new("cargo")
        .args(&["build", "--bin", "gruedasm-txd"])
        .output()
        .expect("Failed to build disassembler");

    assert!(
        build_output.status.success(),
        "Failed to build gruedasm-txd"
    );

    // Test with -n flag (address mode)
    let output = Command::new("./target/debug/gruedasm-txd")
        .args(&["-n", "resources/test/zork1/DATA/ZORK1.DAT"])
        .output()
        .expect("Failed to run disassembler");

    assert!(output.status.success(), "Disassembler failed with -n flag");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check for address-based formatting
    assert!(stdout.contains("Routine 4e"), "Missing routine addresses");
    assert!(
        !stdout.contains("Routine R0"),
        "Should not have R labels in -n mode"
    );

    // Check for instruction addresses
    let has_instruction_addresses = stdout.lines().any(|line| {
        line.contains(":  ") && (line.contains("4e") || line.contains("4f") || line.contains("50"))
    });
    assert!(
        has_instruction_addresses,
        "Missing instruction addresses in -n mode"
    );
}

#[test]
fn test_specific_routine_content() {
    // Test that specific routines are decoded correctly
    let build_output = Command::new("cargo")
        .args(&["build", "--bin", "gruedasm-txd"])
        .output()
        .expect("Failed to build disassembler");

    assert!(
        build_output.status.success(),
        "Failed to build gruedasm-txd"
    );

    let output = Command::new("./target/debug/gruedasm-txd")
        .arg("resources/test/zork1/DATA/ZORK1.DAT")
        .output()
        .expect("Failed to run disassembler");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Look for the first few routines and verify they have expected structure
    let lines: Vec<&str> = stdout.lines().collect();
    let mut routine_count = 0;

    for i in 0..lines.len() {
        if lines[i].starts_with("Routine R0001") {
            routine_count += 1;
            // First routine should be simple
            assert!(lines[i].contains("1 local"), "R0001 should have 1 local");

            // Should have some instructions
            let mut has_instructions = false;
            for j in i + 1..i + 10 {
                if j < lines.len() && !lines[j].is_empty() && !lines[j].starts_with("Routine") {
                    has_instructions = true;
                    break;
                }
            }
            assert!(has_instructions, "R0001 has no instructions");
        }

        if routine_count >= 3 {
            break; // Check first 3 routines
        }
    }

    assert!(routine_count > 0, "No routines found in output");
}
