use gruesome::vm::Game;
use gruesome::disasm_txd::TxdDisassembler;
use log::info;
use std::collections::HashSet;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <game_file>", args[0]);
        std::process::exit(1);
    }
    
    let memory = fs::read(&args[1])?;
    let game = Game::from_memory(memory.clone())?;
    
    // First, get all routines found by code flow
    let mut disasm = TxdDisassembler::new(&game);
    disasm.discover_routines()?;
    let code_flow_routines: HashSet<u32> = disasm.get_routine_addresses().into_iter().collect();
    
    info!("Found {} routines via code flow", code_flow_routines.len());
    
    // Now scan for data-referenced routines
    let data_routines = find_data_referenced_routines(&game, &code_flow_routines);
    
    info!("\n=== DATA-REFERENCED ROUTINES ===");
    info!("Found {} additional routines referenced in data structures", data_routines.len());
    
    let mut sorted: Vec<u32> = data_routines.into_iter().collect();
    sorted.sort();
    
    for addr in sorted {
        info!("  {:05x}", addr);
    }
    
    Ok(())
}

fn find_data_referenced_routines(game: &Game, known_routines: &HashSet<u32>) -> HashSet<u32> {
    let mut found = HashSet::new();
    
    // 1. Scan object properties
    info!("\n=== Scanning Object Properties ===");
    let obj_routines = scan_object_properties(game, known_routines);
    info!("  Found {} routines in object properties", obj_routines.len());
    found.extend(obj_routines);
    
    // 2. Scan global variables  
    info!("\n=== Scanning Global Variables ===");
    let global_routines = scan_global_variables(game, known_routines);
    info!("  Found {} routines in global variables", global_routines.len());
    found.extend(global_routines);
    
    // 3. Conservative memory scan
    info!("\n=== Conservative Memory Scan ===");
    let mem_routines = scan_memory_for_routines(game, known_routines);
    info!("  Found {} potential routines in memory scan", mem_routines.len());
    found.extend(mem_routines);
    
    found
}

fn scan_object_properties(game: &Game, known_routines: &HashSet<u32>) -> HashSet<u32> {
    let mut found = HashSet::new();
    
    // Get object table location
    let obj_table_addr = ((game.memory[0x0A] as u16) << 8) | (game.memory[0x0B] as u16);
    
    // Property defaults table size
    let prop_defaults_size = if game.header.version <= 3 { 31 * 2 } else { 63 * 2 };
    
    // Skip property defaults to get to object entries
    let objects_start = obj_table_addr as usize + prop_defaults_size;
    
    // Object entry size
    let obj_size = if game.header.version <= 3 { 9 } else { 14 };
    
    // Scan through objects (we don't know exact count, so be conservative)
    for obj_num in 1..=255 {
        let obj_addr = objects_start + (obj_num - 1) * obj_size;
        
        if obj_addr + obj_size > game.memory.len() {
            break;
        }
        
        // Get property table address (last 2 bytes of object entry)
        let prop_addr = if game.header.version <= 3 {
            let offset = obj_addr + 7;
            ((game.memory[offset] as u16) << 8) | (game.memory[offset + 1] as u16)
        } else {
            let offset = obj_addr + 12;
            ((game.memory[offset] as u16) << 8) | (game.memory[offset + 1] as u16)
        };
        
        if prop_addr == 0 || prop_addr as usize >= game.memory.len() {
            continue;
        }
        
        // Scan properties for routine references
        scan_property_table(&game.memory, prop_addr as usize, game.header.version, &mut found, known_routines);
    }
    
    found
}

