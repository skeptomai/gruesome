use gruesome::disasm_txd::TxdDisassembler;
use gruesome::instruction::{Instruction, InstructionForm};
use gruesome::vm::Game;
use log::info;
use std::collections::HashSet;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <game_file> <txd_routines.txt>", args[0]);
        std::process::exit(1);
    }

    let game_file = &args[1];
    let txd_file = &args[2];

    // Load game and find routines
    let memory = fs::read(game_file)?;
    let game = Game::from_memory(memory.clone())?;
    let mut disasm = TxdDisassembler::new(&game);

    disasm.discover_routines()?;
    let our_routines: HashSet<u32> = disasm.get_routine_addresses().into_iter().collect();

    // Load TXD routines
    let content = fs::read_to_string(txd_file)?;
    let mut txd_routines = HashSet::new();
    for line in content.lines() {
        let line = line.trim();
        if !line.is_empty() {
            if let Ok(addr) = u32::from_str_radix(line, 16) {
                txd_routines.insert(addr);
            }
        }
    }

    // Find extra routines
    let mut extra_routines: Vec<u32> = our_routines
        .iter()
        .filter(|addr| !txd_routines.contains(addr))
        .cloned()
        .collect();
    extra_routines.sort();

    info!("=== VALIDATING {} EXTRA ROUTINES ===", extra_routines.len());

    let mut valid_count = 0;
    let mut invalid_count = 0;
    let mut suspicious_count = 0;

    for &addr in &extra_routines {
        match validate_routine(&memory, addr, game.header.version) {
            ValidationResult::Valid => valid_count += 1,
            ValidationResult::Invalid(reason) => {
                invalid_count += 1;
                info!("❌ {:05x}: {}", addr, reason);
            }
            ValidationResult::Suspicious(reason) => {
                suspicious_count += 1;
                info!("⚠️  {:05x}: {}", addr, reason);
            }
        }
    }

    info!("\n=== SUMMARY ===");
    info!(
        "Valid routines: {} ({:.1}%)",
        valid_count,
        (valid_count as f64 / extra_routines.len() as f64) * 100.0
    );
    info!(
        "Invalid routines: {} ({:.1}%)",
        invalid_count,
        (invalid_count as f64 / extra_routines.len() as f64) * 100.0
    );
    info!(
        "Suspicious routines: {} ({:.1}%)",
        suspicious_count,
        (suspicious_count as f64 / extra_routines.len() as f64) * 100.0
    );

    if invalid_count > 0 {
        info!(
            "\n⚠️ WARNING: Found {} false positives (invalid routines)",
            invalid_count
        );
    }

    Ok(())
}

enum ValidationResult {
    Valid,
    Invalid(String),
    Suspicious(String),
}

fn validate_routine(memory: &[u8], addr: u32, version: u8) -> ValidationResult {
    let mut pc = addr as usize;

    if pc >= memory.len() {
        return ValidationResult::Invalid("Address out of bounds".to_string());
    }

    // Check locals count
    let locals = memory[pc];
    if locals > 15 {
        return ValidationResult::Invalid(format!("Invalid locals count: {locals}"));
    }
    pc += 1;

    // Skip locals
    if version <= 4 {
        pc += (locals as usize) * 2;
    }

    if pc >= memory.len() {
        return ValidationResult::Invalid("Routine header extends past memory".to_string());
    }

    // Decode instructions
    let mut instruction_count = 0;
    let mut has_terminator = false;
    let max_pc = pc + 10000; // Reasonable routine size limit

    while pc < memory.len() && pc < max_pc {
        match Instruction::decode(memory, pc, version) {
            Ok(inst) => {
                instruction_count += 1;

                // Check for terminating instructions
                match (inst.form, inst.opcode) {
                    // ret, rtrue, rfalse, ret_popped
                    (InstructionForm::Short, 0x00..=0x03) |
                    // quit
                    (InstructionForm::Short, 0x0a) => {
                        has_terminator = true;
                        break;
                    }
                    // Unconditional jump
                    (InstructionForm::Short, 0x0c) => {
                        has_terminator = true;
                        break;
                    }
                    _ => {}
                }

                pc += inst.size;
            }
            Err(e) => {
                // Check if error is the Long 0x00 we fixed
                if e.contains("Invalid Long form opcode 0x00") {
                    return ValidationResult::Invalid("Hits invalid Long opcode 0x00".to_string());
                }
                return ValidationResult::Invalid(format!("Decode error: {e}"));
            }
        }
    }

    // Analyze results
    if instruction_count == 0 {
        ValidationResult::Invalid("No valid instructions".to_string())
    } else if !has_terminator {
        ValidationResult::Suspicious(format!(
            "No terminator after {instruction_count} instructions"
        ))
    } else if instruction_count == 1 {
        ValidationResult::Suspicious("Only 1 instruction".to_string())
    } else if instruction_count > 1000 {
        ValidationResult::Suspicious(format!("Very long: {instruction_count} instructions"))
    } else {
        ValidationResult::Valid
    }
}
