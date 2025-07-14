use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Analyzing MOD instruction error at 0x07554...\n");
    
    // Show raw bytes
    println!("Raw bytes at 0x07554:");
    for i in 0..8 {
        if 0x07554 + i < game_data.len() {
            print!("{:02x} ", game_data[0x07554 + i]);
        }
    }
    println!("\n");
    
    // Decode the instruction
    match Instruction::decode(&game_data, 0x07554, 3) {
        Ok(inst) => {
            println!("Decoded instruction:");
            println!("  Opcode: 0x{:02x}", inst.opcode);
            println!("  Form: {:?}", inst.form);
            println!("  Operand count: {:?}", inst.operand_count);
            println!("  Operand types: {:?}", inst.operand_types);
            println!("  Operands: {:?}", inst.operands);
            println!("  Size: {} bytes", inst.size);
            println!("  Formatted: {}", inst.format_with_version(3));
        }
        Err(e) => {
            println!("Error decoding: {}", e);
            
            // Manual decode
            let byte0 = game_data[0x07554];
            let byte1 = game_data[0x07555];
            
            println!("\nManual analysis:");
            println!("  Byte 0: 0x{:02x} = {:08b} binary", byte0, byte0);
            println!("  Top 2 bits: {:02b}", byte0 >> 6);
            
            if byte0 >= 0xC0 {
                println!("  This is Variable form (top 2 bits = 11)");
                println!("  Bottom 5 bits: 0x{:02x} = opcode", byte0 & 0x1F);
                
                if (byte0 & 0x20) == 0 {
                    println!("  Bit 5 = 0, so this is Variable 2OP");
                } else {
                    println!("  Bit 5 = 1, so this is Variable VAR");
                }
                
                println!("\n  Operand types byte: 0x{:02x}", byte1);
                println!("    Op1: {:02b} = {}", (byte1 >> 6) & 3, 
                        match (byte1 >> 6) & 3 {
                            0 => "Large", 1 => "Small", 2 => "Variable", 3 => "Omitted", _ => "?"
                        });
                println!("    Op2: {:02b} = {}", (byte1 >> 4) & 3,
                        match (byte1 >> 4) & 3 {
                            0 => "Large", 1 => "Small", 2 => "Variable", 3 => "Omitted", _ => "?"
                        });
                println!("    Op3: {:02b} = {}", (byte1 >> 2) & 3,
                        match (byte1 >> 2) & 3 {
                            0 => "Large", 1 => "Small", 2 => "Variable", 3 => "Omitted", _ => "?"
                        });
                println!("    Op4: {:02b} = {}", byte1 & 3,
                        match byte1 & 3 {
                            0 => "Large", 1 => "Small", 2 => "Variable", 3 => "Omitted", _ => "?"
                        });
                
                if (byte0 & 0x1F) == 0x18 {
                    println!("\n  Opcode 0x18 = MOD");
                    println!("  MOD is a 2OP instruction that requires exactly 2 operands");
                    
                    // Count non-omitted operands
                    let mut op_count = 0;
                    for i in 0..4 {
                        let op_type = (byte1 >> (6 - i * 2)) & 3;
                        if op_type != 3 { // Not omitted
                            op_count += 1;
                        }
                    }
                    println!("  Operands provided: {}", op_count);
                }
            }
        }
    }
    
    // Show some context
    println!("\nContext around 0x07554:");
    for addr in (0x07544..=0x07564).step_by(1) {
        if addr < game_data.len() {
            print!("{:05x}: ", addr);
            for i in 0..8 {
                if addr + i < game_data.len() {
                    print!("{:02x} ", game_data[addr + i]);
                }
            }
            println!();
        }
    }
    
    Ok(())
}