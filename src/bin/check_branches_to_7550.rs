use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Checking branches that might lead to 0x07550...\n");
    
    // Decode instructions from 0x0751f and check their branches
    let mut pc = 0x0751f;
    
    while pc < 0x07560 {
        if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, pc, 3) {
            print!("{:05x}: {}", pc, inst.format_with_version(3));
            
            // Check if this instruction has a branch
            if let Some(ref branch) = inst.branch {
                let pc_after = pc + inst.size;
                let target = if branch.offset >= 2 {
                    (pc_after as i32 + branch.offset as i32 - 2) as u32
                } else if branch.offset == 0 {
                    // Return false
                    println!(" -> RFALSE");
                    pc += inst.size;
                    continue;
                } else if branch.offset == 1 {
                    // Return true
                    println!(" -> RTRUE");
                    pc += inst.size;
                    continue;
                } else {
                    (pc_after as i32 + branch.offset as i32 - 2) as u32
                };
                
                println!(" -> branches to 0x{:05x}", target);
                
                if target == 0x07550 {
                    println!("  ^^^ FOUND IT! This branches to 0x07550!");
                } else if target > 0x07540 && target < 0x07560 {
                    println!("  ^^^ This branches near 0x07550");
                }
            } else {
                println!();
            }
            
            pc += inst.size;
        } else {
            println!("{:05x}: DECODE ERROR", pc);
            break;
        }
    }
    
    // Also check specific problematic instructions
    println!("\n\nSpecific checks:");
    
    // The inc_chk at 0x0752e
    println!("\n1. inc_chk at 0x0752e:");
    if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, 0x0752e, 3) {
        println!("   {}", inst.format_with_version(3));
        
        if let Some(ref branch) = inst.branch {
            let pc_after = 0x0752e + inst.size;
            let target = (pc_after as i32 + branch.offset as i32 - 2) as u32;
            println!("   PC after: 0x{:05x}", pc_after);
            println!("   Offset: {:+}", branch.offset);
            println!("   Target: 0x{:05x}", target);
            
            if target == 0x07550 {
                println!("   ^^^ This is it!");
            }
        }
    }
    
    // What if the branch offset is being calculated wrong?
    println!("\n2. Manual calculation for inc_chk at 0x0752e:");
    let pc = 0x0752e;
    let bytes: Vec<u8> = (0..8).map(|i| game_data[pc + i]).collect();
    println!("   Raw bytes: {:02x} {:02x} {:02x} {:02x} {:02x}", 
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4]);
    
    // inc_chk is 2OP:0x05
    // Bytes should be: 45 b3 13 8d 1b
    // 45 = 01000101 = Long form, 2OP:05, both small constants
    // b3 = first operand
    // 13 = second operand  
    // 8d 1b = branch bytes
    
    let branch_byte1 = bytes[3]; // 0x8d
    let branch_byte2 = bytes[4]; // 0x1b
    
    println!("   Branch bytes: {:02x} {:02x}", branch_byte1, branch_byte2);
    println!("   on_true: {}", (branch_byte1 & 0x80) != 0);
    
    if (branch_byte1 & 0x40) == 0 {
        // Long form branch
        let offset = (((branch_byte1 & 0x3F) as i16) << 8) | (branch_byte2 as i16);
        let signed_offset = if offset & 0x2000 != 0 {
            offset | (0xC000u16 as i16)
        } else {
            offset
        };
        
        println!("   Long form offset: {} (0x{:04x})", signed_offset, offset);
        
        let pc_after = 0x0752e + 5; // 5 byte instruction
        let target = (pc_after as i32 + signed_offset as i32 - 2) as u32;
        println!("   Target: 0x{:05x} + {} - 2 = 0x{:05x}", pc_after, signed_offset, target);
    }
    
    Ok(())
}