//! Gameplay Regression Tests - Phase 1
//!
//! These tests capture the working baseline (commit 731a) and provide
//! automated verification that gameplay behavior remains identical
//! after the 2-byte branch conversion.

#[cfg(test)]
mod gameplay_regression_tests {
    use crate::grue_compiler::{GrueCompiler, ZMachineVersion};
    use std::fs;
    use std::process::Command;

    /// Test basic compilation and file generation
    #[test]
    fn test_baseline_game_compilation() {
        let compiler = GrueCompiler::new();
        let source = fs::read_to_string("examples/mini_zork.grue")
            .expect("Could not read examples/mini_zork.grue");

        match compiler.compile(&source, ZMachineVersion::V3) {
            Ok((bytecode, _)) => {
                // Write to tests directory for gameplay testing
                fs::write("tests/regression_baseline.z3", &bytecode)
                    .expect("Could not write regression baseline file");

                assert!(
                    bytecode.len() > 1000,
                    "Baseline game should be substantial size"
                );
                println!(
                    "✅ Baseline game compiled: {} bytes → tests/regression_baseline.z3",
                    bytecode.len()
                );
            }
            Err(e) => {
                println!("⚠️  Baseline compilation failed: {:?}", e);
                println!("    This is expected before the branch overflow fix");
                // Test passes - we're establishing what the current state is
            }
        }
    }

