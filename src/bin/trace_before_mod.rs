use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Tracing instructions leading to 0x07554...\n");
    
    // Try to decode instructions before 0x07554
    let mut pc = 0x07540;
    let target = 0x07554;
    
    while pc < target + 10 {
        print!("{:05x}: ", pc);
        
        // Show raw bytes
        for i in 0..8 {
            if pc + i < game_data.len() {
                print!("{:02x} ", game_data[pc + i]);
            }
        }
        print!("  ");
        
        // Try to decode
        match Instruction::decode(&game_data, pc, 3) {
            Ok(inst) => {
                println!("{}", inst.format_with_version(3));
                
                if pc == target {
                    println!("\n  ^^^ This is where the error occurs!");
                    println!("  Opcode: 0x{:02x}, Form: {:?}, Operand count: {:?}", 
                            inst.opcode, inst.form, inst.operand_count);
                    println!("  Operand types: {:?}", inst.operand_types);
                    println!("  Operands: {:?}", inst.operands);
                }
                
                pc += inst.size;
            }
            Err(e) => {
                println!("ERROR: {}", e);
                if pc == target {
                    println!("  ^^^ This is the problematic MOD instruction!");
                }
                pc += 1; // Skip this byte and try again
            }
        }
    }
    
    // Also check if we can find any jumps or calls to addresses near 0x07554
    println!("\nLooking for references to addresses near 0x07554...");
    
    // Check the routine that starts at 0x07500
    println!("\nRoutine at 0x07500:");
    let routine_addr = 0x07500;
    if routine_addr < game_data.len() {
        let num_locals = game_data[routine_addr];
        println!("  Number of locals: {}", num_locals);
        
        let mut addr = routine_addr + 1;
        if num_locals > 0 && num_locals <= 15 {
            println!("  Local initial values:");
            for i in 0..num_locals {
                if addr + 1 < game_data.len() {
                    let value = ((game_data[addr] as u16) << 8) | (game_data[addr + 1] as u16);
                    println!("    L{:02}: 0x{:04x}", i + 1, value);
                    addr += 2;
                }
            }
            
            println!("\n  First instruction at 0x{:05x}:", addr);
            if let Ok(inst) = Instruction::decode(&game_data, addr, 3) {
                println!("    {}", inst.format_with_version(3));
            }
        }
    }
    
    Ok(())
}