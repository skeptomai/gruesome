use gruesome::display_manager::{create_display, DisplayMode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    // Create a v4 display
    let mut display = create_display(4, DisplayMode::Terminal)?;

    println!("Testing AMFV PART I rendering issue...\n");

    // Try to reproduce the exact issue:
    // The output shows "Tomorrow never yetI *" which suggests:
    // 1. "Tomorrow never yet" is printed
    // 2. Then "I *" appears in the middle

    // Theory: The game might be doing something like:
    display.clear_screen()?;

    // Print the beginning of the quote
    display.print("\"Tomorrow never yet")?;

    // Now try to position cursor elsewhere for "PART I"
    // But maybe our cursor positioning is wrong?
    display.set_cursor(1, 35)?; // Try positioning in the middle
    display.set_text_style(1)?; // Reverse video
    display.print("I *")?; // This might be part of "* PART I *"
    display.set_text_style(0)?; // Reset

    // Continue with the quote
    display.print("\n")?;
    display.print("On any human being rose or set.\"")?;

    display.force_refresh()?;

    println!("\n\nThis reproduces the issue where 'I *' appears in the middle of the text!");
    println!("The problem is likely that cursor positioning doesn't work as expected");
    println!("when printing in the middle of already-printed text.");

    Ok(())
}
