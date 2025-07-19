use gruesome::disassembler::Disassembler;
use gruesome::vm::Game;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;
    let disasm = Disassembler::new(&game);

    println!("=== Finding QUEUE Routine in Zork I ===\n");

    // Based on ZIL knowledge, QUEUE is usually called with:
    // - First arg: routine to queue
    // - Second arg: time/count
    // - Optional third arg: flags

    // Look for calls that match this pattern
    println!("1. Looking for CALL instructions with 2-3 operands...");

    let mut potential_queue_calls = Vec::new();

    for addr in 0x5000..game.header.base_static_mem {
        if let Ok((inst, _)) = disasm.disassemble_instruction(addr as u32) {
            // Look for CALL_2S, CALL_VS, CALL_VS2
            if matches!(inst.opcode, 0x19 | 0x00 | 0x0c) {
                if inst.operands.len() >= 3 {
                    // Check if second operand looks like a time value (1-255)
                    if inst.operands[1] > 0 && inst.operands[1] <= 255 {
                        potential_queue_calls.push((addr, inst.operands[0], inst.operands[1]));
                        println!(
                            "  Found potential QUEUE call at 0x{:04x}: routine=0x{:04x}, arg2={}",
                            addr, inst.operands[0], inst.operands[1]
                        );
                    }
                }
            }
        }
    }

    // Now examine the routines being called to see if they look like QUEUE
    println!("\n2. Examining potential QUEUE routines:");

    let mut seen_routines = std::collections::HashSet::new();

    for (_call_addr, routine_packed, _time) in potential_queue_calls {
        if seen_routines.contains(&routine_packed) {
            continue;
        }
        seen_routines.insert(routine_packed);

        let routine_addr = (routine_packed as u32) * 2;
        println!("\n  Routine at 0x{:04x}:", routine_addr);

        // Disassemble first part of routine
        if let Ok(output) = disasm.disassemble_range(routine_addr, routine_addr + 100) {
            let lines: Vec<&str> = output.lines().collect();

            // Look for QUEUE-like patterns:
            // - Loops through a table
            // - Stores routine address and time
            // - Searches for empty slot

            let mut has_loop = false;
            let mut has_storew = false;
            let mut has_loadw = false;
            let mut has_je_zero = false;

            for line in &lines {
                if line.contains("jump") || line.contains("je ") {
                    has_loop = true;
                }
                if line.contains("storew") {
                    has_storew = true;
                }
                if line.contains("loadw") {
                    has_loadw = true;
                }
                if line.contains("je") && line.contains("#0000") {
                    has_je_zero = true;
                }
            }

            if has_loop && has_storew && has_loadw {
                println!("    *** LIKELY QUEUE ROUTINE ***");
                println!(
                    "    Has loop: {}, storew: {}, loadw: {}, je_zero: {}",
                    has_loop, has_storew, has_loadw, has_je_zero
                );

                // Show more of this routine
                for line in lines.iter().take(20) {
                    println!("    {}", line);
                }
            }
        }
    }

    // Look specifically at known timer-related addresses
    println!("\n3. Checking known timer-related code:");

    // From previous analysis, we know timers are often set up early
    println!("\n  Looking at initialization code around 0x4f00-0x5000:");

    for addr in 0x4f00..0x5000 {
        if let Ok((inst, text)) = disasm.disassemble_instruction(addr as u32) {
            // Look for calls with time-like values
            if matches!(inst.opcode, 0x19 | 0x00 | 0x0c) && inst.operands.len() >= 3 {
                if inst.operands[1] >= 100 && inst.operands[1] <= 400 {
                    println!("    0x{:04x}: {} ; potential timer setup", addr, text);
                }
            }
        }
    }

    // Look for the actual timer interrupt handler pattern
    println!("\n4. Looking for timer interrupt handlers:");
    println!("  (These typically DEC globals 88-90 and check for zero)");

    for addr in game.header.base_static_mem..game.header.base_static_mem + 0x5000 {
        if let Ok((inst, _)) = disasm.disassemble_instruction(addr as u32) {
            // Look for DEC or DEC_CHK on globals 88-90
            if matches!(inst.opcode, 0x03 | 0x04) && inst.operands.len() >= 1 {
                let var = inst.operands[0];
                if var >= 0x58 && var <= 0x5a {
                    println!("\n  Found timer operation at 0x{:04x}:", addr);

                    // Show the routine this is in
                    let routine_start = (addr / 8) * 8; // Rough estimate
                    if let Ok(output) =
                        disasm.disassemble_range(routine_start as u32, (addr + 50) as u32)
                    {
                        for line in output.lines().take(20) {
                            println!("    {}", line);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
