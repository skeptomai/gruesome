use infocom::vm::Game;
use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Verifying Instruction Alignment ===\n");

    // Load Zork 1
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    
    println!("Checking alignment around the bad call at 0x06dfc...\n");
    
    // Try different starting points to see if we get better alignment
    for start_offset in 0..5 {
        println!("=== Starting from 0x{:05x} (offset {}) ===", 0x06df0 + start_offset, start_offset);
        
        let mut addr = 0x06df0 + start_offset;
        let mut count = 0;
        
        while addr < 0x06e10 && count < 10 {
            match Instruction::decode(&game.memory, addr, game.header.version) {
                Ok(inst) => {
                    println!("{:05x}: {} (size: {})", addr, 
                            inst.format_with_version(game.header.version), inst.size);
                    
                    // Special attention to the problematic area
                    if addr <= 0x06dfc && addr + inst.size > 0x06dfc {
                        println!("       ^--- This instruction contains 0x06dfc!");
                    }
                    
                    addr += inst.size;
                    count += 1;
                }
                Err(e) => {
                    println!("{:05x}: ERROR - {}", addr, e);
                    break;
                }
            }
        }
        println!();
    }
    
    // Now let's manually check what's at 0x06dfc
    println!("\n=== Manual check of 0x06dfc ===");
    println!("Raw bytes:");
    for i in 0..20 {
        let addr = 0x06dfc + i;
        println!("{:05x}: {:02x} {:08b}", addr, game.memory[addr], game.memory[addr]);
    }
    
    // Check if this could be in the middle of another instruction
    println!("\n=== Checking if 0x06dfc is mid-instruction ===");
    for check_addr in (0x06df0..0x06dfc).rev() {
        if let Ok(inst) = Instruction::decode(&game.memory, check_addr, game.header.version) {
            if check_addr + inst.size > 0x06dfc {
                println!("Found instruction at {:05x} that extends past 0x06dfc:", check_addr);
                println!("  {}", inst.format_with_version(game.header.version));
                println!("  Size: {} bytes", inst.size);
                println!("  Would end at: {:05x}", check_addr + inst.size);
            }
        }
    }
    
    // Let's also check what the quit routine actually looks like
    println!("\n=== Looking for actual quit handling ===");
    
    // The routine at 0x06df0 seems to be involved in quit handling
    // Let's trace back to see who calls it
    let packed_addr = (0x06df0 / 2) as u16;
    println!("Routine at 0x06df0, packed address: 0x{:04x}", packed_addr);
    
    // Let's see if the instruction stream makes more sense if we follow execution
    println!("\n=== Following execution from known good point ===");
    
    // Start from the return address in the call stack (0x05871)
    println!("Starting from return address 0x05871:");
    let mut addr = 0x05871;
    for _ in 0..10 {
        if let Ok(inst) = Instruction::decode(&game.memory, addr, game.header.version) {
            println!("{:05x}: {}", addr, inst.format_with_version(game.header.version));
            addr += inst.size;
        } else {
            break;
        }
    }
    
    Ok(())
}