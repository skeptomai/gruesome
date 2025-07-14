use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Analyzing Variable 2OP MOD instructions in the game...\n");
    
    // Search for Variable 2OP MOD instructions (0xD8)
    // Variable form: top 2 bits = 11
    // 2OP: bit 5 = 0
    // MOD opcode: bottom 5 bits = 0x18
    // So we're looking for 0xD8 = 11011000
    
    let mut found_count = 0;
    
    for addr in 0x4e38..game_data.len() {
        if game_data[addr] == 0xD8 {
            println!("Found Variable 2OP MOD at 0x{:05x}", addr);
            found_count += 1;
            
            // Show context
            print!("  Bytes: ");
            for i in 0..8 {
                if addr + i < game_data.len() {
                    print!("{:02x} ", game_data[addr + i]);
                }
            }
            println!();
            
            // Check operand types byte
            if addr + 1 < game_data.len() {
                let types_byte = game_data[addr + 1];
                println!("  Operand types: 0x{:02x}", types_byte);
                
                let op1_type = (types_byte >> 6) & 3;
                let op2_type = (types_byte >> 4) & 3;
                
                println!("    Op1: {} ({})", op1_type, match op1_type {
                    0 => "Large", 1 => "Small", 2 => "Variable", 3 => "Omitted", _ => "?"
                });
                println!("    Op2: {} ({})", op2_type, match op2_type {
                    0 => "Large", 1 => "Small", 2 => "Variable", 3 => "Omitted", _ => "?"
                });
                
                if op1_type == 3 || op2_type == 3 {
                    println!("    WARNING: MOD needs 2 operands but has omitted operand!");
                }
            }
            
            // Try to decode the instruction
            match infocom::instruction::Instruction::decode(&game_data, addr, 3) {
                Ok(inst) => {
                    println!("  Decoded: {}", inst.format_with_version(3));
                    if inst.operands.len() < 2 {
                        println!("    ERROR: Only {} operands!", inst.operands.len());
                    }
                }
                Err(e) => {
                    println!("  Decode error: {}", e);
                }
            }
            
            println!();
        }
    }
    
    println!("Found {} Variable 2OP MOD instructions", found_count);
    
    // Also check the specific problematic one
    println!("\n\nSpecific check of 0x07554:");
    let addr = 0x07554;
    if addr < game_data.len() && game_data[addr] == 0xD8 {
        println!("Confirmed: 0xD8 at 0x07554");
        
        // Let's trace backwards to see what might have led here
        println!("\nChecking what comes before 0x07554:");
        
        for back_addr in (0x07540..0x07554).rev() {
            if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, back_addr, 3) {
                if back_addr + inst.size == 0x07554 {
                    println!("\nInstruction at 0x{:05x} (size {}) leads directly to 0x07554:", 
                            back_addr, inst.size);
                    println!("  {}", inst.format_with_version(3));
                    
                    // Is this a branch or call?
                    if inst.branch.is_some() || inst.name(3).contains("call") {
                        println!("  This is a {} instruction", inst.name(3));
                    }
                }
            }
        }
    }
    
    Ok(())
}