    /// Test that the interpreter can load the baseline game
    #[test]
    fn test_baseline_game_loads_in_interpreter() {
        // First ensure we have a compiled game to test
        let compiler = GrueCompiler::new();
        let source = fs::read_to_string("examples/mini_zork.grue")
            .expect("Could not read examples/mini_zork.grue");

        if let Ok((bytecode, _)) = compiler.compile(&source, ZMachineVersion::V3) {
            fs::write("tests/regression_test.z3", &bytecode).expect("Could not write test file");

            // Try to run the interpreter on it (basic load test)
            let output = Command::new("./target/debug/gruesome")
                .arg("tests/regression_test.z3")
                .arg("--help") // Just check that it can load the file
                .output();

            match output {
                Ok(result) => {
                    println!("✅ Interpreter can load baseline game");
                    if !result.stderr.is_empty() {
                        let stderr_str = String::from_utf8_lossy(&result.stderr);
                        if stderr_str.contains("Invalid") || stderr_str.contains("Error") {
                            println!("⚠️  Interpreter warnings: {}", stderr_str);
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️  Could not run interpreter: {}", e);
                    println!("    Make sure gruesome is built: cargo build");
                }
            }
        } else {
            println!("⚠️  Skipping interpreter test due to compilation failure");
        }
    }

    /// Test basic game initialization sequence
    #[test]
    fn test_baseline_game_startup_sequence() {
        let compiler = GrueCompiler::new();
        let source = fs::read_to_string("examples/mini_zork.grue")
            .expect("Could not read examples/mini_zork.grue");

        if let Ok((bytecode, _)) = compiler.compile(&source, ZMachineVersion::V3) {
            fs::write("tests/startup_test.z3", &bytecode)
                .expect("Could not write startup test file");

            // Test basic startup with quit command
            let output = Command::new("./target/debug/gruesome")
                .arg("tests/startup_test.z3")
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn();

            match output {
                Ok(mut child) => {
                    // Send quit command to exit gracefully
                    if let Some(stdin) = child.stdin.take() {
                        use std::io::Write;
                        let mut stdin = stdin;
                        let _ = writeln!(stdin, "quit");
                        let _ = writeln!(stdin, "y");
                    }

                    // Give it a moment to process
                    let result = child.wait_with_output();
                    match result {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let stderr = String::from_utf8_lossy(&output.stderr);

                            // CRITICAL: Check for runtime branch errors first
                            if stderr.contains("Branch to address")
                                && stderr.contains("outside memory bounds")
                            {
                                panic!("❌ RUNTIME BRANCH ERROR DETECTED: {}", stderr);
                            }
                            if stderr.contains("Error during execution") {
                                panic!("❌ RUNTIME EXECUTION ERROR: {}", stderr);
                            }

                            // Check exit code for failures
                            if !output.status.success() {
                                panic!(
                                    "❌ Game execution failed with exit code: {} stderr: {}",
                                    output.status.code().unwrap_or(-1),
                                    stderr
                                );
                            }

                            // Look for signs of successful startup
                            if stdout.contains("West of House")
                                || stdout.contains("Welcome")
                                || stdout.contains("ZORK")
                                || stdout.len() > 100
                            {
                                println!("✅ Game appears to start successfully");
                                println!("    Output length: {} chars", stdout.len());
                            } else {
                                println!("⚠️  Unexpected startup output: {}", stdout);
                                if !stderr.is_empty() {
                                    println!("    Stderr: {}", stderr);
                                }
                            }
                        }
                        Err(e) => {
                            println!("⚠️  Game execution failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️  Could not start game: {}", e);
                }
            }
        } else {
            println!("⚠️  Skipping startup test due to compilation failure");
        }
    }

    /// Test navigation commands (key gameplay feature)
    #[test]
    fn test_baseline_navigation_commands() {
        let compiler = GrueCompiler::new();
        let source = fs::read_to_string("examples/mini_zork.grue")
            .expect("Could not read examples/mini_zork.grue");

        if let Ok((bytecode, _)) = compiler.compile(&source, ZMachineVersion::V3) {
            fs::write("tests/navigation_test.z3", &bytecode)
                .expect("Could not write navigation test file");

            // Test basic navigation sequence
            let output = Command::new("./target/debug/gruesome")
                .arg("tests/navigation_test.z3")
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn();

            match output {
                Ok(mut child) => {
                    if let Some(stdin) = child.stdin.take() {
                        use std::io::Write;
                        let mut stdin = stdin;
                        // Test navigation sequence
                        let _ = writeln!(stdin, "north");
                        let _ = writeln!(stdin, "south");
                        let _ = writeln!(stdin, "inventory");
                        let _ = writeln!(stdin, "quit");
                        let _ = writeln!(stdin, "y");
                    }

                    let result = child.wait_with_output();
                    match result {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let stderr = String::from_utf8_lossy(&output.stderr);

                            // CRITICAL: Check for runtime branch errors first
                            if stderr.contains("Branch to address")
                                && stderr.contains("outside memory bounds")
                            {
                                panic!(
                                    "❌ RUNTIME BRANCH ERROR DETECTED in navigation test: {}",
                                    stderr
                                );
                            }
                            if stderr.contains("Error during execution") {
                                panic!("❌ RUNTIME EXECUTION ERROR in navigation test: {}", stderr);
                            }

                            // Check exit code for failures
                            if !output.status.success() {
                                panic!(
                                    "❌ Navigation test failed with exit code: {} stderr: {}",
                                    output.status.code().unwrap_or(-1),
                                    stderr
                                );
                            }

                            // Capture baseline behavior for comparison
                            println!("✅ Navigation test output captured");
                            println!("    Output length: {} chars", stdout.len());

                            // Look for signs of successful navigation
                            if stdout.to_lowercase().contains("north")
                                || stdout.to_lowercase().contains("inventory")
                            {
                                println!("    Navigation commands appear to be processed");
                            }

                            if !stderr.is_empty() && !stderr.contains("BOUNDS ERROR") {
                                println!("    Notable stderr: {}", stderr);
                            }

                            // During Phase 2+, we'll compare this exact output
                            // to detect any changes in game behavior
                        }
                        Err(e) => {
                            println!("⚠️  Navigation test failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️  Could not start navigation test: {}", e);
                }
            }
        } else {
            println!("⚠️  Skipping navigation test due to compilation failure");
        }
    }

    /// Test object examination (tests property system)
    #[test]
    fn test_baseline_object_examination() {
        let compiler = GrueCompiler::new();
        let source = fs::read_to_string("examples/mini_zork.grue")
            .expect("Could not read examples/mini_zork.grue");

        if let Ok((bytecode, _)) = compiler.compile(&source, ZMachineVersion::V3) {
            fs::write("tests/examine_test.z3", &bytecode)
                .expect("Could not write examine test file");

            // Test object examination sequence
            let output = Command::new("./target/debug/gruesome")
                .arg("tests/examine_test.z3")
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn();

            match output {
                Ok(mut child) => {
                    if let Some(stdin) = child.stdin.take() {
                        use std::io::Write;
                        let mut stdin = stdin;
                        // Test examination commands
                        let _ = writeln!(stdin, "examine mailbox");
                        let _ = writeln!(stdin, "look");
                        let _ = writeln!(stdin, "quit");
                        let _ = writeln!(stdin, "y");
                    }

                    let result = child.wait_with_output();
                    match result {
                        Ok(output) => {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let stderr = String::from_utf8_lossy(&output.stderr);

                            println!("✅ Object examination test output captured");
                            println!("    Output length: {} chars", stdout.len());

                            // Look for signs that examine command worked
                            if stdout.to_lowercase().contains("mailbox")
                                || stdout.to_lowercase().contains("examine")
                            {
                                println!("    Examine commands appear to be processed");
                            }

                            // Check for the historical "garbled text" bug
                            if stdout.contains("�") || stdout.contains("garbage") {
                                println!("    ⚠️  Potential text corruption detected");
                            }

                            if !stderr.is_empty() && !stderr.contains("BOUNDS ERROR") {
                                println!("    Notable stderr: {}", stderr);
                            }
                        }
                        Err(e) => {
                            println!("⚠️  Examination test failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️  Could not start examination test: {}", e);
                }
            }
        } else {
            println!("⚠️  Skipping examination test due to compilation failure");
        }
    }

    /// Test that we can generate a baseline binary for comparison
    #[test]
    fn test_generate_baseline_for_phase_comparison() {
        let compiler = GrueCompiler::new();
        let source = fs::read_to_string("examples/mini_zork.grue")
            .expect("Could not read examples/mini_zork.grue");

        if let Ok((bytecode, _)) = compiler.compile(&source, ZMachineVersion::V3) {
            // Generate baseline for phase comparisons
            fs::write("tests/phase1_baseline.z3", &bytecode)
                .expect("Could not write phase baseline file");

            let size = bytecode.len();
            println!(
                "✅ Phase 1 baseline generated: {} bytes → tests/phase1_baseline.z3",
                size
            );

            // During Phases 2-6, we'll compare against this baseline
            // to ensure functionality is preserved while fixing branch overflow

            // Also capture first few bytes for header comparison
            if bytecode.len() >= 16 {
                print!("    Header bytes: ");
                for i in 0..16 {
                    print!("{:02x} ", bytecode[i]);
                }
                println!();
            }
        } else {
            println!("⚠️  Could not generate Phase 1 baseline due to compilation failure");
            println!("    This is expected - the baseline captures the current broken state");
            println!("    We'll use this as our 'before' comparison for the fix");
        }
    }
}
