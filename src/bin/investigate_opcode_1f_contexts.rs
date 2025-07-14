use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Investigating opcode 0x1F contexts to understand what it should do...\n");
    
    // Find all occurrences of opcode 0x1F
    let mut occurrences = Vec::new();
    
    for i in 0..game_data.len()-3 {
        let byte = game_data[i];
        
        // Check for Variable form 0x1F (0xE0 + 0x1F = 0xFF, but we need to check properly)
        if (byte & 0xE0) == 0xE0 && (byte & 0x1F) == 0x1F {
            occurrences.push(i);
        }
    }
    
    println!("Found {} occurrences of opcode 0x1F", occurrences.len());
    
    // Look at the contexts of the first few occurrences
    for (idx, &addr) in occurrences.iter().take(10).enumerate() {
        println!("\n{}. Opcode 0x1F at 0x{:05x}:", idx + 1, addr);
        
        // Show context
        print!("   Context: ");
        for i in 0..8 {
            if addr + i < game_data.len() {
                print!("{:02x} ", game_data[addr + i]);
            }
        }
        println!();
        
        // Check operands
        if addr + 3 < game_data.len() {
            let operand1 = game_data[addr + 1];
            let operand2 = game_data[addr + 2];
            let store_var = game_data[addr + 3];
            
            println!("   Operands: 0x{:02x}, 0x{:02x} -> V{:02x}", operand1, operand2, store_var);
            
            // Special cases
            if store_var == 0xd1 {
                println!("   *** STORES TO Vd1 (action variable)! ***");
            }
            if store_var == 0x52 {
                println!("   *** STORES TO V52 (LIT variable)! ***");
            }
        }
        
        // Check what comes after
        if addr + 4 < game_data.len() {
            let next_byte = game_data[addr + 4];
            println!("   Next instruction: 0x{:02x}", next_byte);
        }
    }
    
    // Look specifically at the two critical uses:
    println!("\n\nCRITICAL USES:");
    
    // 1. The one that was clearing LIT (we know this one)
    println!("1. Opcode 0x1F that was clearing LIT variable (fixed)");
    
    // 2. The one in main dispatch that should set up action
    println!("2. Opcode 0x1F in main dispatch at 0x07e05:");
    let addr = 0x07e05;
    if addr + 3 < game_data.len() {
        let operand1 = game_data[addr + 1];
        let operand2 = game_data[addr + 2];
        let store_var = game_data[addr + 3];
        
        println!("   Operands: 0x{:02x}, 0x{:02x} -> V{:02x}", operand1, operand2, store_var);
        
        if store_var == 0xd1 {
            println!("   *** THIS SHOULD SET UP THE ACTION CODE! ***");
            println!("   But we're treating it as NOP, so Vd1 stays 0");
        }
    }
    
    println!("\nHypothesis: Opcode 0x1F is NOT a NOP!");
    println!("It's likely a store instruction or action setup instruction.");
    println!("The main dispatch uses it to set up the initial LOOK action.");
    
    // Let's see if there's a pattern in the operands
    println!("\nOperand patterns for opcode 0x1F:");
    let mut operand_patterns = std::collections::HashMap::new();
    
    for &addr in &occurrences {
        if addr + 3 < game_data.len() {
            let operand1 = game_data[addr + 1];
            let operand2 = game_data[addr + 2];
            let key = (operand1, operand2);
            *operand_patterns.entry(key).or_insert(0) += 1;
        }
    }
    
    for ((op1, op2), count) in operand_patterns {
        println!("  Operands 0x{:02x}, 0x{:02x}: {} times", op1, op2, count);
    }
    
    Ok(())
}