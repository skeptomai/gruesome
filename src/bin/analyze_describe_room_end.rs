use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Analyzing DESCRIBE-ROOM routine around the print...\n");
    
    // From the user's disassembly:
    // 8d19: PRINT       "West of House"
    // We need to find what happens after this
    
    // Let's decode instructions starting from 0x8d19
    let mut pc = 0x8d19;
    
    println!("Instructions in DESCRIBE-ROOM after LIT check:");
    
    for _ in 0..20 {
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
                
                // Check if this is the print instruction
                if pc == 0x8d19 && inst.text.is_some() {
                    println!("        Text: \"{}\"", inst.text.as_ref().unwrap());
                }
                
                // Check for calls
                if inst.name(3).starts_with("call") && !inst.operands.is_empty() {
                    let packed = inst.operands[0];
                    let unpacked = (packed as u32) * 2;
                    println!("        -> Calls to 0x{:05x}", unpacked);
                    
                    if unpacked >= 0x07000 && unpacked <= 0x08000 {
                        println!("        -> WARNING: Suspicious address!");
                    }
                }
                
                pc += inst.size;
            }
            Err(e) => {
                println!("ERROR: {}", e);
                pc += 1;
            }
        }
    }
    
    println!("\n\nChecking what comes after the print at 0x8d19:");
    
    // The print instruction should be a print or print_ret
    if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, 0x8d19, 3) {
        let next_pc = 0x8d19 + inst.size;
        println!("  Print instruction size: {} bytes", inst.size);
        println!("  Next PC after print: 0x{:05x}", next_pc);
        
        // What's the next instruction?
        if let Ok(next_inst) = infocom::instruction::Instruction::decode(&game_data, next_pc, 3) {
            println!("  Next instruction: {}", next_inst.format_with_version(3));
            
            if next_inst.name(3).starts_with("call") {
                if !next_inst.operands.is_empty() {
                    let target = (next_inst.operands[0] as u32) * 2;
                    println!("    Calls to: 0x{:05x}", target);
                }
            }
        }
    }
    
    Ok(())
}