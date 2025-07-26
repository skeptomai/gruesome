use gruesome::display_manager::{create_display, DisplayMode};
use std::io::{self};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    // Create a v4 display
    let mut display = create_display(4, DisplayMode::Terminal)?;

    println!("=== Testing Selective Status Line Omissions ===");
    println!("We'll try omitting different parts to isolate the interference...\n");

    // Setup basic window
    display.clear_screen()?;
    display.split_window(2)?;
    display.set_window(1)?;

    // Step 1: Try ONLY printing "Time:" without anything else
    println!("Test 1: Print ONLY 'Time:' at position (1,60)");
    display.set_text_style(1)?;
    display.set_cursor(1, 60)?;
    display.print("Time:")?;
    display.set_text_style(0)?;
    display.set_window(0)?;

    println!("Does 'Time:' show correctly? Press Enter...");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    // Step 2: Try printing "Mode:" then "Time:"
    display.set_window(1)?;
    display.erase_window(1)?; // Clear upper window
    display.set_text_style(1)?;

    println!("\nTest 2: Print 'Mode:' then 'Time:'");
    display.set_cursor(1, 2)?;
    display.print("Mode:")?;
    display.set_cursor(1, 60)?;
    display.print("Time:")?;
    display.set_text_style(0)?;
    display.set_window(0)?;

    println!("Does 'Time:' still show correctly? Press Enter...");
    input.clear();
    io::stdin().read_line(&mut input)?;

    // Step 3: Add "Location:" on line 2
    display.set_window(1)?;
    display.set_text_style(1)?;

    println!("\nTest 3: Add 'Location:' on line 2");
    display.set_cursor(2, 2)?;
    display.print("Location:")?;
    display.set_text_style(0)?;
    display.set_window(0)?;

    println!("Does 'Time:' still show correctly? Press Enter...");
    input.clear();
    io::stdin().read_line(&mut input)?;

    // Step 4: Add "Date:" - this might interfere due to proximity
    display.set_window(1)?;
    display.set_text_style(1)?;

    println!("\nTest 4: Add 'Date:' at position (2,60) - same column as Time:");
    display.set_cursor(2, 60)?;
    display.print("Date:")?;
    display.set_text_style(0)?;
    display.set_window(0)?;

    println!("Does 'Time:' STILL show correctly, or is it now 'ime:'? Press Enter...");
    input.clear();
    io::stdin().read_line(&mut input)?;

    // Step 5: Test exact positioning - what if we print at 59 instead of 60?
    display.set_window(1)?;
    display.erase_window(1)?;
    display.set_text_style(1)?;

    println!("\nTest 5: Print 'Time:' at position (1,59) instead of (1,60)");
    display.set_cursor(1, 59)?;
    display.print("Time:")?;
    display.set_text_style(0)?;
    display.set_window(0)?;

    println!("How does this position look? Press Enter to exit...");
    input.clear();
    io::stdin().read_line(&mut input)?;

    Ok(())
}
