use std::fs::File;
use std::io::prelude::*;

fn main() -> std::io::Result<()> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Searching for 'already standing' text in game data...\n");
    
    // The text would be Z-encoded. Let's search for PRINT_RET instructions
    // that might contain this text
    
    // First, let's look for the string pattern
    let search_patterns = ["already", "standing", "think"];
    
    for pattern in &search_patterns {
        println!("Searching for '{}':", pattern);
        let bytes = pattern.as_bytes();
        
        for i in 0..game_data.len() - bytes.len() {
            let mut found = true;
            for j in 0..bytes.len() {
                // Z-Machine uses uppercase
                if game_data[i + j] != bytes[j].to_ascii_uppercase() {
                    found = false;
                    break;
                }
            }
            
            if found {
                println!("  Found at 0x{:05x}", i);
                
                // Show context
                print!("  Context: ");
                for k in i.saturating_sub(10)..=(i + bytes.len() + 10).min(game_data.len() - 1) {
                    if game_data[k] >= 32 && game_data[k] <= 126 {
                        print!("{}", game_data[k] as char);
                    } else {
                        print!(".");
                    }
                }
                println!();
            }
        }
    }
    
    // Now let's look specifically at address 0x86dd where the PRINT_RET should be
    println!("\nChecking address 0x86dd (where PRINT_RET should be):");
    if 0x86dd < game_data.len() {
        let opcode = game_data[0x86dd];
        println!("  Opcode byte: 0x{:02x} (binary: {:08b})", opcode, opcode);
        
        // PRINT_RET is 0xB3 (10110011)
        if opcode == 0xB3 {
            println!("  This is PRINT_RET!");
            
            // Decode the Z-string that follows
            println!("  Next bytes:");
            for i in 0..20 {
                if 0x86de + i < game_data.len() {
                    print!(" {:02x}", game_data[0x86de + i]);
                }
            }
            println!();
        }
    }
    
    // Also check what's calling routine at 0x86ca
    println!("\nLooking for calls to 0x86ca (packed as 0x4365):");
    
    for i in 0..game_data.len() - 2 {
        let word = ((game_data[i] as u16) << 8) | (game_data[i + 1] as u16);
        if word == 0x4365 {
            println!("  Found reference at 0x{:05x}", i);
            
            // Check what instruction this is part of
            if i >= 2 {
                println!("    Previous bytes: {:02x} {:02x}", game_data[i-2], game_data[i-1]);
            }
        }
    }

    Ok(())
}