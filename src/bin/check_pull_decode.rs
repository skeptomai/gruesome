use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Analyzing PULL instruction at 0x8caf...\n");
    
    // Show raw bytes
    println!("Raw bytes at 0x8caf:");
    for i in 0..5 {
        print!("{:02x} ", game_data[0x8caf + i]);
    }
    println!("\n");
    
    // Decode the instruction
    match Instruction::decode(&game_data, 0x8caf, 3) {
        Ok(inst) => {
            println!("Decoded instruction:");
            println!("  Opcode: 0x{:02x}", inst.opcode);
            println!("  Form: {:?}", inst.form);
            println!("  Operands: {:?}", inst.operands);
            println!("  Store variable: {:?}", inst.store_var);
            println!("  Size: {} bytes", inst.size);
            println!("  Formatted: {}", inst.format_with_version(3));
            
            if !inst.operands.is_empty() {
                println!("\nOperand analysis:");
                println!("  First operand value: {}", inst.operands[0]);
                println!("  As hex: 0x{:02x}", inst.operands[0]);
                println!("  This should be variable number to store to");
                
                if inst.operands[0] == 2 {
                    println!("  Variable 2 = L01 (local variable 1)");
                    println!("  Disassembly shows 'PULL L01' which matches!");
                }
            }
        }
        Err(e) => {
            println!("Error decoding: {}", e);
        }
    }
    
    // Manual decode
    let byte0 = game_data[0x8caf];
    let byte1 = game_data[0x8cb0];
    let byte2 = game_data[0x8cb1];
    
    println!("\nManual analysis:");
    println!("  Byte 0: 0x{:02x} = {:08b} binary", byte0, byte0);
    println!("  This is Variable form, opcode 0x09 (pull)");
    println!("  Byte 1: 0x{:02x} = operand types", byte1);
    println!("  Byte 2: 0x{:02x} = first operand", byte2);
    
    Ok(())
}