use gruesome::display_manager::{create_display, DisplayMode};
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
    
    // Create a v4 display
    let mut display = create_display(4, DisplayMode::Terminal)?;
    
    println!("Testing AMFV-style status line...\n");
    
    // Clear and set up upper window
    display.erase_window(-1)?;
    display.split_window(1)?;  // Single line status
    
    // Select upper window and draw status line
    display.set_window(1)?;
    display.set_text_style(1)?;  // Reverse video
    
    // Test different cursor positions and text
    display.set_cursor(1, 1)?;
    display.print("Time: 7:07pm")?;
    
    display.set_cursor(1, 20)?;
    display.print("Location: PRISM Project Control")?;
    
    display.set_cursor(1, 60)?;
    display.print("Mode: Communications Mode")?;
    
    display.set_text_style(0)?;
    display.set_window(0)?;
    
    // Print some content in lower window
    display.print("\nTest content in lower window\n")?;
    display.force_refresh()?;
    
    println!("\nPress Enter to test overlapping text...");
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    
    // Now test overlapping text
    display.set_window(1)?;
    display.set_text_style(1)?;
    
    // Write at end of line
    display.set_cursor(1, 70)?;
    display.print("3/16/2031")?;
    
    display.set_text_style(0)?;
    display.set_window(0)?;
    display.force_refresh()?;
    
    println!("\nPress Enter to exit...");
    buffer.clear();
    io::stdin().read_line(&mut buffer)?;
    
    Ok(())
}