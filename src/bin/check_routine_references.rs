use gruesome::vm::Game;
use log::info;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <game_file> <routine_addr>", args[0]);
        eprintln!("Example: {} game.z4 e114", args[0]);
        std::process::exit(1);
    }

    let game_file = &args[1];
    let routine_addr = u32::from_str_radix(&args[2], 16)?;

    let memory = fs::read(game_file)?;
    let game = Game::from_memory(memory.clone())?;

    info!(
        "=== CHECKING REFERENCES TO ROUTINE {:05x} ===",
        routine_addr
    );

    // Check if it's referenced in SREAD calls (timer callbacks)
    check_sread_references(&memory, routine_addr, game.header.version);

    // Check object properties for references
    check_object_property_references(&game, routine_addr);

    // Check if packed address appears anywhere in memory
    check_packed_references(&memory, routine_addr, game.header.version);

    Ok(())
}

fn check_sread_references(memory: &[u8], routine_addr: u32, version: u8) {
    info!("\n=== SREAD Timer References ===");

    // SREAD is VAR opcode 0x04
    // Look for patterns: E0 04 <operands> where 4th operand might be our routine
    let packed_addr = pack_routine_address(routine_addr, version);
    info!(
        "Looking for packed address: {:04x} (unpacked: {:05x})",
        packed_addr, routine_addr
    );

    let mut found = false;
    for i in 0..memory.len() - 4 {
        // Check for VAR form SREAD (0xE0 = 1110 0000 = VAR form with 4 operands)
        if memory[i] == 0xE0 && memory[i + 1] == 0x04 {
            // This might be SREAD with 4 operands
            // The 4th operand would be at different positions depending on operand types
            info!("  Found potential SREAD at {:05x}", i);
            found = true;
        }
    }

    if !found {
        info!("  No SREAD instructions with timers found");
    }
}

fn check_object_property_references(game: &Game, routine_addr: u32) {
    info!("\n=== Object Property References ===");

    let packed_addr = pack_routine_address(routine_addr, game.header.version);

    // Get object table location
    let obj_table_addr = ((game.memory[0x0A] as u16) << 8) | (game.memory[0x0B] as u16);
    info!("Object table at: {:04x}", obj_table_addr);

    // For V4+, we need to check properties that might contain routine addresses
    // Properties can contain packed routine addresses

    let mut found_count = 0;

    // Scan memory for the packed address in areas that look like properties
    for i in obj_table_addr as usize..game.memory.len() - 2 {
        let word = ((game.memory[i] as u16) << 8) | (game.memory[i + 1] as u16);
        if word == packed_addr {
            info!("  Found packed address {:04x} at {:05x}", packed_addr, i);
            found_count += 1;
        }
    }

    if found_count == 0 {
        info!("  Routine not found in object properties");
    } else {
        info!("  Found {} potential references", found_count);
    }
}

fn check_packed_references(memory: &[u8], routine_addr: u32, version: u8) {
    info!("\n=== All Packed Address References ===");

    let packed_addr = pack_routine_address(routine_addr, version);
    let mut locations = Vec::new();

    // Search for the packed address as a 16-bit word
    for i in 0..memory.len() - 2 {
        let word = ((memory[i] as u16) << 8) | (memory[i + 1] as u16);
        if word == packed_addr {
            locations.push(i);
        }
    }

    if locations.is_empty() {
        info!(
            "  Packed address {:04x} not found anywhere in memory",
            packed_addr
        );
        info!("  This routine is likely unreferenced dead code");
    } else {
        info!(
            "  Packed address {:04x} found at {} locations:",
            packed_addr,
            locations.len()
        );
        for (idx, &loc) in locations.iter().enumerate() {
            if idx < 10 {
                info!(
                    "    {:05x}: Context: {:02x} {:02x} [{:02x} {:02x}] {:02x} {:02x}",
                    loc,
                    if loc >= 2 { memory[loc - 2] } else { 0 },
                    if loc >= 1 { memory[loc - 1] } else { 0 },
                    memory[loc],
                    memory[loc + 1],
                    if loc + 2 < memory.len() {
                        memory[loc + 2]
                    } else {
                        0
                    },
                    if loc + 3 < memory.len() {
                        memory[loc + 3]
                    } else {
                        0
                    }
                );
            }
        }
        if locations.len() > 10 {
            info!("    ... and {} more", locations.len() - 10);
        }
    }
}

fn pack_routine_address(addr: u32, version: u8) -> u16 {
    match version {
        1..=3 => (addr / 2) as u16,
        4..=5 => (addr / 4) as u16,
        6..=7 => (addr / 4) as u16, // More complex for v6/7 but simplified here
        8 => (addr / 8) as u16,
        _ => (addr / 2) as u16,
    }
}
