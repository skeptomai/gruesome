use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Analyzing PUSH instruction at 0x8ca6...\n");
    
    // Decode the PUSH at 0x8ca6
    match Instruction::decode(&game_data, 0x8ca6, 3) {
        Ok(inst) => {
            println!("Instruction at 0x8ca6:");
            println!("  Opcode: 0x{:02x}", inst.opcode);
            println!("  Form: {:?}", inst.form);
            println!("  Operands: {:?}", inst.operands);
            println!("  Size: {} bytes", inst.size);
            println!("  Formatted: {}", inst.format_with_version(3));
            
            if inst.size != 3 {
                println!("\n*** PROBLEM: Size is {} but should be 3! ***", inst.size);
            }
        }
        Err(e) => {
            println!("Error decoding at 0x8ca6: {}", e);
        }
    }
    
    // Show the raw bytes
    println!("\nRaw bytes at 0x8ca6:");
    for i in 0..5 {
        print!("{:02x} ", game_data[0x8ca6 + i]);
    }
    println!();
    
    // Manually analyze the instruction
    let opcode_byte = game_data[0x8ca6];
    println!("\nManual analysis of 0xe8:");
    println!("  Binary: {:08b}", opcode_byte);
    println!("  Form: Variable (top 2 bits = 11)");
    println!("  Opcode: 0x{:02x} (bottom 5 bits = 01000 = 8 = push)", opcode_byte & 0x1F);
    
    // Check operand types byte
    let operand_types = game_data[0x8ca7];
    println!("\nOperand types byte 0x{:02x}:", operand_types);
    println!("  Binary: {:08b}", operand_types);
    println!("  Operand 1: {} (bits 7-6)", match (operand_types >> 6) & 0x03 {
        0 => "Large constant",
        1 => "Small constant", 
        2 => "Variable",
        3 => "Omitted",
        _ => "?"
    });
    println!("  Operand 2: {} (bits 5-4)", match (operand_types >> 4) & 0x03 {
        0 => "Large constant",
        1 => "Small constant",
        2 => "Variable", 
        3 => "Omitted",
        _ => "?"
    });
    
    // The issue might be here - if operand 1 is Variable (10), then:
    // - Byte 0: 0xe8 (opcode)
    // - Byte 1: 0xbf (operand types)
    // - Byte 2: 0x01 (variable number)
    // Total: 3 bytes
    
    // But if the decoder thinks it's something else, it might calculate wrong size
    
    // Also check what happens if we decode from 0x8ca9
    println!("\n\nWhat if we decode from 0x8ca9 (where we should be)?");
    match Instruction::decode(&game_data, 0x8ca9, 3) {
        Ok(inst) => {
            println!("  Formatted: {}", inst.format_with_version(3));
            println!("  Size: {} bytes", inst.size);
            
            // After this instruction, PC should be at:
            println!("  After this instruction, PC should be: 0x{:05x}", 0x8ca9 + inst.size);
        }
        Err(e) => {
            println!("  Error: {}", e);
        }
    }
    
    Ok(())
}