use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Searching for LOOK action code...\n");
    
    // In the comparison at 0x08cc7, we have "jl #0005, Vd1"
    // This suggests action codes > 5 are valid
    // Let's look for patterns that might indicate action codes
    
    // First, let's check what values are compared with 5
    println!("The comparison at 0x08cc7 is checking if 5 < Vd1");
    println!("This suggests:");
    println!("- Action codes 0-5 are special/invalid");
    println!("- Valid action codes are > 5");
    println!("\nLOOK is typically one of the first valid actions, so it might be 6, 7, 8...");
    
    // Let's also check if there's a dispatch table
    // Action dispatch tables often start with a series of routine addresses
    
    // Look for the routine at 0x577c which was mentioned as V-LOOK
    println!("\nChecking routine at 0x577c (V-LOOK?):");
    let v_look_packed = 0x577c / 2; // 0x2bbe
    println!("Packed address for 0x577c: 0x{:04x}", v_look_packed);
    
    // Search for references to this packed address
    let mut found_at = Vec::new();
    for i in 0..game_data.len()-2 {
        let word = ((game_data[i] as u16) << 8) | (game_data[i+1] as u16);
        if word == v_look_packed {
            found_at.push(i);
        }
    }
    
    println!("\nFound packed address 0x{:04x} at:", v_look_packed);
    for addr in &found_at {
        println!("  0x{:05x}", addr);
        
        // Check if this looks like part of a table
        if *addr >= 10 && *addr + 10 < game_data.len() {
            println!("    Context (possible table):");
            for offset in 0..5 {
                let idx = addr - (offset * 2);
                if idx + 1 < game_data.len() {
                    let entry = ((game_data[idx] as u16) << 8) | (game_data[idx+1] as u16);
                    let unpacked = (entry as u32) * 2;
                    println!("      [{:+3}] 0x{:05x}: 0x{:04x} -> 0x{:05x}", 
                             -(offset as i32) * 2, idx, entry, unpacked);
                }
            }
            for offset in 1..5 {
                let idx = addr + (offset * 2);
                if idx + 1 < game_data.len() {
                    let entry = ((game_data[idx] as u16) << 8) | (game_data[idx+1] as u16);
                    let unpacked = (entry as u32) * 2;
                    println!("      [{:+3}] 0x{:05x}: 0x{:04x} -> 0x{:05x}", 
                             offset * 2, idx, entry, unpacked);
                }
            }
        }
    }
    
    // Also check if there's a pattern like "store #XX to Vd1" 
    // for various action codes
    println!("\n\nLooking for instructions that store to Vd1 (0xd1):");
    println!("These might reveal action codes...");
    
    // Look for patterns like:
    // - store #XX -> Vd1 (0E instruction: store small constant)
    // - store Vxx -> Vd1 (Variable assignments)
    
    // First, let's look for all store instructions to 0xd1
    for i in 0..game_data.len()-3 {
        let byte = game_data[i];
        
        // Check for store instruction patterns
        // 0E: store (small constant #, variable)
        if byte == 0x0E && i + 2 < game_data.len() && game_data[i+2] == 0xd1 {
            let constant = game_data[i+1];
            println!("\n  Found: store #{} -> Vd1 at 0x{:05x}", constant, i);
            
            // Show context
            if i >= 5 && i + 5 < game_data.len() {
                print!("    Context: ");
                for j in (i-5)..=(i+5) {
                    print!("{:02x} ", game_data[j]);
                }
                println!();
            }
        }
        
        // Check for variable form store instructions that might store to d1
        if (byte & 0xC0) == 0xC0 { // Variable form
            // Could be various opcodes that store to variables
            // Let's check a few bytes ahead for 0xd1
            for offset in 2..6 {
                if i + offset < game_data.len() && game_data[i + offset] == 0xd1 {
                    println!("\n  Possible store to Vd1 at 0x{:05x} (offset {})", i, offset);
                    print!("    Bytes: ");
                    for j in i..(i+8).min(game_data.len()) {
                        print!("{:02x} ", game_data[j]);
                    }
                    println!();
                }
            }
        }
    }
    
    // Let's also look for common action codes in the game
    println!("\n\nSearching for common action codes in comparisons:");
    for i in 0..game_data.len()-3 {
        let byte = game_data[i];
        
        // Look for jl (less than) instructions comparing with small constants
        if byte == 0x02 && i + 2 < game_data.len() {
            let const_val = game_data[i+1];
            let var = game_data[i+2];
            
            if const_val <= 20 && var == 0xd1 {
                println!("  Found: jl #{}, Vd1 at 0x{:05x}", const_val, i);
            }
        }
    }

    Ok(())
}