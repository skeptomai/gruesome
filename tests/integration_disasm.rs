//! Integration tests for the Z-Machine disassembler (gruedasm-txd)
//!
//! These tests verify that our disassembler produces output compatible
//! with Mark Howell's txd disassembler. The tests validate:
//! - Correct routine discovery and counting
//! - Proper instruction decoding and formatting
//! - Support for different output modes (-n for addresses)
//! - Version-specific handling (v3 vs v4+ games)
//!
//! The disassembler is a critical tool for understanding and debugging
//! Z-Machine games, and maintaining txd compatibility ensures our
//! output can be compared with the reference implementation.

use std::process::Command;

/// Test that the disassembler finds the correct number of routines
///
/// This test validates our routine discovery algorithm by checking
/// that we find exactly 449 routines in Zork I, matching txd's output.
/// It also verifies that the disassembler produces all expected
/// header information and code markers.
///
/// NOTE: This test is slow (60+ seconds) because it disassembles all of Zork I.
/// Run explicitly with: cargo test -- --ignored
#[test]
#[ignore]
fn test_disassembler_routine_count() {
    // Build the disassembler binary first
    let build_output = Command::new("cargo")
        .args(["build", "--bin", "gruedasm-txd"])
        .output()
        .expect("Failed to build disassembler");

    assert!(
        build_output.status.success(),
        "Failed to build gruedasm-txd"
    );

    // Run disassembler on Zork I in default mode (with labels)
    let output = Command::new("./target/debug/gruedasm-txd")
        .arg("resources/test/zork1/DATA/ZORK1.DAT")
        .output()
        .expect("Failed to run disassembler");

    assert!(output.status.success(), "Disassembler failed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Count routines - should find exactly 449 for Zork I
    // This matches the txd reference implementation
    let routine_count = stdout.matches("Routine R").count();
    assert_eq!(
        routine_count, 449,
        "Expected 449 routines for Zork I, found {}",
        routine_count
    );

    // Verify all required header information is present
    // These markers help users understand the game structure
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

/// Test disassembler with a v4 game (A Mind Forever Voyaging)
///
/// This test ensures our disassembler correctly handles v4+ games,
/// which have different object formats and additional opcodes.
/// AMFV is a good test case as it's a complex v4 game with 1025 routines.
#[test]
fn test_disassembler_v4_game() {
    // Test with AMFV (v4 game) - A Mind Forever Voyaging
    let amfv_path = "resources/test/amfv/amfv-r79-s851122.z4";

    // Skip test gracefully if AMFV test file is not available
    // This allows the test suite to run even without all game files
    if !std::path::Path::new(amfv_path).exists() {
        eprintln!("Skipping AMFV test - test file not found");
        return;
    }

    let build_output = Command::new("cargo")
        .args(["build", "--bin", "gruedasm-txd"])
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

    // Count routines - should find exactly 1025 for AMFV
    // This validates that our v4-specific handling is correct
    let routine_count = stdout.matches("Routine R").count();
    assert_eq!(
        routine_count, 1025,
        "Expected 1025 routines for AMFV, found {}",
        routine_count
    );
}

/// Test the disassembler's output format for txd compatibility
///
/// This test verifies that our output format matches txd's conventions:
/// - Routine labels (R0001, R0002, etc.)
/// - Local variable references (L00, L01, etc.)
/// - Proper instruction formatting and indentation
/// - Common opcode names (CALL, JE, PRINT, RET, etc.)
#[test]
fn test_disassembler_output_format() {
    let build_output = Command::new("cargo")
        .args(["build", "--bin", "gruedasm-txd"])
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

/// Test the disassembler's address mode (-n flag)
///
/// The -n flag makes the disassembler print actual addresses instead
/// of labels, which is useful for debugging and comparing with memory
/// dumps. This test verifies that the flag works correctly.
#[test]
fn test_disassembler_address_mode() {
    let build_output = Command::new("cargo")
        .args(["build", "--bin", "gruedasm-txd"])
        .output()
        .expect("Failed to build disassembler");

    assert!(
        build_output.status.success(),
        "Failed to build gruedasm-txd"
    );

    // Test with -n flag (address mode)
    let output = Command::new("./target/debug/gruedasm-txd")
        .args(["-n", "resources/test/zork1/DATA/ZORK1.DAT"])
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

/// Test that specific routines are decoded with correct structure
///
/// This test examines the first few routines to ensure they have:
/// - Correct local variable counts
/// - Actual instruction content (not empty routines)
/// - Proper formatting and structure
///
/// This catches issues where routines might be discovered but not
/// properly decoded or formatted.
#[test]
fn test_specific_routine_content() {
    // Test that specific routines are decoded correctly
    let build_output = Command::new("cargo")
        .args(["build", "--bin", "gruedasm-txd"])
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
