use gruesome::display_manager::{create_display, DisplayMode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    // Create a v4 display
    let mut display = create_display(4, DisplayMode::Terminal)?;

    println!("Testing v4 window selection...\n");

    // Simulate AMFV's opening sequence:

    // 1. Clear everything with erase_window(-1)
    println!("Step 1: erase_window(-1)");
    display.erase_window(-1)?;

    // 2. Create upper window
    println!("\nStep 2: split_window(3)");
    display.split_window(3)?;

    // 3. Select upper window
    println!("\nStep 3: set_window(1)");
    display.set_window(1)?;

    // 4. Print "PART I" to upper window
    println!("\nStep 4: Print to upper window");
    display.set_cursor(2, 30)?;
    display.set_text_style(1)?; // Reverse video
    display.print("* PART I *")?;
    display.set_text_style(0)?; // Reset

    // 5. Switch back to lower window
    println!("\nStep 5: set_window(0)");
    display.set_window(0)?;

    // 6. Print quote to lower window
    println!("\nStep 6: Print quote to lower window");
    display.print("\n\n\n\n\n"); // Move down a bit
    display.print("\"Tomorrow never yet\n")?;
    display.print("On any human being rose or set.\"\n")?;
    display.print("-- William Marsden\n")?;

    display.force_refresh()?;

    println!("\n\nDone. Check if PART I appears in upper window and quote in lower window.");

    Ok(())
}
