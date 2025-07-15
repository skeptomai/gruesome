use infocom::vm::Game;
use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Checking Address 0x486e ===\n");

    // Load Zork 1
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    
    // Print header info
    println!("Game version: {}", game.header.version);
    println!("Initial PC (code start): 0x{:04x}", game.header.initial_pc);
    println!("High memory start: 0x{:04x}", game.header.base_high_mem);
    println!("Static memory base: 0x{:04x}", game.header.base_static_mem);
    println!();
    
    // Check the packed address
    let packed_addr = 0x486e_u16;
    let unpacked_addr = (packed_addr as u32) * 2;
    
    println!("Packed address: 0x{:04x}", packed_addr);
    println!("Unpacked address: 0x{:05x}", unpacked_addr);
    println!();
    
    // Check if this is a valid code address
    if unpacked_addr < game.header.initial_pc as u32 {
        println!("ERROR: Unpacked address 0x{:05x} is BELOW code start 0x{:04x}!", 
                unpacked_addr, game.header.initial_pc);
        println!("This cannot be a valid routine address!");
    } else {
        println!("Address is above code start.");
    }
    
    // Check what's at the unpacked address
    if (unpacked_addr as usize) < game.memory.len() {
        println!("\nMemory at 0x{:05x}:", unpacked_addr);
        
        // Show raw bytes
        print!("Raw bytes: ");
        for i in 0..16 {
            let addr = unpacked_addr as usize + i;
            if addr < game.memory.len() {
                print!("{:02x} ", game.memory[addr]);
            }
        }
        println!();
        
        // Try to decode as a routine header
        let first_byte = game.memory[unpacked_addr as usize];
        println!("\nIf this were a routine:");
        println!("  Number of locals: {} {}", first_byte,
                if first_byte > 15 { "(INVALID - max is 15!)" } else { "" });
        
        if first_byte <= 15 {
            // Show local variable initial values
            println!("  Local initial values:");
            for i in 0..first_byte {
                let addr = unpacked_addr as usize + 1 + (i as usize * 2);
                if addr + 1 < game.memory.len() {
                    let value = ((game.memory[addr] as u16) << 8) | (game.memory[addr + 1] as u16);
                    println!("    Local {}: 0x{:04x}", i, value);
                }
            }
            
            // Try to decode first instruction
            let code_start = unpacked_addr as usize + 1 + (first_byte as usize * 2);
            if code_start < game.memory.len() {
                println!("\n  First instruction would be at: 0x{:05x}", code_start);
                if let Ok(inst) = Instruction::decode(&game.memory, code_start, game.header.version) {
                    println!("  First instruction: {}", inst.format_with_version(game.header.version));
                } else {
                    println!("  ERROR: Cannot decode instruction at 0x{:05x}", code_start);
                }
            }
        }
    } else {
        println!("ERROR: Address 0x{:05x} is beyond memory bounds!", unpacked_addr);
    }
    
    // Search for references to this packed address
    println!("\n=== Searching for references to packed address 0x{:04x} ===", packed_addr);
    
    let search_bytes = [(packed_addr >> 8) as u8, (packed_addr & 0xFF) as u8];
    let mut found_refs = Vec::new();
    
    for i in 0..game.memory.len() - 1 {
        if game.memory[i] == search_bytes[0] && game.memory[i + 1] == search_bytes[1] {
            found_refs.push(i);
        }
    }
    
    println!("Found {} potential references:", found_refs.len());
    for &addr in &found_refs {
        println!("  At 0x{:05x}", addr);
        
        // Try to decode instruction at various offsets before this
        for offset in 1..=5 {
            if addr >= offset {
                let inst_addr = addr - offset;
                if let Ok(inst) = Instruction::decode(&game.memory, inst_addr, game.header.version) {
                    // Check if this instruction would include our address
                    if inst_addr + inst.size > addr {
                        println!("    -> Part of instruction at 0x{:05x}: {}", 
                                inst_addr, inst.format_with_version(game.header.version));
                        
                        // Check if it's a call instruction
                        if inst.opcode == 0x01 || inst.opcode == 0x05 || 
                           inst.opcode == 0x06 || inst.opcode == 0x07 || inst.opcode == 0x08 {
                            println!("       This is a CALL instruction!");
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}