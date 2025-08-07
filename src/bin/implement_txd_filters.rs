use gruesome::disasm_txd::TxdDisassembler;
use gruesome::instruction::{Instruction, InstructionForm};
use gruesome::vm::Game;
use log::info;
use std::collections::{HashMap, HashSet};

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

    // Apply TXD-style filters
    let mut filtered_routines = HashSet::new();
    let mut rejection_reasons: HashMap<u32, Vec<String>> = HashMap::new();

    // Sort for consistent processing
    let mut sorted_routines = all_routines.clone();
    sorted_routines.sort();

    // Step 1: Build accurate routine boundaries
    let mut routine_bounds: Vec<(u32, u32)> = Vec::new();
    for &addr in &sorted_routines {
        match get_routine_bounds(&game, addr) {
            Ok((start, end)) => routine_bounds.push((start, end)),
            Err(e) => {
                rejection_reasons
                    .entry(addr)
                    .or_default()
                    .push(format!("Invalid bounds: {}", e));
            }
        }
    }

    // Step 2: Build call graph
    let call_graph = build_call_graph(&game, &sorted_routines)?;
    let called_routines: HashSet<u32> = call_graph
        .values()
        .flat_map(|targets| targets.iter().cloned())
        .collect();

    // Step 3: Apply filters
    for &addr in &sorted_routines {
        let mut reasons = Vec::new();

        // Filter 1: Inside another routine
        let mut _is_nested = false;
        for &(start, end) in &routine_bounds {
            if start < addr && addr < end {
                _is_nested = true;
                reasons.push(format!("Inside routine {:05x}", start));
                break;
            }
        }

        // Filter 2: Not called by any routine
        if !called_routines.contains(&addr) && addr != game.header.initial_pc as u32 {
            reasons.push("Not called by any routine".to_string());
        }

        // Filter 3: Check routine quality
        if let Ok(quality) = check_routine_quality(&game, addr) {
            if quality.starts_with_ret_popped {
                reasons.push("Starts with ret_popped".to_string());
            }
            if quality.immediate_jump {
                reasons.push("Immediate unconditional jump".to_string());
            }
            if quality.falls_through && quality.instruction_count < 5 {
                reasons.push(format!(
                    "Falls through after {} instructions",
                    quality.instruction_count
                ));
            }
        }

        // Accept routine if no rejection reasons
        if reasons.is_empty() {
            filtered_routines.insert(addr);
        } else {
            rejection_reasons.insert(addr, reasons);
        }
    }

    // Report results
    info!("\n=== TXD-STYLE FILTERING RESULTS ===");
    info!("Original routines: {}", all_routines.len());
    info!("Filtered routines: {}", filtered_routines.len());
    info!("Rejected routines: {}", rejection_reasons.len());

    // Compare with TXD
    let txd_count = 982; // Known TXD count for AMFV
    info!("\n=== COMPARISON ===");
    info!("Our filtered count: {}", filtered_routines.len());
    info!("TXD count: {}", txd_count);
    info!("Difference: {}", filtered_routines.len() as i32 - txd_count);

    // Show rejection reason statistics
    info!("\n=== REJECTION REASONS ===");
    let mut reason_counts: HashMap<String, usize> = HashMap::new();
    for reasons in rejection_reasons.values() {
        for reason in reasons {
            *reason_counts.entry(reason.clone()).or_insert(0) += 1;
        }
    }

    let mut sorted_reasons: Vec<_> = reason_counts.into_iter().collect();
    sorted_reasons.sort_by_key(|(_, count)| -(*count as i32));

    for (reason, count) in sorted_reasons {
        info!("{}: {} routines", reason, count);
    }

    // Show some examples
    info!("\n=== REJECTION EXAMPLES ===");
    for (addr, reasons) in rejection_reasons.iter().take(10) {
        info!("{:05x}: {}", addr, reasons.join(", "));
    }

    Ok(())
}

