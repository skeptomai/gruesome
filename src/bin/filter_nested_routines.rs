use gruesome::disasm_txd::TxdDisassembler;
use gruesome::vm::Game;
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

    // Sort routines by address for efficient filtering
    let mut sorted_routines = all_routines.clone();
    sorted_routines.sort();

    // Build routine boundaries
    let mut routine_bounds: Vec<(u32, u32)> = Vec::new();
    for &addr in &sorted_routines {
        let end = estimate_routine_end(&game, addr)?;
        routine_bounds.push((addr, end));
    }

    // Filter out routines that are inside other routines
    let mut filtered_routines = HashSet::new();
    let mut nested_routines = Vec::new();

    for &addr in &sorted_routines {
        let mut is_nested = false;

        // Check if this routine is inside any other routine
        for &(start, end) in &routine_bounds {
            if start < addr && addr < end {
                // This routine is inside another routine
                is_nested = true;
                nested_routines.push((addr, start));
                break;
            }
        }

        if !is_nested {
            filtered_routines.insert(addr);
        }
    }

    info!("\n=== FILTERING RESULTS ===");
    info!("Original routines: {}", all_routines.len());
    info!("Nested routines removed: {}", nested_routines.len());
    info!("Remaining routines: {}", filtered_routines.len());

    // Show some examples of nested routines
    info!("\n=== NESTED ROUTINE EXAMPLES ===");
    for (nested, parent) in nested_routines.iter().take(10) {
        let offset = nested - parent;
        info!(
            "  {:05x} is inside {:05x} at offset +{}",
            nested, parent, offset
        );
    }

    // Compare with TXD count
    let txd_count = 982; // Known TXD count for AMFV
    info!("\n=== COMPARISON ===");
    info!("After filtering: {} routines", filtered_routines.len());
    info!("TXD finds: {} routines", txd_count);
    info!(
        "Difference: {} routines",
        filtered_routines.len() as i32 - txd_count
    );

    Ok(())
}

fn estimate_routine_end(game: &Game, addr: u32) -> Result<u32, String> {
    use gruesome::instruction::{Instruction, InstructionForm};

    let locals = game.memory[addr as usize];
    let mut pc = addr as usize + 1;

    if game.header.version <= 4 {
        pc += (locals as usize) * 2;
    }

    let mut instruction_count = 0;
    let max_size = 10000; // Maximum routine size to prevent runaway

    while pc < game.memory.len() && (pc - addr as usize) < max_size {
        match Instruction::decode(&game.memory, pc, game.header.version) {
            Ok(inst) => {
                pc += inst.size;
                instruction_count += 1;

                // Check for routine end
                match (inst.form, inst.opcode) {
                    // ret, rtrue, rfalse, ret_popped
                    (InstructionForm::Short, 0x00..=0x03) |
                    // quit
                    (InstructionForm::Short, 0x0a) |
                    // jump (unconditional)
                    (InstructionForm::Short, 0x0c) => {
                        return Ok(pc as u32);
                    }
                    _ => {}
                }

                // Safety check for very long routines
                if instruction_count > 1000 {
                    return Ok(pc as u32);
                }
            }
            Err(_) => {
                // Hit invalid instruction, routine probably ends here
                return Ok(pc as u32);
            }
        }
    }

    Ok(pc as u32)
}