fn scan_property_table(memory: &[u8], mut addr: usize, version: u8, found: &mut HashSet<u32>, known_routines: &HashSet<u32>) {
    // Skip object name
    if addr >= memory.len() {
        return;
    }
    
    let name_len = memory[addr] as usize;
    addr += 1 + name_len * 2;
    
    // Scan properties
    while addr < memory.len() {
        if version <= 3 {
            let size_byte = memory[addr];
            if size_byte == 0 {
                break;
            }
            
            let prop_size = ((size_byte >> 5) + 1) as usize;
            addr += 1;
            
            // Check property data for routine addresses
            if prop_size >= 2 && addr + 2 <= memory.len() {
                let word = ((memory[addr] as u16) << 8) | (memory[addr + 1] as u16);
                if let Some(routine_addr) = unpack_routine_address(word, version) {
                    if is_valid_routine_header(memory, routine_addr, version) && !known_routines.contains(&routine_addr) {
                        found.insert(routine_addr);
                    }
                }
            }
            
            addr += prop_size;
        } else {
            // V4+ property format
            if addr >= memory.len() {
                break;
            }
            
            let first_byte = memory[addr];
            if first_byte == 0 {
                break;
            }
            
            let (prop_size, size_bytes) = if (first_byte & 0x80) != 0 {
                // Two size bytes
                if addr + 1 >= memory.len() {
                    break;
                }
                let size = (memory[addr + 1] & 0x3F) as usize;
                (if size == 0 { 64 } else { size }, 2)
            } else {
                // One size byte
                (if (first_byte & 0x40) != 0 { 2 } else { 1 }, 1)
            };
            
            addr += size_bytes;
            
            // Check property data for routine addresses
            if prop_size >= 2 && addr + 2 <= memory.len() {
                let word = ((memory[addr] as u16) << 8) | (memory[addr + 1] as u16);
                if let Some(routine_addr) = unpack_routine_address(word, version) {
                    if is_valid_routine_header(memory, routine_addr, version) && !known_routines.contains(&routine_addr) {
                        found.insert(routine_addr);
                    }
                }
            }
            
            addr += prop_size;
        }
    }
}

fn scan_global_variables(game: &Game, known_routines: &HashSet<u32>) -> HashSet<u32> {
    let mut found = HashSet::new();
    
    // Get global variables table location
    let globals_addr = ((game.memory[0x0C] as u16) << 8) | (game.memory[0x0D] as u16);
    
    // Scan all 240 global variables
    for i in 0..240 {
        let addr = globals_addr as usize + i * 2;
        if addr + 2 > game.memory.len() {
            break;
        }
        
        let word = ((game.memory[addr] as u16) << 8) | (game.memory[addr + 1] as u16);
        if let Some(routine_addr) = unpack_routine_address(word, game.header.version) {
            if is_valid_routine_header(&game.memory, routine_addr, game.header.version) && !known_routines.contains(&routine_addr) {
                found.insert(routine_addr);
                info!("  Found routine {:05x} in global variable {}", routine_addr, i + 16);
            }
        }
    }
    
    found
}

fn scan_memory_for_routines(game: &Game, known_routines: &HashSet<u32>) -> HashSet<u32> {
    let mut found = HashSet::new();
    
    // Scan memory looking for words that could be packed routine addresses
    // Start after header and be conservative
    let start = 0x40;
    let end = game.memory.len().saturating_sub(2);
    
    for i in start..end {
        let word = ((game.memory[i] as u16) << 8) | (game.memory[i + 1] as u16);
        
        if word == 0 {
            continue;
        }
        
        if let Some(routine_addr) = unpack_routine_address(word, game.header.version) {
            if is_valid_routine_header(&game.memory, routine_addr, game.header.version) && !known_routines.contains(&routine_addr) {
                // Additional validation: check if this looks like a real routine
                if looks_like_routine(&game.memory, routine_addr, game.header.version) {
                    found.insert(routine_addr);
                }
            }
        }
    }
    
    found
}

fn unpack_routine_address(packed: u16, version: u8) -> Option<u32> {
    if packed == 0 {
        return None;
    }
    
    let addr = match version {
        1..=3 => (packed as u32) * 2,
        4..=5 => (packed as u32) * 4,
        6..=7 => (packed as u32) * 4, // Simplified
        8 => (packed as u32) * 8,
        _ => return None,
    };
    
    Some(addr)
}

fn is_valid_routine_header(memory: &[u8], addr: u32, version: u8) -> bool {
    let addr = addr as usize;
    if addr >= memory.len() {
        return false;
    }
    
    let locals = memory[addr];
    if locals > 15 {
        return false;
    }
    
    // Make sure we have enough room for the header
    let header_size = 1 + if version <= 4 { locals as usize * 2 } else { 0 };
    addr + header_size <= memory.len()
}

fn looks_like_routine(memory: &[u8], addr: u32, version: u8) -> bool {
    // Additional heuristics to reduce false positives
    let addr = addr as usize;
    
    // Check if first instruction after header looks valid
    let locals = memory[addr];
    let code_start = addr + 1 + if version <= 4 { locals as usize * 2 } else { 0 };
    
    if code_start >= memory.len() {
        return false;
    }
    
    // Very basic check: first byte should be a valid opcode form
    let first_byte = memory[code_start];
    
    // Check for common instruction patterns
    match first_byte {
        0x00..=0x1F => false,  // Too low for 2OP
        0x20..=0x7F => true,   // 2OP
        0x80..=0xBF => true,   // 1OP
        0xC0..=0xDF => true,   // 2OP/VAR
        0xE0..=0xFF => true,   // VAR
    }
}