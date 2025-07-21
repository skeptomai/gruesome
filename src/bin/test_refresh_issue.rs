use gruesome::display_manager::{create_display, DisplayMode};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
    
    // Create a v4 display
    let mut display = create_display(4, DisplayMode::Terminal)?;
    
    println!("Testing status line refresh during print...\n");
    
    // Setup
    display.clear_screen()?;
    display.split_window(1)?;
    
    // Select upper window and start printing
    display.set_window(1)?;
    display.set_text_style(1)?;
    
    // Print "T" at position 1,1
    display.set_cursor(1, 1)?;
    display.print("T")?;
    
    // Simulate a refresh happening mid-word
    display.set_window(0)?;  // This might trigger refresh
    thread::sleep(Duration::from_millis(50));
    
    // Switch back and continue printing
    display.set_window(1)?;
    display.print("ime: 7:07pm")?;
    
    // Final switch to lower window
    display.set_text_style(0)?;
    display.set_window(0)?;
    
    println!("\nCheck if 'T' is missing from 'Time:' in status line");
    
    Ok(())
}