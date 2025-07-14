use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Checking instruction decoding around 0x07550...\n");
    
    // Decode instructions from 0x0751f (where we jump to) onwards
    let mut pc = 0x0751f;
    
    for _ in 0..10 {
        if pc >= game_data.len() {
            break;
        }
        
        print!("{:05x}: ", pc);
        
        // Show raw bytes
        for i in 0..8 {
            if pc + i < game_data.len() {
                print!("{:02x} ", game_data[pc + i]);
            }
        }
        print!("  ");
        
        // Try to decode
        match infocom::instruction::Instruction::decode(&game_data, pc, 3) {
            Ok(inst) => {
                println!("{}", inst.format_with_version(3));
                
                let next_pc = pc + inst.size;
                
                // Highlight if this leads to 0x07554
                if pc <= 0x07554 && next_pc > 0x07554 {
                    println!("        ^^^ This instruction contains 0x07554!");
                    println!("        Instruction ends at 0x{:05x}", next_pc);
                    
                    if next_pc == 0x07554 {
                        println!("        ERROR: Next instruction would start at 0x07554");
                        println!("        But 0x07554 is in the middle of the next instruction!");
                    }
                }
                
                pc = next_pc;
            }
            Err(e) => {
                println!("ERROR: {}", e);
                pc += 1;
            }
        }
        
        if pc > 0x07560 {
            break;
        }
    }
    
    // Also check the branch instruction at 0x093bb
    println!("\n\nChecking the branch instruction at 0x093bb:");
    
    if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, 0x093bb, 3) {
        println!("0x093bb: {}", inst.format_with_version(3));
        
        if let Some(ref branch) = inst.branch {
            let pc_after = 0x093bb + inst.size;
            let target = (pc_after as i32 + branch.offset as i32 - 2) as u32;
            
            println!("  PC after instruction: 0x{:05x}", pc_after);
            println!("  Branch offset: {:+} (0x{:04x})", branch.offset, branch.offset as u16);
            println!("  Target: 0x{:05x} + {:+} - 2 = 0x{:05x}", pc_after, branch.offset, target);
            
            if target != 0x0751f {
                println!("  ERROR: Expected target 0x0751f, got 0x{:05x}", target);
            }
        }
    }
    
    Ok(())
}