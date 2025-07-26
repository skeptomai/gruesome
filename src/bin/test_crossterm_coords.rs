/// Simple test to verify crossterm coordinate system
/// Places characters at the four corners of the terminal to verify positioning

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = io::stdout();
    
    println!("Testing crossterm coordinate system...");
    
    // Get terminal size
    let (crossterm_width, crossterm_height) = terminal::size()?;
    
    // Try stty as well
    let stty_size = std::process::Command::new("stty")
        .arg("size")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let size_str = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = size_str.split_whitespace().collect();
                if parts.len() == 2 {
                    if let (Ok(height), Ok(width)) = (parts[0].parse::<u16>(), parts[1].parse::<u16>()) {
                        return Some((width, height));
                    }
                }
            }
            None
        });
    
    println!("Crossterm reports: {}x{}", crossterm_width, crossterm_height);
    if let Some((stty_width, stty_height)) = stty_size {
        println!("stty reports: {}x{}", stty_width, stty_height);
        let offset = crossterm_height.saturating_sub(stty_height);
        println!("Offset would be: {}", offset);
    } else {
        println!("stty: failed to get size");
    }
    println!("Press any key to start coordinate test...");
    
    // Wait for keypress
    let _ = std::io::stdin().read_line(&mut String::new());
    
    // Initialize terminal
    execute!(
        stdout,
        Hide,
        Clear(ClearType::All)
    )?;
    
    // Enable raw mode
    terminal::enable_raw_mode()?;
    
    // Test coordinates - try both raw and offset positioning
    let offset = if let Some((_, stty_height)) = stty_size {
        crossterm_height.saturating_sub(stty_height)
    } else {
        0
    };
    
    // Test 1: Raw (0, 0) - might be above visible area
    queue!(stdout, MoveTo(0, 0))?;
    queue!(stdout, SetBackgroundColor(Color::Red), SetForegroundColor(Color::White))?;
    queue!(stdout, Print("RAW_TL"))?;
    queue!(stdout, ResetColor)?;
    
    // Test 2: With offset - should be visible
    queue!(stdout, MoveTo(0, offset))?;
    queue!(stdout, SetBackgroundColor(Color::Yellow), SetForegroundColor(Color::Black))?;
    queue!(stdout, Print("OFFSET_TL"))?;
    queue!(stdout, ResetColor)?;
    
    // Use actual terminal size for positioning
    let (width, height) = stty_size.unwrap_or((crossterm_width, crossterm_height));
    
    // Top-right (width-2, offset) - should be visible  
    queue!(stdout, MoveTo(width.saturating_sub(2), offset))?;
    queue!(stdout, SetBackgroundColor(Color::Green), SetForegroundColor(Color::White))?;
    queue!(stdout, Print("TR"))?;
    queue!(stdout, ResetColor)?;
    
    // Bottom-left (0, height-1+offset) - should be visible
    queue!(stdout, MoveTo(0, height.saturating_sub(1) + offset))?;
    queue!(stdout, SetBackgroundColor(Color::Blue), SetForegroundColor(Color::White))?;
    queue!(stdout, Print("BL"))?;
    queue!(stdout, ResetColor)?;
    
    // Bottom-right (width-2, height-1+offset) - should be visible
    queue!(stdout, MoveTo(width.saturating_sub(2), height.saturating_sub(1) + offset))?;
    queue!(stdout, SetBackgroundColor(Color::Magenta), SetForegroundColor(Color::White))?;
    queue!(stdout, Print("BR"))?;
    queue!(stdout, ResetColor)?;
    
    // Center message
    let center_x = width / 2;
    let center_y = height / 2;
    queue!(stdout, MoveTo(center_x.saturating_sub(10), center_y + offset))?;
    queue!(stdout, SetBackgroundColor(Color::Magenta), SetForegroundColor(Color::White))?;
    queue!(stdout, Print(format!("CENTER - Size: {}x{}", width, height)))?;
    queue!(stdout, ResetColor)?;
    
    // Status line at row 1
    queue!(stdout, MoveTo(0, 1))?;
    queue!(stdout, SetBackgroundColor(Color::White), SetForegroundColor(Color::Black))?;
    queue!(stdout, Print("Status line test - row 1"))?;
    for _ in 20..width {
        queue!(stdout, Print(" "))?;
    }
    queue!(stdout, ResetColor)?;
    
    // Test upper window area (rows 0-6 like AMFV uses)
    for row in 2..7 {
        queue!(stdout, MoveTo(0, row))?;
        queue!(stdout, SetBackgroundColor(Color::Cyan), SetForegroundColor(Color::Black))?;
        let text = format!("Upper window row {}", row);
        queue!(stdout, Print(text))?;
        
        // Fill rest of line with spaces
        for _ in (format!("Upper window row {}", row).len() as u16)..width {
            queue!(stdout, Print(" "))?;
        }
        queue!(stdout, ResetColor)?;
    }
    
    stdout.flush()?;
    
    // Move to bottom of screen for output messages
    queue!(stdout, MoveTo(0, height.saturating_sub(8)))?;
    queue!(stdout, ResetColor)?;
    stdout.flush()?;
    
    // Cleanup
    execute!(stdout, Show, ResetColor)?;
    terminal::disable_raw_mode()?;
    
    println!("\n\nCoordinate test results:");
    println!("You should see:");
    println!("- Red 'TL' at top-left corner");
    println!("- Green 'TR' at top-right corner"); 
    println!("- Blue 'BL' at bottom-left corner");
    println!("- Yellow 'BR' at bottom-right corner");
    println!("- Magenta center message");
    println!("- White status line at row 1");
    println!("- Cyan upper window rows 2-6");
    println!("\nPress any key to exit...");
    
    // Wait for keypress
    let _ = std::io::stdin().read_line(&mut String::new());
    
    // Final cleanup
    execute!(io::stdout(), Clear(ClearType::All))?;
    
    Ok(())
}