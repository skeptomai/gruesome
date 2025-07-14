use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Analyzing main dispatch routine at 0x07e04...\n");
    
    let addr = 0x07e04;
    
    // Show the bytes at this address
    println!("Bytes at 0x07e04 (main dispatch):");
    for i in 0..20 {
        if addr + i < game_data.len() {
            println!("  0x{:05x}: {:02x}", addr + i, game_data[addr + i]);
        }
    }
    
    // Look for the routine header
    let mut routine_start = addr;
    while routine_start > 0 && routine_start > addr - 50 {
        routine_start -= 1;
        
        // Check if this could be a routine header
        if routine_start > 0 && game_data[routine_start - 1] >= 0xB0 && game_data[routine_start - 1] <= 0xBF {
            let locals = game_data[routine_start];
            if locals <= 15 {
                println!("\nPossible routine start at 0x{:05x} with {} locals", routine_start, locals);
                
                // Show the routine structure
                print!("  Routine bytes: ");
                for i in 0..20 {
                    if routine_start + i < game_data.len() {
                        print!("{:02x} ", game_data[routine_start + i]);
                    }
                }
                println!();
                
                // If this is the start of our routine, analyze it
                if routine_start <= addr && addr < routine_start + 100 {
                    println!("  This contains our main dispatch at offset {}", addr - routine_start);
                    
                    // Check if there's a pattern that suggests initial action setup
                    let mut offset = routine_start + 1; // Skip locals count
                    for _ in 0..5 {
                        if offset + 3 < game_data.len() {
                            let byte = game_data[offset];
                            
                            // Check for store instructions
                            if byte == 0x0E {
                                let constant = game_data[offset + 1];
                                let var = game_data[offset + 2];
                                println!("    Found store #{} -> V{:02x} at 0x{:05x}", constant, var, offset);
                                
                                if var == 0xd1 {
                                    println!("      *** This stores to Vd1 (action variable)! ***");
                                }
                            }
                            
                            offset += 3; // Rough estimate for next instruction
                        }
                    }
                }
            }
        }
    }
    
    println!("\n\nLet's also check what should happen in a working interpreter:");
    println!("1. After serial number, game should call main dispatch");
    println!("2. Main dispatch should either:");
    println!("   a) Set up initial LOOK action (store #6 to Vd1), or");
    println!("   b) Call a routine that sets up the initial game state");
    println!("3. Then proceed to show the room description");
    
    println!("\nCurrently we see:");
    println!("- Main dispatch at 0x07e04 immediately calls 0x08c9a");
    println!("- 0x08c9a does 'jl #0005, Vd1' with Vd1=0");
    println!("- This branches to STAND routine instead of room description");
    
    Ok(())
}