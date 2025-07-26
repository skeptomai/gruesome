use gruesome::display_manager::{create_display, DisplayMode};
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    println!("Testing exact terminal operations...\n");

    // Create a v4 display
    let mut display = create_display(4, DisplayMode::Terminal)?;

    // Step 1: Clear screen
    println!("Step 1: Clear screen");
    display.erase_window(-1)?;
    thread::sleep(Duration::from_millis(500));

    // Step 2: Split window
    println!("Step 2: Split window (3 lines)");
    display.split_window(3)?;
    thread::sleep(Duration::from_millis(500));

    // Step 3: Write to upper window
    println!("Step 3: Write to upper window");
    display.set_window(1)?;
    display.set_cursor(2, 35)?;
    display.set_text_style(1)?;
    display.print("* PART I *")?;
    display.set_text_style(0)?;
    thread::sleep(Duration::from_millis(500));

    // Step 4: Switch to lower window (should trigger refresh)
    println!("Step 4: Switch to lower window");
    display.set_window(0)?;
    thread::sleep(Duration::from_millis(1000));

    println!("\nUpper window should show '* PART I *' now");
    println!("Press Enter to continue to next step...");
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;

    // Step 5: Clear lower window
    println!("Step 5: Clear lower window");
    display.erase_window(0)?;
    thread::sleep(Duration::from_millis(500));

    println!("\nUpper window should STILL show '* PART I *'");
    println!("Press Enter to continue...");
    buffer.clear();
    io::stdin().read_line(&mut buffer)?;

    // Step 6: Print to lower window
    println!("Step 6: Print quote");
    display.print("\n\n\n\n\n\n\n\n")?;
    display.print("      \"Tomorrow never yet\n")?;
    display.print("      On any human being rose or set.\"\n")?;
    display.print("                           -- William Marsden\n")?;

    println!("\nBoth windows should be visible now");
    println!("Press Enter to exit...");
    buffer.clear();
    io::stdin().read_line(&mut buffer)?;

    Ok(())
}
