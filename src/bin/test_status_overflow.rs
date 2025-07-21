use gruesome::display_manager::{create_display, DisplayMode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
    
    // Create a v4 display
    let mut display = create_display(4, DisplayMode::Terminal)?;
    
    println!("Testing status line text overlap...\n");
    
    // Clear and set up
    display.clear_screen()?;
    display.split_window(1)?;
    
    // Write overlapping text
    display.set_window(1)?;
    display.set_text_style(1)?;
    
    // Write at column 70 - should be near end of 80 column terminal
    display.set_cursor(1, 70)?;
    display.print("3/16/2031")?;  // This is 9 chars, goes to column 79
    
    // Now write at column 0 - this might be the "ime:" issue
    display.set_cursor(1, 1)?;
    display.print("Time: 7:07pm")?;
    
    // Force refresh
    display.set_text_style(0)?;
    display.set_window(0)?;
    display.force_refresh()?;
    
    println!("\nCheck if text at end overwrites beginning");
    
    Ok(())
}