use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Analyzing the inc_chk at 0x0751f that causes the problem...\n");
    
    // The inc_chk at 0x0751f
    let addr = 0x0751f;
    let bytes: Vec<u8> = (0..6).map(|i| game_data[addr + i]).collect();
    
    println!("Raw bytes at 0x0751f: {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]);
    
    // Decode the instruction
    if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, addr, 3) {
        println!("Decoded as: {}", inst.format_with_version(3));
        println!("Size: {} bytes", inst.size);
        
        if inst.size != 4 {
            println!("ERROR: Expected size 4 for inc_chk!");
        }
        
        // The branch
        if let Some(ref branch) = inst.branch {
            println!("\nBranch info:");
            println!("  on_true: {}", branch.on_true);
            println!("  offset: {} (0x{:04x})", branch.offset, branch.offset as u16);
            
            let pc_after = addr + inst.size;
            let target = (pc_after as i32 + branch.offset as i32 - 2) as u32;
            
            println!("  PC after: 0x{:05x}", pc_after);
            println!("  Target: 0x{:05x}", target);
            
            if target == 0x07547 {
                println!("  ^^^ This branches into the print_ret text!");
            }
        }
    }
    
    // The issue is that we're branching to 0x07547, but that's inside
    // the print_ret string that starts at 0x0753a
    
    println!("\nThe problem:");
    println!("1. inc_chk at 0x0751f branches to 0x07547 on FALSE");
    println!("2. But 0x07547 is inside the text of print_ret at 0x0753a!");
    println!("3. We execute text data as code, leading to garbage instructions");
    
    // Wait, let me check if the branch offset is correct
    println!("\nDouble-checking branch calculation:");
    
    // inc_chk is 05 42 3e 66
    // 05 = opcode (inc_chk)
    // 42 = variable to increment (0x42)
    // 3e = value to check against (0x3e = 62)
    // 66 = branch byte
    
    let branch_byte = bytes[3]; // 0x66
    println!("Branch byte: 0x{:02x} = {:08b}", branch_byte, branch_byte);
    println!("  Bit 7 (on_true): {}", (branch_byte & 0x80) >> 7);
    println!("  Bit 6 (format): {}", (branch_byte & 0x40) >> 6);
    
    if (branch_byte & 0x40) != 0 {
        // Short form
        let offset = (branch_byte & 0x3F) as i16;
        println!("  Short form offset: {} (6-bit unsigned)", offset);
        
        let pc_after = 0x0751f + 4;
        let target = (pc_after as i32 + offset as i32 - 2) as u32;
        println!("  Target: 0x{:05x} + {} - 2 = 0x{:05x}", pc_after, offset, target);
        
        if target == 0x07547 {
            println!("  Confirmed: branches to 0x07547");
        }
    }
    
    // So the branch is correct. The question is: why is there code
    // that branches into the middle of a string?
    
    println!("\nPossible explanations:");
    println!("1. The print_ret at 0x0753a is not really there");
    println!("2. The code at 0x0751f is not meant to be executed");
    println!("3. There's dynamic code modification happening");
    
    // Let's check what calls to this area
    println!("\nThe branch to 0x0751f comes from JG at 0x093bb");
    println!("That instruction does: jg #0011, #003b [FALSE -7839]");
    println!("It compares 17 > 59, which is FALSE");
    println!("So it takes the branch to 0x0751f");
    
    println!("\nBUT: Maybe the values being compared are wrong?");
    
    Ok(())
}