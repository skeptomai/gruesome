use gruesome::disasm_txd::TxdDisassembler;
use gruesome::instruction::{Instruction, InstructionForm};
use gruesome::vm::Game;
use log::info;
use std::collections::HashSet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <game_file> <txd_routines.txt>", args[0]);
        std::process::exit(1);
    }

    let game_file = &args[1];
    let txd_file = &args[2];

    // Load TXD routines
    let txd_content = std::fs::read_to_string(txd_file)?;
    let txd_routines: HashSet<u32> = txd_content
        .lines()
        .filter_map(|line| u32::from_str_radix(line.trim(), 16).ok())
        .collect();

    info!("Loaded {} routines from TXD", txd_routines.len());

    // Run our disassembler
    let memory = std::fs::read(game_file)?;
    let game = Game::from_memory(memory)?;
    let mut disasm = TxdDisassembler::new(&game);

    disasm.discover_routines()?;
    let our_routines: HashSet<u32> = disasm.get_routine_addresses().into_iter().collect();

    info!("We found {} routines", our_routines.len());

    // Find extras
    let extras: HashSet<u32> = our_routines.difference(&txd_routines).cloned().collect();

    info!("\n=== TXD HEURISTIC ANALYSIS ===\n");
    info!(
        "Checking why TXD might reject our {} extra routines:\n",
        extras.len()
    );

    // Check each extra routine for TXD-specific patterns
    let mut rejected_patterns = std::collections::HashMap::new();

    for &addr in &extras {
        let reasons = analyze_txd_rejection_reasons(&game, addr, &our_routines)?;
        for reason in reasons {
            *rejected_patterns.entry(reason).or_insert(0) += 1;
        }
    }

    // Report findings
    let mut patterns: Vec<_> = rejected_patterns.into_iter().collect();
    patterns.sort_by_key(|(_, count)| -(*count as i32));

    for (reason, count) in patterns {
        info!("{}: {} routines", reason, count);
    }

    info!("\n=== SPECIFIC EXAMPLES ===\n");

    // Show specific examples of each pattern
    let mut shown_patterns = HashSet::new();
    for &addr in extras.iter().take(10) {
        let reasons = analyze_txd_rejection_reasons(&game, addr, &our_routines)?;
        let pattern = reasons.join(", ");

        if !shown_patterns.contains(&pattern) {
            shown_patterns.insert(pattern.clone());
            info!("Address {:05x}: {}", addr, pattern);
            show_routine_context(&game, addr)?;
            info!("");
        }
    }

    Ok(())
}

fn analyze_txd_rejection_reasons(
    game: &Game,
    addr: u32,
    all_routines: &HashSet<u32>,
) -> Result<Vec<String>, String> {
    let mut reasons = Vec::new();

    // Check if this appears to be in the middle of another routine
    let inside_another = all_routines.iter().any(|&r| r < addr && addr < r + 1000);
    if inside_another {
        // Find which routine contains this
        if let Some(&container) = all_routines.iter().find(|&&r| r < addr && addr < r + 1000) {
            let offset = addr - container;
            reasons.push(format!(
                "Inside routine {:05x} at offset +{}",
                container, offset
            ));
        }
    }

    // Get locals count
    let locals = game.memory[addr as usize];
    let mut pc = addr as usize + 1;

    // Skip local variable initial values
    if game.header.version <= 4 {
        pc += (locals as usize) * 2;
    }

    // Analyze first instruction
    match Instruction::decode(&game.memory, pc, game.header.version) {
        Ok(inst) => {
            // Check for specific patterns TXD might reject

            // Pattern 1: Starts with ret_popped
            if matches!((inst.form, inst.opcode), (InstructionForm::Short, 0x09)) {
                reasons.push("Starts with ret_popped".to_string());
            }

            // Pattern 2: Very short routine that just jumps
            if matches!((inst.form, inst.opcode), (InstructionForm::Short, 0x0c)) {
                let jump_offset = inst.operands.get(0).copied().unwrap_or(0) as i16;
                reasons.push(format!("Immediate jump to offset {}", jump_offset));
            }

            // Pattern 3: Falls through without proper termination
            let mut temp_pc = pc;
            let mut inst_count = 0;
            let mut has_termination = false;

            while temp_pc < game.memory.len() && inst_count < 20 {
                match Instruction::decode(&game.memory, temp_pc, game.header.version) {
                    Ok(inst) => {
                        inst_count += 1;
                        temp_pc += inst.size;

                        // Check for proper termination
                        match (inst.form, inst.opcode) {
                            (InstructionForm::Short, 0x00..=0x03) | // ret variants
                            (InstructionForm::Short, 0x0a) | // quit
                            (InstructionForm::Short, 0x0c) => { // jump
                                has_termination = true;
                                break;
                            }
                            _ => {}
                        }
                    }
                    Err(_) => break,
                }
            }

            if !has_termination && inst_count > 0 {
                reasons.push(format!("Falls through after {} instructions", inst_count));
            }

            // Pattern 4: Unreachable code (no calls to it)
            let called = is_routine_called(game, addr, all_routines)?;
            if !called {
                reasons.push("Not called by any routine".to_string());
            }
        }
        Err(e) => {
            reasons.push(format!("Invalid first instruction: {}", e));
        }
    }

    if reasons.is_empty() {
        reasons.push("Unknown reason".to_string());
    }

    Ok(reasons)
}

fn is_routine_called(
    game: &Game,
    target: u32,
    all_routines: &HashSet<u32>,
) -> Result<bool, String> {
    let packed_addr = pack_address(target, game.header.version);

    // Search through all routines for calls to this address
    for &routine_addr in all_routines {
        if check_routine_calls(game, routine_addr, packed_addr)? {
            return Ok(true);
        }
    }

    Ok(false)
}

fn pack_address(addr: u32, version: u8) -> u16 {
    match version {
        1..=3 => (addr / 2) as u16,
        4..=5 => (addr / 4) as u16,
        6..=7 => (addr / 4) as u16, // Simplified
        8 => (addr / 8) as u16,
        _ => 0,
    }
}

fn check_routine_calls(game: &Game, routine_addr: u32, target_packed: u16) -> Result<bool, String> {
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
                    (InstructionForm::Short, 0x05..=0x07) => true, // call_1n etc
                    _ => false,
                };

                if is_call && !inst.operands.is_empty() && inst.operands[0] == target_packed {
                    return Ok(true);
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

    Ok(false)
}

fn is_routine_end(inst: &Instruction) -> bool {
    match (inst.form, inst.opcode) {
        (InstructionForm::Short, 0x00..=0x03) | // ret variants
        (InstructionForm::Short, 0x0a) | // quit
        (InstructionForm::Short, 0x0c) => true, // jump
        _ => false,
    }
}

fn show_routine_context(game: &Game, addr: u32) -> Result<(), String> {
    let locals = game.memory[addr as usize];
    info!("  Locals: {}", locals);

    let mut pc = addr as usize + 1;
    if game.header.version <= 4 {
        pc += (locals as usize) * 2;
    }

    // Show first few instructions
    for i in 0..3 {
        if pc >= game.memory.len() {
            break;
        }

        match Instruction::decode(&game.memory, pc, game.header.version) {
            Ok(inst) => {
                info!(
                    "  Inst {}: {}",
                    i + 1,
                    inst.format_with_version(game.header.version)
                );
                pc += inst.size;
            }
            Err(e) => {
                info!("  Inst {}: ERROR - {}", i + 1, e);
                break;
            }
        }
    }

    Ok(())
}
