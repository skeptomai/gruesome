use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Analyzing where Vd1 is initialized to 0 at PC 0x04f05:\n");
    
    let pc = 0x04f05;
    
    // Show context around this instruction
    println!("Instructions around 0x04f05:");
    for offset in [-10, -8, -6, -4, -2, 0, 2, 4, 6, 8, 10].iter() {
        let addr = (pc as i32 + offset) as usize;
        if addr < game_data.len() {
            println!("  0x{:05x}: {:02x}", addr, game_data[addr]);
        }
    }
    
    // Check what instruction this is
    println!("\nDecoding instruction at 0x04f05:");
    let byte = game_data[pc];
    println!("  Opcode byte: 0x{:02x} (binary: {:08b})", byte, byte);
    
    if byte == 0x0E {
        // store (small constant #, variable)
        if pc + 2 < game_data.len() {
            let constant = game_data[pc + 1];
            let var = game_data[pc + 2];
            println!("  This is: store #{} -> V{:02x}", constant, var);
            if var == 0xd1 {
                println!("  *** This stores {} to Vd1! ***", constant);
            }
        }
    }
    
    // Let's also check what routine we're in
    println!("\n\nLooking for routine header before 0x04f05:");
    let mut search_pc = pc;
    while search_pc > 0 && search_pc > pc - 100 {
        search_pc -= 1;
        
        // Check for routine header pattern
        // Routines start with a byte indicating number of locals
        if search_pc > 0 && game_data[search_pc - 1] >= 0xB0 && game_data[search_pc - 1] <= 0xBF {
            // Previous instruction was likely a call, return, or branch
            let locals = game_data[search_pc];
            if locals <= 15 {
                println!("  Possible routine start at 0x{:05x} with {} locals", search_pc, locals);
                
                // Show some following instructions
                print!("    Next bytes: ");
                for i in 0..10 {
                    if search_pc + i < game_data.len() {
                        print!("{:02x} ", game_data[search_pc + i]);
                    }
                }
                println!();
            }
        }
    }
    
    // Let's check what happens after Vd1 is set to 0
    println!("\n\nWhat happens after setting Vd1 to 0:");
    let mut offset = pc + 3; // Skip the store instruction
    for _ in 0..5 {
        if offset + 5 < game_data.len() {
            println!("  0x{:05x}: {:02x} {:02x} {:02x} {:02x} {:02x}", 
                     offset, game_data[offset], game_data[offset+1], 
                     game_data[offset+2], game_data[offset+3], game_data[offset+4]);
            offset += 3; // Rough estimate
        }
    }

    Ok(())
}