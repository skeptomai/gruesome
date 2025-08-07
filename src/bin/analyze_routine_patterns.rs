use gruesome::disasm_txd::TxdDisassembler;
use gruesome::instruction::{Instruction, InstructionForm};
use gruesome::vm::Game;
use log::info;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
struct RoutinePattern {
    falls_through: bool,
    has_return: bool,
    ends_with_jump: bool,
    instruction_count: usize,
    first_instruction_invalid: bool,
    starts_with_ret_popped: bool,
    #[allow(dead_code)]
    called_by_others: bool,
    reachable_by_fallthrough: bool,
}

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

    // Find extras and missing
    let extras: HashSet<u32> = our_routines.difference(&txd_routines).cloned().collect();
    let missing: HashSet<u32> = txd_routines.difference(&our_routines).cloned().collect();

    // Analyze all routines and build patterns
    let mut patterns: HashMap<u32, RoutinePattern> = HashMap::new();

    // First pass: analyze each routine
    for &addr in &our_routines {
        let pattern = analyze_routine(&game, addr)?;
        patterns.insert(addr, pattern);
    }

    // Second pass: check reachability
    let mut sorted_routines: Vec<u32> = our_routines.iter().cloned().collect();
    sorted_routines.sort();

    for i in 0..sorted_routines.len() {
        let addr = sorted_routines[i];

        // Check if this routine can be reached by falling through from previous
        if i > 0 {
            let prev_addr = sorted_routines[i - 1];
            if let Some(prev_pattern) = patterns.get(&prev_addr) {
                if prev_pattern.falls_through {
                    // Check if we fall through directly to this routine
                    let prev_end = estimate_routine_end(&game, prev_addr)?;
                    if prev_end == addr {
                        if let Some(pattern) = patterns.get_mut(&addr) {
                            pattern.reachable_by_fallthrough = true;
                        }
                    }
                }
            }
        }
    }

    // Categorize extra routines by patterns
    let mut categorized: HashMap<String, Vec<u32>> = HashMap::new();

    for &addr in &extras {
        if let Some(pattern) = patterns.get(&addr) {
            let category = categorize_pattern(pattern);
            categorized
                .entry(category)
                .or_default()
                .push(addr);
        }
    }

    // Report findings
    info!("\n=== EXTRA ROUTINE PATTERNS ===\n");

    for (category, addrs) in categorized.iter() {
        info!("{} ({} routines):", category, addrs.len());
        for &addr in addrs.iter().take(5) {
            if let Some(pattern) = patterns.get(&addr) {
                info!("  {:05x}: {:?}", addr, pattern);
            }
        }
        if addrs.len() > 5 {
            info!("  ... and {} more", addrs.len() - 5);
        }
        info!("");
    }

    // Analyze missing routines
    info!("\n=== MISSING ROUTINE ANALYSIS ===\n");

    let mut inside_other_routine = 0;
    let mut invalid_opcodes = 0;
    let mut other_missing = 0;

    for &addr in &missing {
        if addr as usize >= game.memory.len() {
            continue;
        }

        // Check if inside another routine we found
        let in_routine = our_routines.iter().any(|&r| addr > r && addr < r + 10000);

        if in_routine {
            inside_other_routine += 1;
        } else {
            // Try to decode first instruction
            match Instruction::decode(&game.memory, addr as usize, game.header.version) {
                Ok(_) => other_missing += 1,
                Err(e) if e.contains("Invalid Long form opcode 0x00") => invalid_opcodes += 1,
                Err(_) => other_missing += 1,
            }
        }
    }

    info!("Inside other routines: {}", inside_other_routine);
    info!("Invalid Long 0x00: {}", invalid_opcodes);
    info!("Other missing: {}", other_missing);

    // Summary
    info!("\n=== SUMMARY ===\n");
    info!("We find {} extra routines", extras.len());
    info!("We miss {} routines", missing.len());
    info!("Most missing routines are inside others or have invalid opcodes");
    info!("Extra routines appear to be:");
    info!("- Code fragments reachable by fallthrough");
    info!("- Orphan code not called directly");
    info!("- Valid routines TXD's heuristics reject");

    Ok(())
}

fn analyze_routine(game: &Game, addr: u32) -> Result<RoutinePattern, String> {
    let mut pattern = RoutinePattern {
        falls_through: true,
        has_return: false,
        ends_with_jump: false,
        instruction_count: 0,
        first_instruction_invalid: false,
        starts_with_ret_popped: false,
        called_by_others: false,
        reachable_by_fallthrough: false,
    };

    // Get locals count
    let locals = game.memory[addr as usize];
    let mut pc = addr as usize + 1;

    // Skip local variable initial values
    if game.header.version <= 4 {
        pc += (locals as usize) * 2;
    }

    // Analyze instructions
    let mut first = true;
    while pc < game.memory.len() && pattern.instruction_count < 100 {
        match Instruction::decode(&game.memory, pc, game.header.version) {
            Ok(inst) => {
                if first {
                    first = false;
                    // Check for ret_popped as first instruction
                    if matches!((inst.form, inst.opcode), (InstructionForm::Short, 0x09)) {
                        pattern.starts_with_ret_popped = true;
                    }
                }

                // Check for routine end
                if is_routine_end(&inst) {
                    pattern.falls_through = false;
                    match (inst.form, inst.opcode) {
                        (InstructionForm::Short, 0x00..=0x03) => pattern.has_return = true,
                        (InstructionForm::Short, 0x0c) => pattern.ends_with_jump = true,
                        _ => {}
                    }
                    break;
                }

                pc += inst.size;
                pattern.instruction_count += 1;
            }
            Err(_) => {
                if first {
                    pattern.first_instruction_invalid = true;
                }
                break;
            }
        }
    }

    Ok(pattern)
}

fn is_routine_end(inst: &Instruction) -> bool {
    match (inst.form, inst.opcode) {
        // ret, rtrue, rfalse, ret_popped
        (InstructionForm::Short, 0x00..=0x03) => true,
        // quit
        (InstructionForm::Short, 0x0a) => true,
        // jump (unconditional)
        (InstructionForm::Short, 0x0c) => true,
        _ => false,
    }
}

fn estimate_routine_end(game: &Game, addr: u32) -> Result<u32, String> {
    let locals = game.memory[addr as usize];
    let mut pc = addr as usize + 1;

    if game.header.version <= 4 {
        pc += (locals as usize) * 2;
    }

    while pc < game.memory.len() {
        match Instruction::decode(&game.memory, pc, game.header.version) {
            Ok(inst) => {
                pc += inst.size;
                if is_routine_end(&inst) {
                    return Ok(pc as u32);
                }
            }
            Err(_) => return Ok(pc as u32),
        }
    }

    Ok(pc as u32)
}

fn categorize_pattern(pattern: &RoutinePattern) -> String {
    if pattern.first_instruction_invalid {
        "Invalid first instruction".to_string()
    } else if pattern.starts_with_ret_popped {
        "Starts with ret_popped".to_string()
    } else if pattern.reachable_by_fallthrough && pattern.instruction_count < 5 {
        "Short fallthrough fragment".to_string()
    } else if pattern.falls_through && !pattern.has_return && !pattern.ends_with_jump {
        "Falls through (no explicit end)".to_string()
    } else if pattern.has_return && pattern.instruction_count < 3 {
        "Very short with return".to_string()
    } else if pattern.ends_with_jump && pattern.instruction_count < 3 {
        "Very short with jump".to_string()
    } else {
        "Other valid routine".to_string()
    }
}
