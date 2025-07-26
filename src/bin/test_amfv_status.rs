use gruesome::display_manager::{create_display, DisplayMode};
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    // Create a v4 display
    let mut display = create_display(4, DisplayMode::Terminal)?;

    println!("=== Testing EXACT AMFV Status Line Pattern ===");
    println!("Reproducing sequence from disassembly at 19a3b...\n");

    // Clear and setup like AMFV does
    display.clear_screen()?;
    display.split_window(2)?; // AMFV uses 2-line upper window
    display.set_window(1)?; // Select upper window

    // Step 1: Clear upper window lines (routine 19a8c called twice with exact buffering)
    println!("Step 1: Clearing upper window lines with exact 19a8c pattern...");

    // First call: 19a8c(#01) - clear line 1
    display.set_cursor(1, 1)?; // 19a9b: SET_CURSOR L00,#01 (L00=1)
    display.set_text_style(1)?; // 19a9f: SET_TEXT_STYLE REVERSE
    display.set_buffer_mode(true)?; // 19aa2: BUFFER_MODE #01 - TURN ON BUFFERING
    display
        .print("                                                                               ")?; // 19aa5: PRINT long spaces
    display.set_buffer_mode(false)?; // 19ac2: BUFFER_MODE #00 - TURN OFF BUFFERING (flushes)
    display.set_text_style(0)?; // 19ad1: SET_TEXT_STYLE ROMAN

    // Second call: 19a8c(#02) - clear line 2
    display.set_cursor(2, 1)?; // 19a9b: SET_CURSOR L00,#01 (L00=2)
    display.set_text_style(1)?; // 19a9f: SET_TEXT_STYLE REVERSE
    display.set_buffer_mode(true)?; // 19aa2: BUFFER_MODE #01 - TURN ON BUFFERING
    display
        .print("                                                                               ")?; // 19aa5: PRINT long spaces
    display.set_buffer_mode(false)?; // 19ac2: BUFFER_MODE #00 - TURN OFF BUFFERING (flushes)
    display.set_text_style(0)?; // 19ad1: SET_TEXT_STYLE ROMAN

    display.set_text_style(0)?; // Reset style

    // Step 2: Print labels exactly as AMFV does with EXACT buffering pattern
    println!("Step 2: Printing status line labels...");

    // EXACT sequence from disassembly:
    display.set_buffer_mode(false)?; // 19a3b: BUFFER_MODE #00 - TURN OFF BUFFERING

    display.set_text_style(1)?; // 19a4e: SET_TEXT_STYLE REVERSE

    // Print "Mode:" at (1,2) - exact position from disassembly
    display.set_cursor(1, 2)?; // 19a51: SET_CURSOR #01,#02
    display.print("Mode:")?; // 19a55: PRINT "Mode:"

    // Print "Time:" at (1,60) - THIS IS WHERE THE T GOES MISSING!
    display.set_cursor(1, 60)?; // 19a5c: SET_CURSOR #01,#3c (60 decimal)
    println!("DEBUG: About to print 'Time:' at cursor (1,60) with buffering OFF...");
    io::stdout().flush()?;
    display.print("Time:")?; // 19a60: PRINT "Time:"
    println!("DEBUG: Finished printing 'Time:'");

    // Print "Location:" at (2,2)
    display.set_cursor(2, 2)?; // 19a67: SET_CURSOR #02,#02
    display.print("Location:")?; // 19a6b: PRINT "Location:"

    // Print "Date:" at (2,60)
    display.set_cursor(2, 60)?; // 19a74: SET_CURSOR #02,#3c
    display.print("Date:")?; // 19a78: PRINT "Date:"

    display.set_text_style(0)?; // 19a7f: SET_TEXT_STYLE ROMAN
    display.set_buffer_mode(true)?; // 19a82: BUFFER_MODE #01 - TURN ON BUFFERING
    display.set_window(0)?; // 19a85: SET_WINDOW #00

    println!("\n=== Status line rendering complete ===");
    println!("Check if 'Time:' shows as 'ime:' (missing T)");
    println!("Also check for overlapping text at line endings");
    println!("Press Enter to continue...");

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    // Test additional overlapping scenario
    println!("\n=== Testing overlapping text issue ===");
    display.set_window(1)?;
    display.set_text_style(1)?;
    display.set_cursor(1, 77)?; // Near end of line
    display.print("Co3/16/2031")?; // This might cause overlapping
    display.set_window(0)?;

    println!("Check for overlapping ':Co3/16/2031' text");
    println!("Press Enter to exit...");
    input.clear();
    io::stdin().read_line(&mut input)?;

    Ok(())
}
