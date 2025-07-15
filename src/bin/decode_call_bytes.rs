use infocom::vm::Game;
use infocom::instruction::{Instruction, OperandCount};
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Decoding the Call Instruction Bytes ===\n");

    // Load Zork 1
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    
    println!("Looking at PC 0x06dfc where the bad call occurs:");
    println!();
    
    // Show raw bytes
    println!("Raw bytes starting at 0x06dfc:");
    for i in 0..10 {
        println!("  {:05x}: {:02x} {:08b}", 0x06dfc + i, 
                game.memory[0x06dfc + i],
                game.memory[0x06dfc + i]);
    }
    println!();
    
    // Decode the instruction
    match Instruction::decode(&game.memory, 0x06dfc, game.header.version) {
        Ok(inst) => {
            println!("Decoded instruction:");
            println!("  Opcode: 0x{:02x}", inst.opcode);
            println!("  Operand count: {:?}", inst.operand_count);
            println!("  Operands: {:?}", inst.operands);
            println!("  Size: {} bytes", inst.size);
            println!("  Format: {}", inst.format_with_version(game.header.version));
            
            if inst.opcode == 0x01 {
                println!("\nThis is indeed a call instruction!");
                if let Some(&packed) = inst.operands.get(0) {
                    println!("  Packed address: 0x{:04x}", packed);
                    println!("  Unpacked address: 0x{:05x}", (packed as u32) * 2);
                }
            }
        }
        Err(e) => {
            println!("Error decoding instruction: {}", e);
        }
    }
    
    // Let's manually decode the VAR form instruction
    println!("\nManual decoding of VAR form:");
    let byte0 = game.memory[0x06dfc];
    println!("First byte: 0x{:02x} = {:08b}", byte0, byte0);
    
    if byte0 >= 0xE0 {
        println!("This is a VAR form instruction (first byte >= 0xE0)");
        let opcode = byte0 & 0x1F;
        println!("Opcode: 0x{:02x}", opcode);
        
        // Next byte is operand types
        let types_byte = game.memory[0x06dfd];
        println!("\nOperand types byte: 0x{:02x} = {:08b}", types_byte, types_byte);
        
        // Decode operand types
        let mut operand_types = Vec::new();
        for i in 0..4 {
            let shift = 6 - (i * 2);
            let op_type = (types_byte >> shift) & 0x03;
            match op_type {
                0 => operand_types.push("large constant (2 bytes)"),
                1 => operand_types.push("small constant (1 byte)"),
                2 => operand_types.push("variable"),
                3 => operand_types.push("omitted"),
                _ => unreachable!()
            }
        }
        
        println!("\nOperand types:");
        for (i, op_type) in operand_types.iter().enumerate() {
            println!("  Operand {}: {}", i, op_type);
        }
        
        // Now decode the actual operands
        println!("\nOperand values:");
        let mut offset = 0x06dfe; // Start after opcode and types byte
        
        for i in 0..4 {
            let shift = 6 - (i * 2);
            let op_type = (types_byte >> shift) & 0x03;
            
            match op_type {
                0 => {
                    // Large constant (2 bytes)
                    let high = game.memory[offset];
                    let low = game.memory[offset + 1];
                    let value = ((high as u16) << 8) | (low as u16);
                    println!("  Operand {}: 0x{:04x} (bytes: {:02x} {:02x})", i, value, high, low);
                    offset += 2;
                }
                1 => {
                    // Small constant (1 byte)
                    let value = game.memory[offset];
                    println!("  Operand {}: 0x{:02x}", i, value);
                    offset += 1;
                }
                2 => {
                    // Variable
                    let var_num = game.memory[offset];
                    println!("  Operand {}: V{:02x}", i, var_num);
                    offset += 1;
                }
                3 => {
                    // Omitted
                    println!("  Operand {}: (omitted)", i);
                    break;
                }
                _ => unreachable!()
            }
        }
        
        // Store variable if it's a call
        if opcode == 0x00 || opcode == 0x01 {
            let store_var = game.memory[offset];
            println!("\nStore variable: V{:02x}", store_var);
        }
    }
    
    Ok(())
}