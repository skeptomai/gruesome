use gruesome::vm::Game;
use gruesome::disasm_txd::TxdDisassembler;
use gruesome::instruction::{Instruction, InstructionForm};
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
    
    // Test cases of routines nested in code body
    let nested_routines = vec![
        (0x0d198, 0x0d184),
        (0x0d6f4, 0x0d6e8),
        (0x0e6f8, 0x0e6e8),
        (0x0e96c, 0x0e960),
        (0x25564, 0x25550),
        (0x2b3b4, 0x2b384),
    ];
    
    info!("=== CHECKING IF NESTED ROUTINES ARE CALLED ===");
    
    // Build a set of all called addresses
    let mut called_addresses = HashSet::new();
    let routines = disasm.get_routine_addresses();
    
    for &routine_addr in &routines {
        let calls = get_routine_calls(&game, routine_addr);
        for call in calls {
            called_addresses.insert(call);
        }
    }
    
    info!("\nTotal unique addresses called: {}", called_addresses.len());
    
    // Check each nested routine
    for (nested, parent) in nested_routines {
        let is_called = called_addresses.contains(&nested);
        let is_in_txd = check_in_txd_list(nested);
        
        info!("\n{:05x} (nested in {:05x}):", nested, parent);
        info!("  Called by any routine: {}", is_called);
        info!("  In TXD list: {}", is_in_txd);
        
        // Decode a few instructions at this address to see what it looks like
        info!("  First few instructions:");
        let mut pc = nested as usize;
        let locals = game.memory[pc];
        info!("    Locals: {}", locals);
        pc += 1;
        if game.header.version <= 4 {
            pc += (locals as usize) * 2;
        }
        
        for i in 0..3 {
            match Instruction::decode(&game.memory, pc, game.header.version) {
                Ok(inst) => {
                    info!("    +{:02x}: {:?} {:?}", pc - nested as usize, inst.form, inst.opcode);
                    pc += inst.size;
                }
                Err(e) => {
                    info!("    +{:02x}: Invalid instruction: {}", pc - nested as usize, e);
                    break;
                }
            }
        }
    }
    
    Ok(())
}

fn get_routine_calls(game: &Game, routine_addr: u32) -> Vec<u32> {
    let mut calls = Vec::new();
    let mut pc = routine_addr as usize;
    
    // Skip locals
    let locals = game.memory[pc];
    pc += 1;
    if game.header.version <= 4 {
        pc += (locals as usize) * 2;
    }
    
    // Decode instructions looking for calls
    let max_size = 10000;
    while pc < game.memory.len() && (pc - routine_addr as usize) < max_size {
        match Instruction::decode(&game.memory, pc, game.header.version) {
            Ok(inst) => {
                // Check for call instructions
                match (inst.form, inst.opcode) {
                    // call_1s, call_2s, call_vs, call_vs2
                    (InstructionForm::Short, 0x08) |
                    (InstructionForm::Variable, 0x00) |
                    (InstructionForm::Variable, 0x19) |
                    (InstructionForm::Variable, 0x1a) => {
                        if !inst.operands.is_empty() && inst.operands[0] != 0 {
                            let packed_addr = inst.operands[0] as u32;
                            let call_addr = packed_addr * 2; // v3 packing
                            calls.push(call_addr);
                        }
                    }
                    // call_1n, call_2n, call_vn, call_vn2 (v5+)
                    (InstructionForm::Short, 0x0f) |
                    (InstructionForm::Variable, 0x1f) |
                    (InstructionForm::Variable, 0x19) |
                    (InstructionForm::Variable, 0x1a) if game.header.version >= 5 => {
                        if !inst.operands.is_empty() && inst.operands[0] != 0 {
                            let packed_addr = inst.operands[0] as u32;
                            let call_addr = packed_addr * 2; // v3 packing for now
                            calls.push(call_addr);
                        }
                    }
                    _ => {}
                }
                
                pc += inst.size;
                
                // Check for routine end
                match (inst.form, inst.opcode) {
                    // ret, rtrue, rfalse, ret_popped
                    (InstructionForm::Short, 0x00..=0x03) |
                    // quit
                    (InstructionForm::Short, 0x0a) => {
                        break;
                    }
                    _ => {}
                }
            }
            Err(_) => break,
        }
    }
    
    calls
}

fn check_in_txd_list(addr: u32) -> bool {
    // TXD routines from our file
    let txd_routines = vec![
        0xcaf4, 0xcb1c, 0xcb34, 0xcb54, 0xcba4, 0xcbc8, 0xcd10, 0xcd1c, 0xcf88, 0xd004, 
        0xd078, 0xd164, 0xd184, 0xd198, 0xd1f0, 0xd254, 0xd61c, 0xd63c, 0xd6e8, 0xd6f4,
        // ... truncated for brevity, but we need the specific ones
    ];
    
    txd_routines.contains(&addr)
}