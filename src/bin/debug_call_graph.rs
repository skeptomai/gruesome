use gruesome::disasm_txd::TxdDisassembler;
use gruesome::instruction::{Instruction, InstructionForm};
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

    info!("Found {} routines", all_routines.len());

    // Build comprehensive call graph
    let mut total_calls = 0;
    let mut routines_with_calls = 0;
    let mut called_routines = HashSet::new();

    for &routine_addr in &all_routines {
        let targets = find_all_call_targets(&game, routine_addr)?;
        if !targets.is_empty() {
            routines_with_calls += 1;
            total_calls += targets.len();
            for target in targets {
                called_routines.insert(target);
            }
        }
    }

    info!("\n=== CALL GRAPH STATISTICS ===");
    info!("Total routines: {}", all_routines.len());
    info!("Routines that make calls: {}", routines_with_calls);
    info!("Total call targets found: {}", total_calls);
    info!("Unique routines called: {}", called_routines.len());
    info!(
        "Uncalled routines: {}",
        all_routines.len() - called_routines.len()
    );

    // Check some specific routines
    info!("\n=== SAMPLE ROUTINE ANALYSIS ===");
    for &addr in all_routines.iter().take(10) {
        let targets = find_all_call_targets(&game, addr)?;
        info!("Routine {:05x}: {} calls", addr, targets.len());
        for target in targets.iter().take(3) {
            info!("  -> {:05x}", target);
        }
    }

    // Find entry points (called but not in our routine list)
    info!("\n=== POTENTIAL MISSING ROUTINES ===");
    let routine_set: HashSet<u32> = all_routines.iter().cloned().collect();
    let mut missing_targets: Vec<u32> = called_routines.difference(&routine_set).cloned().collect();
    missing_targets.sort();

    info!(
        "Found {} call targets not in our routine list:",
        missing_targets.len()
    );
    for addr in missing_targets.iter().take(20) {
        info!("  {:05x}", addr);
    }

    Ok(())
}

fn find_all_call_targets(game: &Game, routine_addr: u32) -> Result<HashSet<u32>, String> {
    let mut targets = HashSet::new();

    let locals = game.memory[routine_addr as usize];
    let mut pc = routine_addr as usize + 1;

    if game.header.version <= 4 {
        pc += (locals as usize) * 2;
    }

    while pc < game.memory.len() {
        match Instruction::decode(&game.memory, pc, game.header.version) {
            Ok(inst) => {
                // Check all forms of call instructions
                match (inst.form, inst.opcode) {
                    // Variable form calls
                    (InstructionForm::Variable, 0x00) => {
                        // call/call_vs/call_vs2
                        if !inst.operands.is_empty() && inst.operands[0] != 0 {
                            let unpacked = unpack_address(inst.operands[0], game.header.version);
                            targets.insert(unpacked);
                        }
                    }
                    (InstructionForm::Variable, 0x19) => {
                        // call_vn
                        if !inst.operands.is_empty() && inst.operands[0] != 0 {
                            let unpacked = unpack_address(inst.operands[0], game.header.version);
                            targets.insert(unpacked);
                        }
                    }
                    (InstructionForm::Variable, 0x1a) => {
                        // call_vn2
                        if !inst.operands.is_empty() && inst.operands[0] != 0 {
                            let unpacked = unpack_address(inst.operands[0], game.header.version);
                            targets.insert(unpacked);
                        }
                    }
                    // Short form calls
                    (InstructionForm::Short, 0x05) => {
                        // call_1n
                        if !inst.operands.is_empty() && inst.operands[0] != 0 {
                            let unpacked = unpack_address(inst.operands[0], game.header.version);
                            targets.insert(unpacked);
                        }
                    }
                    (InstructionForm::Short, 0x06) => {
                        // call_2n (v5+)
                        if game.header.version >= 5
                            && !inst.operands.is_empty()
                            && inst.operands[0] != 0
                        {
                            let unpacked = unpack_address(inst.operands[0], game.header.version);
                            targets.insert(unpacked);
                        }
                    }
                    (InstructionForm::Short, 0x07) => {
                        // call_2s (v4+)
                        if game.header.version >= 4
                            && !inst.operands.is_empty()
                            && inst.operands[0] != 0
                        {
                            let unpacked = unpack_address(inst.operands[0], game.header.version);
                            targets.insert(unpacked);
                        }
                    }
                    // Long form call (2OP)
                    (InstructionForm::Long, 0x00) => {
                        // 2OP call (older name for call_2s)
                        if !inst.operands.is_empty() && inst.operands[0] != 0 {
                            let unpacked = unpack_address(inst.operands[0], game.header.version);
                            targets.insert(unpacked);
                        }
                    }
                    _ => {}
                }

                // Check for routine end
                if is_routine_end(&inst) {
                    break;
                }

                pc += inst.size;
            }
            Err(_) => break,
        }
    }

    Ok(targets)
}

fn unpack_address(packed: u16, version: u8) -> u32 {
    match version {
        1..=3 => (packed as u32) * 2,
        4..=5 => (packed as u32) * 4,
        6..=7 => (packed as u32) * 4, // Simplified
        8 => (packed as u32) * 8,
        _ => 0,
    }
}

fn is_routine_end(inst: &Instruction) -> bool {
    match (inst.form, inst.opcode) {
        (InstructionForm::Short, 0x00..=0x03) | // ret variants
        (InstructionForm::Short, 0x0a) | // quit
        (InstructionForm::Short, 0x0c) => true, // jump (unconditional)
        _ => false,
    }
}
