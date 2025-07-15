use infocom::instruction::Instruction;
use infocom::vm::{Game, VM};
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the game
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;

    println!("Debugging CALL instruction at 0x6dfc in V-QUIT routine\n");

    // Examine the bytes at 0x6dfc
    let pc = 0x6dfc;
    println!("Raw bytes at 0x{:04x}:", pc);
    for i in 0..8 {
        print!("{:02x} ", game.memory[pc + i]);
    }
    println!("\n");

    // Expected: e0 3f 48 6e 00
    // e0 = VAR:00 (call)
    // 3f = operand types: 00 11 11 11 (large, omitted, omitted, omitted)
    // 48 6e = large constant 0x486e (packed address)
    // 00 = store to stack

    // Decode the instruction
    match Instruction::decode(&game.memory, pc, 3) {
        Ok(inst) => {
            println!("Decoded instruction: {}", inst);
            println!("  Form: {:?}", inst.form);
            println!("  Opcode: 0x{:02x}", inst.opcode);
            println!("  Operand count: {:?}", inst.operand_count);
            println!("  Operands: {:?}", inst.operands);
            println!("  Store var: {:?}", inst.store_var);
            println!("  Size: {} bytes", inst.size);
            
            if !inst.operands.is_empty() {
                let packed = inst.operands[0];
                println!("\nFirst operand (packed address): 0x{:04x}", packed);
                println!("Unpacked (V3: * 2): 0x{:05x}", packed * 2);
                
                if packed == 0x486e {
                    println!("✓ Correct packed address!");
                } else if packed == 0x6e48 {
                    println!("✗ Bytes reversed! Got 0x6e48 instead of 0x486e");
                } else {
                    println!("✗ Wrong value entirely!");
                }
            }
        }
        Err(e) => {
            println!("Failed to decode instruction: {}", e);
            
            // Try to manually decode
            println!("\nManual decode attempt:");
            let opcode_byte = game.memory[pc];
            println!("Opcode byte: 0x{:02x}", opcode_byte);
            
            if opcode_byte == 0xe0 {
                println!("This is VAR:00 (call)");
                let types_byte = game.memory[pc + 1];
                println!("Operand types byte: 0x{:02x}", types_byte);
                
                // Decode operand types
                let op1_type = (types_byte >> 6) & 0x03;
                let op2_type = (types_byte >> 4) & 0x03;
                let op3_type = (types_byte >> 2) & 0x03;
                let op4_type = types_byte & 0x03;
                
                println!("Operand types: {} {} {} {}",
                    match op1_type {
                        0 => "large",
                        1 => "small",
                        2 => "variable",
                        3 => "omitted",
                        _ => "?",
                    },
                    match op2_type {
                        0 => "large",
                        1 => "small",
                        2 => "variable",
                        3 => "omitted",
                        _ => "?",
                    },
                    match op3_type {
                        0 => "large",
                        1 => "small",
                        2 => "variable",
                        3 => "omitted",
                        _ => "?",
                    },
                    match op4_type {
                        0 => "large",
                        1 => "small",
                        2 => "variable",
                        3 => "omitted",
                        _ => "?",
                    }
                );
                
                if op1_type == 0 {
                    println!("\nFirst operand is large constant (2 bytes):");
                    let byte1 = game.memory[pc + 2];
                    let byte2 = game.memory[pc + 3];
                    println!("  Bytes: {:02x} {:02x}", byte1, byte2);
                    let value = ((byte1 as u16) << 8) | (byte2 as u16);
                    println!("  Value (big-endian): 0x{:04x}", value);
                    println!("  Unpacked address: 0x{:05x}", value * 2);
                }
            }
        }
    }

    Ok(())
}