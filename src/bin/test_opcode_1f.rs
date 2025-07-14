use std::fs::File;
use std::io::prelude::*;

fn main() -> std::io::Result<()> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    // Find instances of opcode 0x1F in the game
    println!("Searching for opcode 0x1F instances...\n");
    
    for i in 0..game_data.len()-4 {
        let byte = game_data[i];
        
        // Check for Long form 2OP:0x1F (byte pattern: 01xx xxxx where bottom 5 bits = 0x1F)
        if (byte >> 6) < 2 && (byte & 0x1F) == 0x1F {
            println!("Found opcode 0x1F at 0x{:05x}:", i);
            println!("  Bytes: {:02x} {:02x} {:02x} {:02x}", 
                     game_data[i], game_data[i+1], game_data[i+2], game_data[i+3]);
            
            // Decode the instruction
            let op1_type = if byte & 0x40 != 0 { "variable" } else { "small const" };
            let op2_type = if byte & 0x20 != 0 { "variable" } else { "small const" };
            
            println!("  Operand 1: {} 0x{:02x}", op1_type, game_data[i+1]);
            println!("  Operand 2: {} 0x{:02x}", op2_type, game_data[i+2]);
            println!("  Store to: variable 0x{:02x}", game_data[i+3]);
            println!();
        }
    }

    // Now let's specifically examine the instance at 0x08cb0
    println!("\nDetailed analysis of instance at 0x08cb0:");
    let addr = 0x08cb0;
    if addr + 4 < game_data.len() {
        let opcode_byte = game_data[addr];
        let op1 = game_data[addr + 1];
        let op2 = game_data[addr + 2]; 
        let store_var = game_data[addr + 3];
        
        println!("  Opcode byte: 0x{:02x} (binary: {:08b})", opcode_byte, opcode_byte);
        println!("  Operation: 0x1F with operands {} and variable {}", op1, op2);
        println!("  Stores to: variable 0x{:02x} ({})", store_var, 
                 if store_var == 0x52 { "LIT global" } else { "other" });
        
        // If operand 1 is a small constant, let's see what values are commonly used
        if opcode_byte & 0x40 == 0 {
            println!("\n  First operand is constant: {}", op1);
            println!("  This might be:");
            println!("    - Shift amount (if shift instruction)");
            println!("    - Bit mask (if bitwise operation)");
            println!("    - Arithmetic constant");
        }
    }

    Ok(())
}