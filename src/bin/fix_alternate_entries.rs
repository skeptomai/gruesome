use gruesome::vm::Game;
use gruesome::disasm_txd::TxdDisassembler;
use log::info;
use std::collections::HashSet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <game_file>", args[0]);
        std::process::exit(1);
    }
    
    let game_file = &args[1];
    
    // Run our disassembler
    let memory = std::fs::read(game_file)?;
    let game = Game::from_memory(memory)?;
    let mut disasm = TxdDisassembler::new(&game);
    
    disasm.discover_routines()?;
    let all_routines: Vec<u32> = disasm.get_routine_addresses();
    
    info!("Found {} routines initially", all_routines.len());
    
    // Sort routines for analysis
    let mut sorted_routines = all_routines.clone();
    sorted_routines.sort();
    
    // Find alternate entry points
    let mut alternate_entries = Vec::new();
    let mut real_routines = HashSet::new();
    
    for i in 0..sorted_routines.len() {
        let addr = sorted_routines[i];
        let locals = game.memory[addr as usize];
        let header_size = 1 + if game.header.version <= 4 {
            (locals as usize) * 2
        } else {
            0
        };
        
        // Check if this routine starts inside another routine's header/locals
        let mut is_alternate = false;
        for j in 0..i {
            let other_addr = sorted_routines[j];
            let other_locals = game.memory[other_addr as usize];
            let other_header_size = 1 + if game.header.version <= 4 {
                (other_locals as usize) * 2
            } else {
                0
            };
            
            // Check if addr is inside other's header area
            if addr > other_addr && addr < other_addr + other_header_size as u32 {
                is_alternate = true;
                alternate_entries.push((addr, other_addr));
                info!("  {:05x} is alternate entry to {:05x} (offset +{})", 
                     addr, other_addr, addr - other_addr);
                break;
            }
        }
        
        if !is_alternate {
            real_routines.insert(addr);
        }
    }
    
    info!("\n=== ALTERNATE ENTRY POINT ANALYSIS ===");
    info!("Found {} alternate entry points", alternate_entries.len());
    info!("Real routines: {}", real_routines.len());
    
    // Compare with TXD
    let txd_count = 982; // Known TXD count for AMFV
    info!("\n=== COMPARISON ===");
    info!("After removing alternates: {} routines", real_routines.len());
    info!("TXD finds: {} routines", txd_count);
    info!("Difference: {} routines", real_routines.len() as i32 - txd_count);
    
    // Show all alternate entries found
    info!("\n=== ALL ALTERNATE ENTRIES ===");
    for (alt, parent) in &alternate_entries {
        let alt_locals = game.memory[*alt as usize];
        let parent_locals = game.memory[*parent as usize];
        info!("  {:05x} (locals={}) inside {:05x} (locals={}) at offset +{}", 
             alt, alt_locals, parent, parent_locals, alt - parent);
        
        // Show why this is suspicious
        let parent_header_size = 1 + if game.header.version <= 4 {
            (parent_locals as usize) * 2
        } else {
            0
        };
        
        if *alt < parent + parent_header_size as u32 {
            info!("    ^ Inside header/locals area!");
        }
    }
    
    Ok(())
}