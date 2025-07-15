use infocom::vm::Game;
use infocom::instruction::Instruction;
use infocom::text;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Finding the Real Quit Routine ===\n");

    // Load Zork 1
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    
    println!("Looking for quit-related routines...\n");
    
    // Search for the quit opcode (0x0A in 0OP form)
    println!("1. Searching for quit opcode (0x0A):");
    let mut quit_locations = Vec::new();
    
    for addr in 0x4e38..game.memory.len() {
        if game.memory[addr] == 0x8A {  // 0OP form of quit (10001010)
            quit_locations.push(addr);
            println!("   Found quit opcode at: 0x{:05x}", addr);
        }
    }
    
    // Look at the code around 0x06e07 which we saw has a quit opcode
    println!("\n2. Code around 0x06e07 (where we saw quit opcode):");
    for addr in 0x06e00..=0x06e10 {
        if let Ok(inst) = Instruction::decode(&game.memory, addr, game.header.version) {
            println!("   {:05x}: {}", addr, inst.format_with_version(game.header.version));
        }
    }
    
    // Search for strings related to quitting
    println!("\n3. Searching for quit-related strings:");
    let abbrev_addr = game.header.abbrev_table as usize;
    
    // Look for "Do you wish to leave the game?"
    for addr in 0x4e38..game.memory.len() - 10 {
        if let Ok((text, _)) = text::decode_string(&game.memory, addr, abbrev_addr) {
            if text.contains("Do you wish") || text.contains("leave the game") {
                println!("   Found at 0x{:05x}: {}", addr, text);
                
                // Look for references to this address
                let packed = (addr / 2) as u16;
                println!("   Packed address would be: 0x{:04x}", packed);
            }
        }
    }
    
    // Let's trace back from the routine at 0x06df0
    println!("\n4. Analyzing the routine containing the bad call:");
    println!("   Routine at 0x06df0 has {} locals", game.memory[0x06df0]);
    
    // Find who calls this routine
    let packed_06df0 = (0x06df0 / 2) as u16;
    println!("   Packed address: 0x{:04x}", packed_06df0);
    
    println!("\n5. Looking for calls to 0x{:04x}:", packed_06df0);
    for addr in 0x4e38..game.memory.len() - 5 {
        if let Ok(inst) = Instruction::decode(&game.memory, addr, game.header.version) {
            // Check if this is a call instruction
            if (inst.opcode == 0x00 || inst.opcode == 0x01) && !inst.operands.is_empty() {
                if inst.operands[0] == packed_06df0 {
                    println!("   Called from: 0x{:05x}", addr);
                    
                    // Show context
                    for offset in -10..10 {
                        let ctx_addr = (addr as i32 + offset * 3) as usize;
                        if ctx_addr >= 0x4e38 && ctx_addr < game.memory.len() {
                            if let Ok(ctx_inst) = Instruction::decode(&game.memory, ctx_addr, game.header.version) {
                                println!("     {:05x}: {}", ctx_addr, ctx_inst.format_with_version(game.header.version));
                            }
                        }
                    }
                }
            }
        }
    }
    
    println!("\n6. The mystery:");
    println!("   - The code at 0x06dfc calls 0x486e");
    println!("   - 0x486e unpacks to 0x090dc");
    println!("   - But 0x090dc contains what looks like inline code, not a routine");
    println!("   - This suggests the call address might be calculated or modified at runtime");
    
    Ok(())
}