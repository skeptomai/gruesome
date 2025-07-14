use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Analyzing JZ instruction at 0x8cb2...\n");
    
    // Show raw bytes
    println!("Raw bytes at 0x8cb2:");
    for i in 0..8 {
        print!("{:02x} ", game_data[0x8cb2 + i]);
    }
    println!("\n");
    
    // Decode the instruction
    match Instruction::decode(&game_data, 0x8cb2, 3) {
        Ok(inst) => {
            println!("Decoded instruction:");
            println!("  Opcode: 0x{:02x}", inst.opcode);
            println!("  Size: {} bytes", inst.size);
            println!("  Formatted: {}", inst.format_with_version(3));
            
            if let Some(ref branch) = inst.branch {
                println!("\nBranch details:");
                println!("  on_true: {}", branch.on_true);
                println!("  offset: {} (0x{:04x})", branch.offset, branch.offset as u16);
                
                // Calculate where it would branch to
                let pc_after_inst = 0x8cb2 + inst.size;
                let target = (pc_after_inst as i32 + branch.offset as i32 - 2) as u32;
                println!("  PC after instruction: 0x{:05x}", pc_after_inst);
                println!("  Branch target: 0x{:05x}", target);
                
                if target == 0x8ce4 {
                    println!("  ✓ Correct! Should branch to 0x8ce4");
                } else {
                    println!("  ✗ Wrong! Should branch to 0x8ce4, not 0x{:05x}", target);
                    
                    // What offset would give us 0x8ce4?
                    let correct_offset = 0x8ce4 as i32 - pc_after_inst as i32 + 2;
                    println!("  Correct offset would be: {} (0x{:04x})", 
                            correct_offset, correct_offset as u16);
                }
            }
        }
        Err(e) => {
            println!("Error decoding: {}", e);
        }
    }
    
    // According to disassembly: JZ G42 [FALSE] 8ce4
    println!("\nAccording to disassembly:");
    println!("  Should be: JZ G42 [FALSE] 8ce4");
    println!("  G42 = V52 (global variable 42)");
    
    Ok(())
}