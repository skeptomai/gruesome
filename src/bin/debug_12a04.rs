use gruesome::disasm_txd::TxdDisassembler;
use gruesome::instruction::Instruction;
use gruesome::vm::Game;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let memory = fs::read("resources/test/amfv/amfv-r79-s851122.z4")?;
    let game = Game::from_memory(memory.clone())?;
    let mut disasm = TxdDisassembler::new(&game);

    println!("\n=== Testing why 12a04 is not found ===");

    // First, check if it's a valid routine
    let addr = 0x12a04;
    let locals = memory[addr];
    println!("Address {:05x}: locals = {}", addr, locals);

    // Try to decode instructions
    let mut pc = addr + 1;
    if game.header.version <= 4 {
        pc += (locals as usize) * 2;
    }

    println!("First instruction at: {:05x}", pc);

    // Decode several instructions
    for i in 0..10 {
        match Instruction::decode(&memory, pc, game.header.version) {
            Ok(inst) => {
                println!("  {:05x}: {:?}", pc, inst);
                pc += inst.size;
            }
            Err(e) => {
                println!("  {:05x}: ERROR: {}", pc, e);
                break;
            }
        }
    }

    // Now check if our disassembler finds it
    println!("\n=== Running disassembler ===");
    disasm.discover_routines()?;
    let routines = disasm.get_routine_addresses();

    if routines.contains(&0x12a04) {
        println!("✓ Found 12a04 in our routine list");
    } else {
        println!("✗ Did NOT find 12a04 in our routine list");

        // Check nearby routines
        let mut nearby: Vec<u32> = routines
            .iter()
            .filter(|&&r| r >= 0x12900 && r <= 0x12b00)
            .cloned()
            .collect();
        nearby.sort();

        println!("\nNearby routines we found:");
        for r in nearby {
            println!("  {:05x}", r);
        }
    }

    // Check if it's in our boundaries
    println!("\n=== Checking boundaries ===");
    // We'd need access to low_address and high_address from disasm
    // but they're private. Let's check if any routine calls 12a04

    let packed_12a04 = 0x12a04 / 4; // V4 packing
    println!("Looking for calls to packed address {:04x}", packed_12a04);

    let mut found_calls = 0;
    for &routine in &routines {
        // Scan routine for CALL instructions
        let mut pc = routine + 1;
        let locals = memory[routine as usize];
        if game.header.version <= 4 {
            pc += (locals as u32) * 2;
        }

        while pc < routine + 1000 && (pc as usize) < memory.len() {
            match Instruction::decode(&memory, pc as usize, game.header.version) {
                Ok(inst) => {
                    // Check if it's a CALL with our target
                    if matches!(inst.opcode, 0x00 | 0x08 | 0x19 | 0x1a)
                        && !inst.operands.is_empty()
                        && inst.operands[0] == packed_12a04 as u16
                    {
                        println!("  Found call from {:05x}", routine);
                        found_calls += 1;
                    }
                    pc += inst.size as u32;
                }
                Err(_) => break,
            }
        }
    }

    if found_calls == 0 {
        println!("  No calls found from discovered routines");
    }

    Ok(())
}
