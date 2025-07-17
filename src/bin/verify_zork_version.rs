use gruesome::vm::Game;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;
    
    let game = Game::from_memory(memory)?;
    
    println!("Zork I Version: {}", game.header.version);
    println!("Timer support starts in Version 4");
    println!();
    
    // Check if SREAD calls in Zork I actually have 4 operands
    println!("Checking SREAD operand counts in Zork I...");
    
    // The sread calls we found earlier - let's verify their operand counts
    let sread_addresses = vec![
        0x0120, 0x15b5, 0x1f58, 0x213b, 0x2739, 0x2e53,
        0x3092, 0x30ba, 0x362f, 0x3a8d, 0x44f7, 0x452f,
        0x49d6, 0x4b56, 0x5015, 0x5046, 0x6cc0, 0x6e23,
        0x703f, 0x733a, 0x7652
    ];
    
    for addr in sread_addresses {
        let byte = game.memory.get(addr).copied().unwrap_or(0);
        println!("  0x{:04x}: byte = 0x{:02x}", addr, byte);
        
        // In V3, SREAD is a 2OP instruction, not VAR
        // Check if it's actually opcode 0x04 (sread)
        if byte == 0x04 || (byte & 0x1F) == 0x04 {
            println!("    -> Looks like a potential SREAD");
        }
    }
    
    Ok(())
}