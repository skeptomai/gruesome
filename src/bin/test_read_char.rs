use env_logger;
use gruesome::timed_input::TimedInput;
use log::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("=== Testing read_char Implementation ===\n");

    let mut timed_input = TimedInput::new();

    // Test 1: Basic character read (no timeout)
    println!("Test 1: Press any key (no timeout):");

    match timed_input.read_char_with_timeout_callback::<fn() -> Result<bool, String>>(0, 0, None) {
        Ok((ch, terminated)) => {
            println!("You pressed: '{}' (0x{:02x})", ch, ch as u8);
            println!("Was terminated: {}", terminated);
        }
        Err(e) => println!("Error: {}", e),
    }

    // Test 2: Character read with timeout
    println!("\nTest 2: Press a key within 3 seconds:");

    let mut timer_fired = false;
    let callback = || -> Result<bool, String> {
        println!("\n*** Timer fired! ***");
        timer_fired = true;
        Ok(false) // Continue reading
    };

    match timed_input.read_char_with_timeout_callback(30, 0x1234, Some(callback)) {
        Ok((ch, terminated)) => {
            if terminated {
                println!("\nTimeout! No character received");
            } else {
                println!("You pressed: '{}' (0x{:02x})", ch, ch as u8);
            }
            if timer_fired {
                println!("Timer fired during input");
            }
        }
        Err(e) => println!("Error: {}", e),
    }

    // Test 3: Character read with terminating timeout
    println!("\nTest 3: Wait 2 seconds for timeout termination:");

    let callback2 = || -> Result<bool, String> {
        println!("\n*** Timer expired - terminating input ***");
        Ok(true) // Terminate input
    };

    match timed_input.read_char_with_timeout_callback(20, 0x5678, Some(callback2)) {
        Ok((ch, terminated)) => {
            if terminated {
                println!("Input terminated by timer (returned null char)");
            } else {
                println!("You pressed: '{}' (0x{:02x})", ch, ch as u8);
            }
        }
        Err(e) => println!("Error: {}", e),
    }

    println!("\nTests complete!");
    Ok(())
}
