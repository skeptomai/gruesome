use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Examining routine at 0x08d70...\n");
    
    let addr = 0x08d70;
    let num_locals = game_data[addr];
    println!("Number of locals: {}", num_locals);
    
    if num_locals <= 15 {
        let mut pc = addr + 1;
        
        // Read local initial values
        for i in 0..num_locals {
            let value = ((game_data[pc] as u16) << 8) | (game_data[pc + 1] as u16);
            println!("  L{:02}: 0x{:04x}", i + 1, value);
            pc += 2;
        }
        
        println!("\nFirst few instructions:");
        
        // Try to decode some instructions
        for _ in 0..10 {
            print!("{:05x}: ", pc);
            for i in 0..8 {
                if pc + i < game_data.len() {
                    print!("{:02x} ", game_data[pc + i]);
                }
            }
            print!("\n");
            
            // Try to decode
            match infocom::instruction::Instruction::decode(&game_data, pc, 3) {
                Ok(inst) => {
                    println!("        {}", inst.format_with_version(3));
                    
                    // Check if this might call to 0x07500
                    if inst.name(3).starts_with("call") && !inst.operands.is_empty() {
                        let packed = inst.operands[0];
                        let unpacked = (packed as u32) * 2;
                        if unpacked == 0x07500 {
                            println!("        ^^^ This calls to 0x07500!");
                        }
                    }
                    
                    pc += inst.size;
                }
                Err(_) => {
                    pc += 1;
                }
            }
            
            if pc > addr + 100 {
                break;
            }
        }
    }
    
    // Also check what's at 0x07500
    println!("\n\nWhat's at 0x07500:");
    for i in 0..32 {
        if i % 8 == 0 {
            print!("\n{:05x}: ", 0x07500 + i);
        }
        print!("{:02x} ", game_data[0x07500 + i]);
    }
    println!();
    
    // Check if 0x07500 looks like code or data
    println!("\nAnalysis of 0x07500:");
    let byte = game_data[0x07500];
    println!("  First byte: 0x{:02x} = {}", byte, byte);
    if byte <= 15 {
        println!("  Could be routine with {} locals", byte);
    } else {
        println!("  {} is too many locals for a routine (max 15)", byte);
        println!("  This is probably DATA, not code!");
    }
    
    Ok(())
}