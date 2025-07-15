use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;
    
    // Look around PC 0x5491 to understand the loop
    let pc: usize = 0x5491;
    
    println!("Examining code around PC {:05x}:", pc);
    println!();
    
    // Show instructions before and after
    for addr in (pc.saturating_sub(10))..=(pc + 10) {
        print!("{:05x}: ", addr);
        for i in 0..8 {
            if addr + i < memory.len() {
                print!("{:02x} ", memory[addr + i]);
            }
        }
        if addr == pc {
            print!(" <-- Current PC");
        }
        println!();
    }
    
    println!("\nLooking for branch instructions that might jump to 0x5491...");
    
    // The ADD instruction at 0x5491 is 4 bytes: 54 94 b4 03
    // After that is 0x5495
    // Let's check what's at 0x5495
    let next_pc = 0x5495;
    println!("\nNext instruction at {:05x}:", next_pc);
    print!("  Bytes: ");
    for i in 0..8 {
        print!("{:02x} ", memory[next_pc + i]);
    }
    println!();
    
    // Check if it's a branch instruction
    let inst_byte = memory[next_pc];
    
    // Check for jump instructions
    if inst_byte == 0x8c || inst_byte == 0x8d {
        println!("  This is a jump instruction!");
        
        // For jump, the offset is the next word
        let offset = ((memory[next_pc + 1] as u16) << 8) | (memory[next_pc + 2] as u16);
        let offset_signed = offset as i16;
        
        // Jump is relative to the address after the jump instruction
        let jump_from = next_pc + 3; // 1 byte opcode + 2 byte offset
        let jump_target = (jump_from as i32 + offset_signed as i32 - 2) as u32;
        
        println!("  Jump offset: {} (0x{:04x})", offset_signed, offset);
        println!("  Jump from: 0x{:05x}", jump_from);
        println!("  Jump target: 0x{:05x}", jump_target);
        
        if jump_target == 0x5491 {
            println!("  >>> This jumps back to 0x5491, creating a loop!");
        }
    }
    
    // Also check for conditional branches (they have form bits)
    if inst_byte >= 0x60 && inst_byte <= 0x7F {
        println!("  This might be a conditional branch (Short form)");
        
        // For short form branches, bottom 6 bits contain offset
        let offset = inst_byte & 0x3F;
        let condition_sense = (inst_byte & 0x80) != 0;
        
        println!("  Branch condition: {}", if condition_sense { "true" } else { "false" });
        println!("  Short offset: {}", offset);
    }
    
    Ok(())
}