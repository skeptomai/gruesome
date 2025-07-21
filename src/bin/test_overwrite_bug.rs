use gruesome::display_manager::{create_display, DisplayMode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
    
    // Create a v4 display
    let mut display = create_display(4, DisplayMode::Terminal)?;
    
    println!("=== Testing Character Overwrite Bug ===");
    println!("This reproduces the exact AMFV sequence that caused 'Time:' -> 'ime:'");
    
    // Setup
    display.clear_screen()?;
    display.split_window(2)?;
    display.set_window(1)?;
    display.set_text_style(1)?;
    
    // Step 1: Print "Time:" at position (1,60) like AMFV does
    println!("\nStep 1: Print 'Time:' at position (1,60)");
    display.set_cursor(1, 60)?;
    display.print("Time:")?;
    display.set_window(0)?;
    
    println!("Line should show: 'Time:' at column 60");
    println!("Press Enter to continue...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    // Step 2: Now print a space at position (1,60) - this was overwriting the 'T'
    println!("\nStep 2: Print a space ' ' at position (1,60) - this used to overwrite 'T'");
    display.set_window(1)?;
    display.set_cursor(1, 60)?;  // Position 60 = column 59 in 0-based indexing
    display.print(" ")?;  // This should NOT overwrite existing text
    display.set_window(0)?;
    
    println!("Line should STILL show: 'Time:' (T should not be overwritten)");
    println!("If you see 'ime:' then the bug is not fixed");
    println!("Press Enter to continue...");
    input.clear();
    std::io::stdin().read_line(&mut input)?;
    
    // Step 3: Test printing time digits after the colon
    println!("\nStep 3: Print time digits '7:07pm' starting at position (1,67)");
    display.set_window(1)?;
    display.set_cursor(1, 67)?;  // After "Time: " 
    display.print("7:07pm")?;
    display.set_window(0)?;
    
    println!("Final result should show: 'Time: 7:07pm' (complete and correct)");
    println!("Press Enter to exit...");
    input.clear();
    std::io::stdin().read_line(&mut input)?;
    
    Ok(())
}