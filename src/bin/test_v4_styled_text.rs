use gruesome::display_manager::create_display;
use gruesome::display_trait::ZMachineDisplay;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a v4 display
    let mut display = create_display(4, gruesome::display_manager::DisplayMode::Terminal)?;
    
    println!("Testing v4 styled text rendering...\n");
    
    // Simulate what AMFV might be doing:
    
    // 1. Clear screen
    display.clear_screen()?;
    
    // 2. Position cursor (let's say at line 5, column 20)
    display.set_cursor(5, 20)?;
    
    // 3. Set reverse video
    display.set_text_style(1)?;
    
    // 4. Print "* PART I *"
    display.print("* PART I *")?;
    
    // 5. Reset style
    display.set_text_style(0)?;
    
    // 6. Move cursor down a few lines
    display.set_cursor(10, 1)?;
    
    // 7. Print the quote
    display.print("\"Tomorrow never yet")?;
    display.print("\n")?;
    display.print("On any human being rose or set.\"")?;
    display.print("\n")?;
    display.print("-- William Marsden")?;
    
    // 8. Move cursor for prompt
    display.set_cursor(20, 1)?;
    display.print("[Hit any key to continue.]")?;
    
    display.force_refresh()?;
    
    println!("\n\nTest complete. Check if styled text appears correctly.");
    
    Ok(())
}