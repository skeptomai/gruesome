use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Analyzing the JE instruction at 0x07547...\n");
    
    // The je at 0x07547
    if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, 0x07547, 3) {
        println!("Instruction: {}", inst.format_with_version(3));
        
        if let Some(ref branch) = inst.branch {
            println!("\nBranch details:");
            println!("  on_true: {}", branch.on_true);
            println!("  offset: {} (0x{:04x})", branch.offset, branch.offset as u16);
            
            let pc_after = 0x07547 + inst.size;
            let target = (pc_after as i32 + branch.offset as i32 - 2) as u32;
            
            println!("  PC after instruction: 0x{:05x}", pc_after);
            println!("  Branch target: 0x{:05x}", target);
            
            if branch.offset < 0 {
                println!("  This is a backward branch!");
            }
        }
        
        println!("\nThe instruction tests if 0xAE == 0x31");
        println!("  0xAE = 174");
        println!("  0x31 = 49");
        println!("  174 == 49? FALSE");
        
        println!("\nSince the condition is FALSE and on_true=true,");
        println!("we DON'T take the branch, so we continue to 0x0754c");
    }
    
    // But wait, let's check the execution trace from the inc_chk
    println!("\n\nChecking the flow from inc_chk at 0x0751f:");
    
    if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, 0x0751f, 3) {
        println!("0x0751f: {}", inst.format_with_version(3));
        
        if let Some(ref branch) = inst.branch {
            println!("  Branch on FALSE to 0x07547");
            println!("  This checks if 0x42 (66) >= 0x3E (62)");
            println!("  If the inc results in value >= 62, branch");
        }
    }
    
    // The real question is: should we be in this code at all?
    println!("\n\nThe real issue:");
    println!("We're executing code in the 0x07500-0x07560 range");
    println!("But earlier analysis showed this contains a print_ret string");
    println!("This suggests we're executing data as code!");
    
    // Let's verify - what's the string in the print_ret?
    println!("\nChecking the print_ret at 0x0753a:");
    let text_start = 0x0753b; // After the opcode byte
    let mut text_bytes = Vec::new();
    let mut addr = text_start;
    
    // Z-strings end when top bit is set
    loop {
        if addr + 1 >= game_data.len() {
            break;
        }
        let word = ((game_data[addr] as u16) << 8) | (game_data[addr + 1] as u16);
        text_bytes.push(word);
        addr += 2;
        
        if (word & 0x8000) != 0 {
            break; // End of string
        }
        
        if text_bytes.len() > 20 {
            break; // Safety limit
        }
    }
    
    println!("  Text data (words): {:?}", text_bytes);
    println!("  Text ends at: 0x{:05x}", addr);
    
    println!("\nThe bytes we're executing as code (0x07547-0x07554) are");
    println!("actually part of the encoded text string!");
    
    Ok(())
}