fn get_routine_bounds(game: &Game, addr: u32) -> Result<(u32, u32), String> {
    let locals = game.memory[addr as usize];
    let mut pc = addr as usize + 1;

    if game.header.version <= 4 {
        pc += (locals as usize) * 2;
    }

    let start = addr;
    let mut instruction_count = 0;

    while pc < game.memory.len() {
        match Instruction::decode(&game.memory, pc, game.header.version) {
            Ok(inst) => {
                pc += inst.size;
                instruction_count += 1;

                // Check for routine end
                if is_routine_end(&inst) {
                    return Ok((start, pc as u32));
                }

                // Safety limit
                if instruction_count > 1000 {
                    return Ok((start, pc as u32));
                }
            }
            Err(_) => {
                // Invalid instruction marks end
                return Ok((start, pc as u32));
            }
        }
    }

    Ok((start, pc as u32))
}

fn build_call_graph(game: &Game, routines: &[u32]) -> Result<HashMap<u32, HashSet<u32>>, String> {
    let mut call_graph = HashMap::new();

    for &routine_addr in routines {
        let targets = find_call_targets(game, routine_addr)?;
        call_graph.insert(routine_addr, targets);
    }

    Ok(call_graph)
}

fn find_call_targets(game: &Game, routine_addr: u32) -> Result<HashSet<u32>, String> {
    let mut targets = HashSet::new();

    let locals = game.memory[routine_addr as usize];
    let mut pc = routine_addr as usize + 1;

    if game.header.version <= 4 {
        pc += (locals as usize) * 2;
    }

    while pc < game.memory.len() {
        match Instruction::decode(&game.memory, pc, game.header.version) {
            Ok(inst) => {
                // Check for call instructions
                let is_call = match (inst.form, inst.opcode) {
                    (InstructionForm::Variable, 0x00) | // call
                    (InstructionForm::Variable, 0x19) | // call_vn
                    (InstructionForm::Variable, 0x1a) | // call_vn2
                    (InstructionForm::Short, 0x05..=0x07) | // call_1n etc
                    (InstructionForm::Long, 0x00) => true, // Long form call (2OP)
                    _ => false,
                };

                if is_call && !inst.operands.is_empty() {
                    let packed = inst.operands[0];
                    if packed != 0 {
                        let unpacked = unpack_address(packed, game.header.version);
                        targets.insert(unpacked);
                    }
                }

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

#[derive(Debug)]
struct RoutineQuality {
    starts_with_ret_popped: bool,
    immediate_jump: bool,
    falls_through: bool,
    instruction_count: usize,
}

fn check_routine_quality(game: &Game, addr: u32) -> Result<RoutineQuality, String> {
    let mut quality = RoutineQuality {
        starts_with_ret_popped: false,
        immediate_jump: false,
        falls_through: true,
        instruction_count: 0,
    };

    let locals = game.memory[addr as usize];
    let mut pc = addr as usize + 1;

    if game.header.version <= 4 {
        pc += (locals as usize) * 2;
    }

    let mut first = true;
    while pc < game.memory.len() && quality.instruction_count < 20 {
        match Instruction::decode(&game.memory, pc, game.header.version) {
            Ok(inst) => {
                if first {
                    first = false;

                    // Check for ret_popped
                    if matches!((inst.form, inst.opcode), (InstructionForm::Short, 0x09)) {
                        quality.starts_with_ret_popped = true;
                    }

                    // Check for immediate jump
                    if matches!((inst.form, inst.opcode), (InstructionForm::Short, 0x0c)) {
                        quality.immediate_jump = true;
                    }
                }

                quality.instruction_count += 1;

                if is_routine_end(&inst) {
                    quality.falls_through = false;
                    break;
                }

                pc += inst.size;
            }
            Err(_) => break,
        }
    }

    Ok(quality)
}

fn is_routine_end(inst: &Instruction) -> bool {
    match (inst.form, inst.opcode) {
        (InstructionForm::Short, 0x00..=0x03) | // ret variants
        (InstructionForm::Short, 0x0a) | // quit
        (InstructionForm::Short, 0x0c) => true, // jump
        _ => false,
    }
}
