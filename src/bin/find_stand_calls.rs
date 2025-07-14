use std::fs::File;
use std::io::prelude::*;

fn main() -> std::io::Result<()> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    // Look for calls to routine 0x86ca (STAND handler)
    // In packed form: 0x86ca / 2 = 0x4365
    
    println!("Searching for calls to STAND routine at 0x86ca (packed: 0x4365)...\n");
    
    // Search for CALL instructions with this address
    for i in 0..game_data.len()-4 {
        let byte = game_data[i];
        
        // Check for various CALL instruction forms
        // call_2s (2OP:19) - Long form
        if (byte >> 6) < 2 && (byte & 0x1F) == 0x19 {
            // Check if next bytes could be 0x4365
            if i + 3 < game_data.len() {
                let op1 = ((game_data[i+1] as u16) << 8) | (game_data[i+2] as u16);
                if op1 == 0x4365 {
                    println!("Found call_2s to STAND at 0x{:05x}", i);
                }
            }
        }
        
        // call (VAR:0) - Variable form
        if byte == 0xE0 || byte == 0xC0 {
            // Check operand types byte
            if i + 1 < game_data.len() {
                let types = game_data[i + 1];
                // First operand should be large constant
                if (types >> 6) == 0 {
                    if i + 4 < game_data.len() {
                        let addr = ((game_data[i+2] as u16) << 8) | (game_data[i+3] as u16);
                        if addr == 0x4365 {
                            println!("Found call to STAND at 0x{:05x}", i);
                        }
                    }
                }
            }
        }
    }
    
    // Also look for the action dispatcher - it might be jumping to wrong routine
    println!("\nLooking for action dispatch tables near common addresses...");
    
    // Search for patterns that might be action tables (sequences of routine addresses)
    for i in 0x1000..0x8000 {
        if i + 2 < game_data.len() {
            let addr = ((game_data[i] as u16) << 8) | (game_data[i+1] as u16);
            if addr == 0x4365 {
                println!("Found reference to STAND routine (0x4365) at 0x{:05x}", i);
                
                // Show some context
                print!("  Context: ");
                for j in (i.saturating_sub(4))..=(i+6).min(game_data.len()-1) {
                    print!("{:02x} ", game_data[j]);
                }
                println!();
            }
        }
    }

    Ok(())
}