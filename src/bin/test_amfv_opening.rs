use gruesome::display_manager::{create_display, DisplayMode};
use std::io::{self};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    // Create a v4 display
    let mut display = create_display(4, DisplayMode::Terminal)?;

    println!("Simulating AMFV opening sequence...\n");

    // Step 1: Clear everything
    display.erase_window(-1)?;

    // Step 2: Split window for header
    display.split_window(3)?;

    // Step 3: Select upper window
    display.set_window(1)?;

    // Step 4: Position cursor and print PART I
    display.set_cursor(2, 35)?; // Center position
    display.set_text_style(1)?; // Reverse video
    display.print("* PART I *")?;
    display.set_text_style(0)?; // Reset style

    // Step 5: Switch to lower window
    display.set_window(0)?;

    // Step 6: Clear lower window (AMFV might do this)
    display.erase_window(0)?;

    // Step 7: Print quote
    let _ = display.print("\n\n\n\n\n\n\n\n\n\n"); // Move down
    display.print("      \"Tomorrow never yet\n")?;
    display.print("      On any human being rose or set.\"\n")?;
    display.print("                           -- William Marsden\n")?;

    // Force refresh everything
    display.force_refresh()?;

    println!("\n\nPress Enter to continue...");
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;

    Ok(())
}
