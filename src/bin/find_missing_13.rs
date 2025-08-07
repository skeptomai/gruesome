use gruesome::vm::Game;
use log::info;
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

    // The 13 legitimate data-referenced routines from categorize_missing
    let missing_13 = vec![
        0x12a04, 0x12b18, 0x12b38, 0x1b0d8, 0x1b980, 0x1bf3c, 0x1d854, 0x1da50, 0x1dc1c, 0x1e138,
        0x1f250, 0x20ae8, 0x2b248,
    ];

    info!("=== SEARCHING FOR THE 13 MISSING DATA-REFERENCED ROUTINES ===");

    for &routine_addr in &missing_13 {
        info!("\n=== Routine {:05x} ===", routine_addr);

        // Check where it's referenced
        find_references(&game, routine_addr);
    }

    Ok(())
}

fn find_references(game: &Game, routine_addr: u32) {
    let packed_addr = pack_routine_address(routine_addr, game.header.version);
    info!("Packed address: {:04x}", packed_addr);

    // Find all occurrences in memory
    let mut locations = Vec::new();
    for i in 0..game.memory.len() - 2 {
        let word = ((game.memory[i] as u16) << 8) | (game.memory[i + 1] as u16);
        if word == packed_addr {
            locations.push(i);
        }
    }

    if locations.is_empty() {
        info!("  NOT FOUND in memory!");
        return;
    }

    info!("  Found at {} locations:", locations.len());

    for &loc in &locations {
        // Try to determine what type of reference this is
        analyze_reference_context(game, loc, packed_addr);
    }
}

fn analyze_reference_context(game: &Game, loc: usize, _packed_addr: u16) {
    // Check if it's in the object table area
    let obj_table_addr = ((game.memory[0x0A] as u16) << 8) | (game.memory[0x0B] as u16);
    let globals_addr = ((game.memory[0x0C] as u16) << 8) | (game.memory[0x0D] as u16);

    if loc >= obj_table_addr as usize && loc < globals_addr as usize {
        info!("    {:05x}: In object table area (likely property)", loc);
        // Try to determine which object
        find_object_containing_address(game, loc as u32);
    } else if loc >= globals_addr as usize && loc < globals_addr as usize + 240 * 2 {
        let global_num = (loc - globals_addr as usize) / 2 + 16;
        info!("    {:05x}: In global variable {}", loc, global_num);
    } else {
        // Show context
        info!(
            "    {:05x}: Context: {:02x} {:02x} [{:02x} {:02x}] {:02x} {:02x}",
            loc,
            game.memory.get(loc.wrapping_sub(2)).copied().unwrap_or(0),
            game.memory.get(loc.wrapping_sub(1)).copied().unwrap_or(0),
            game.memory[loc],
            game.memory[loc + 1],
            if loc + 2 < game.memory.len() {
                game.memory[loc + 2]
            } else {
                0
            },
            if loc + 3 < game.memory.len() {
                game.memory[loc + 3]
            } else {
                0
            }
        );

        // Check if it's in a high memory area (could be grammar table)
        let static_mem = ((game.memory[0x0E] as u16) << 8) | (game.memory[0x0F] as u16);
        if loc >= static_mem as usize {
            info!("        (In static/high memory - could be grammar table)");
        }
    }
}

fn find_object_containing_address(game: &Game, addr: u32) {
    // Get object table location
    let obj_table_addr = ((game.memory[0x0A] as u16) << 8) | (game.memory[0x0B] as u16);

    // Property defaults table size
    let prop_defaults_size = if game.header.version <= 3 {
        31 * 2
    } else {
        63 * 2
    };

    // Skip property defaults to get to object entries
    let objects_start = obj_table_addr as usize + prop_defaults_size;

    // Object entry size
    let obj_size = if game.header.version <= 3 { 9 } else { 14 };

    // Search through objects
    for obj_num in 1..=1000 {
        // Be generous with object count
        let obj_addr = objects_start + (obj_num - 1) * obj_size;

        if obj_addr + obj_size > game.memory.len() {
            break;
        }

        // Get property table address
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

        // Check if our address falls within this object's property area
        if addr >= prop_addr as u32 && addr < prop_addr as u32 + 1000 {
            // Generous property size
            info!("        Likely in object {} properties", obj_num);

            // Try to determine which property
            identify_property(game, prop_addr as usize, addr as usize);
            return;
        }
    }
}

fn identify_property(game: &Game, prop_table_start: usize, target_addr: usize) {
    let mut addr = prop_table_start;

    // Skip object name
    if addr >= game.memory.len() {
        return;
    }

    let name_len = game.memory[addr] as usize;
    addr += 1 + name_len * 2;

    // Scan properties
    while addr < game.memory.len() && addr < target_addr + 100 {
        if game.header.version <= 3 {
            let size_byte = game.memory[addr];
            if size_byte == 0 {
                break;
            }

            let prop_num = size_byte & 0x1F;
            let prop_size = ((size_byte >> 5) + 1) as usize;
            addr += 1;

            if target_addr >= addr && target_addr < addr + prop_size {
                info!("        In property {} (size {})", prop_num, prop_size);
                return;
            }

            addr += prop_size;
        } else {
            // V4+ property format
            if addr >= game.memory.len() {
                break;
            }

            let first_byte = game.memory[addr];
            if first_byte == 0 {
                break;
            }

            let prop_num = first_byte & 0x3F;
            let (prop_size, size_bytes) = if (first_byte & 0x80) != 0 {
                // Two size bytes
                if addr + 1 >= game.memory.len() {
                    break;
                }
                let size = (game.memory[addr + 1] & 0x3F) as usize;
                (if size == 0 { 64 } else { size }, 2)
            } else {
                // One size byte
                (if (first_byte & 0x40) != 0 { 2 } else { 1 }, 1)
            };

            addr += size_bytes;

            if target_addr >= addr && target_addr < addr + prop_size {
                info!("        In property {} (size {})", prop_num, prop_size);
                return;
            }

            addr += prop_size;
        }
    }
}

fn pack_routine_address(addr: u32, version: u8) -> u16 {
    match version {
        1..=3 => (addr / 2) as u16,
        4..=5 => (addr / 4) as u16,
        6..=7 => (addr / 4) as u16,
        8 => (addr / 8) as u16,
        _ => (addr / 2) as u16,
    }
}
