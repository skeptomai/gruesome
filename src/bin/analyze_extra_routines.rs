use gruesome::disasm_txd::TxdDisassembler;
use gruesome::instruction::{Instruction, InstructionForm};
use gruesome::vm::Game;
use log::info;
use std::collections::{HashMap, HashSet};

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
    let mut extras: Vec<u32> = our_routines.difference(&txd_routines).cloned().collect();
    extras.sort();

    // Find missing
    let mut missing: Vec<u32> = txd_routines.difference(&our_routines).cloned().collect();
    missing.sort();

    info!("\n=== EXTRA ROUTINES WE FOUND ({}) ===", extras.len());

    // Build a call graph to check reachability
    let mut call_targets = HashMap::new();
    for &routine_addr in &our_routines {
        let targets = find_call_targets(&game, routine_addr)?;
        call_targets.insert(routine_addr, targets);
    }

    // Categorize extra routines
    let mut unreachable = Vec::new();
    let mut reachable_by_our_extras = Vec::new();
    let mut reachable_by_txd = Vec::new();

    for &addr in &extras {
        let mut called_by_txd = false;
        let mut called_by_extra = false;

        // Check who calls this routine
        for (&caller, targets) in &call_targets {
            if targets.contains(&addr) {
                if txd_routines.contains(&caller) {
                    called_by_txd = true;
                } else if extras.contains(&caller) {
                    called_by_extra = true;
                }
            }
        }

        if called_by_txd {
            reachable_by_txd.push(addr);
        } else if called_by_extra {
            reachable_by_our_extras.push(addr);
        } else {
            unreachable.push(addr);
        }
    }

    // Analyze each category
    info!(
        "\n1. Extra routines called by TXD routines ({}):",
        reachable_by_txd.len()
    );
    for &addr in &reachable_by_txd {
        analyze_routine(&game, addr)?;
    }

    info!(
        "\n2. Extra routines only called by other extras ({}):",
        reachable_by_our_extras.len()
    );
    for &addr in &reachable_by_our_extras {
        analyze_routine(&game, addr)?;
    }

    info!(
        "\n3. Extra routines not called by anyone ({}):",
        unreachable.len()
    );
    for &addr in &unreachable {
        analyze_routine(&game, addr)?;
    }

    // Check missing routines
    info!(
        "\n\n=== ROUTINES TXD FOUND THAT WE MISSED ({}) ===",
        missing.len()
    );
    for &addr in &missing {
        info!("\n{:05x}:", addr);

        // Check if it's in a valid memory range
        if addr as usize >= game.memory.len() {
            info!("  Out of bounds!");
            continue;
        }

        // Try to decode it
        match Instruction::decode(&game.memory, addr as usize, game.header.version) {
            Ok(inst) => {
                info!("  First instruction: {:?}", inst);

                // Check if it's in our discovered ranges
                let in_routine = our_routines.iter().any(|&r| {
                    addr > r && addr < r + 1000 // Rough estimate
                });
                if in_routine {
                    info!("  This appears to be inside another routine we found");
                }
            }
            Err(e) => {
                info!("  Decode error: {}", e);

                // Show memory context
                let start = (addr as usize).saturating_sub(4);
                let end = ((addr as usize) + 4).min(game.memory.len());
                info!("  Memory context: {:02x?}", &game.memory[start..end]);
            }
        }
    }

    Ok(())
}

fn find_call_targets(game: &Game, routine_addr: u32) -> Result<HashSet<u32>, String> {
    let mut targets = HashSet::new();

    // Get routine locals count
    let locals = game.memory[routine_addr as usize];
    let mut pc = routine_addr as usize + 1;

    // Skip local variable initial values
    if game.header.version <= 4 {
        pc += (locals as usize) * 2;
    }

    // Scan through routine looking for calls
    while pc < game.memory.len() {
        match Instruction::decode(&game.memory, pc, game.header.version) {
            Ok(inst) => {
                // Check for call instructions
                match (inst.form, inst.opcode) {
                    // call (2OP:0x00 in long form)
                    (InstructionForm::Long, 0x00) if !inst.operands.is_empty() => {
                        let packed = inst.operands[0];
                        let unpacked = unpack_address(packed, game.header.version);
                        if unpacked != 0 {
                            targets.insert(unpacked);
                        }
                    }
                    // call_1n, call_2n, etc (1OP)
                    (InstructionForm::Short, 0x05..=0x07) if !inst.operands.is_empty() => {
                        let packed = inst.operands[0];
                        let unpacked = unpack_address(packed, game.header.version);
                        if unpacked != 0 {
                            targets.insert(unpacked);
                        }
                    }
                    // call_vs, call_vn (VAR)
                    (InstructionForm::Variable, 0x00 | 0x19) if !inst.operands.is_empty() => {
                        let packed = inst.operands[0];
                        let unpacked = unpack_address(packed, game.header.version);
                        if unpacked != 0 {
                            targets.insert(unpacked);
                        }
                    }
                    _ => {}
                }

                // Check for return/jump/quit
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
        6..=7 => {
            // Would need routine offset and string offset from header
            // For now, assume simple multiplication
            (packed as u32) * 4
        }
        8 => (packed as u32) * 8,
        _ => 0,
    }
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

fn analyze_routine(game: &Game, addr: u32) -> Result<(), String> {
    info!("\n  {:05x}:", addr);

    // Get locals count
    let locals = game.memory[addr as usize];
    info!("    Locals: {}", locals);

    let mut pc = addr as usize + 1;
    if game.header.version <= 4 {
        pc += (locals as usize) * 2;
    }

    // Decode first few instructions
    let mut inst_count = 0;
    let mut has_return = false;
    let mut ends_with_jump = false;
    let mut falls_through = true;

    while pc < game.memory.len() && inst_count < 20 {
        match Instruction::decode(&game.memory, pc, game.header.version) {
            Ok(inst) => {
                if inst_count < 3 {
                    info!(
                        "    Inst {}: {:?}",
                        inst_count + 1,
                        inst.format_with_version(game.header.version)
                    );
                }

                // Check for routine end
                if is_routine_end(&inst) {
                    match (inst.form, inst.opcode) {
                        (InstructionForm::Short, 0x00..=0x03) => has_return = true,
                        (InstructionForm::Short, 0x0c) => ends_with_jump = true,
                        _ => {}
                    }
                    falls_through = false;
                    break;
                }

                pc += inst.size;
                inst_count += 1;
            }
            Err(_) => break,
        }
    }

    info!(
        "    Instructions: {}, Return: {}, Jump: {}, Falls through: {}",
        inst_count, has_return, ends_with_jump, falls_through
    );

    Ok(())
}
