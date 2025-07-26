/// Test program for the non-blocking timed input system.
///
/// This test validates the core input handling that enables Z-Machine timer
/// interrupts during SREAD operations. The timed input system is essential for:
/// - Lantern countdown timers in Zork I
/// - Match and candle timers
/// - Real-time game mechanics in Border Zone and other v4+ games
/// - Proper input echo behavior per Z-Machine specification
///
/// Tests both basic input and timed input with callbacks to ensure the
/// system works correctly across different terminal environments.
use gruesome::timed_input::TimedInput;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("=== Testing Non-Blocking Timed Input ===\n");

    let mut timed_input = TimedInput::new();

    // Test 1: Basic input (no timer)
    println!("Test 1: Basic input (no timer)");
    println!("Type something and press Enter:");
    println!("- Character-by-character echo shows non-blocking mode");
    println!("- Arrow keys and backspace work\n");

    match timed_input.read_line_basic() {
        Ok(input) => println!("\nYou typed: '{input}'"),
        Err(e) => println!("\nError: {e}"),
    }

    // Test 2: Timed input with 5 second timeout
    println!("\nTest 2: Timed input with 5 second timeout");
    println!("Type something within 5 seconds:");

    // Create a simple timer callback that just logs
    let timer_fired = std::cell::RefCell::new(false);
    let callback = || -> Result<bool, String> {
        println!("\n*** TIMER FIRED! ***");
        *timer_fired.borrow_mut() = true;
        Ok(false) // Don't terminate input
    };

    match timed_input.read_line_with_timer(50, 0x1234, Some(callback)) {
        // 50 tenths = 5 seconds
        Ok((input, terminated)) => {
            println!("\nYou typed: '{input}'");
            if terminated {
                println!("Input was TERMINATED by timer!");
            } else {
                println!("Input completed before timeout");
            }
        }
        Err(e) => println!("\nError: {e}"),
    }

    // Test 3: Very short timeout
    println!("\nTest 3: Very short timeout (1 second)");
    println!("Try to type something (it will timeout):");

    let callback2 = || -> Result<bool, String> {
        println!("\n*** TIMER EXPIRED - TERMINATING INPUT ***");
        Ok(true) // Terminate input
    };

    match timed_input.read_line_with_timer(10, 0x5678, Some(callback2)) {
        // 10 tenths = 1 second
        Ok((input, terminated)) => {
            if terminated {
                println!("\nTimed out! Partial input: '{input}'");
            } else {
                println!("\nFast typing! You entered: '{input}'");
            }
        }
        Err(e) => println!("\nError: {e}"),
    }

    println!("\nTests complete!");
    Ok(())
}
