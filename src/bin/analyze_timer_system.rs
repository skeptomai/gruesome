use gruesome::disassembler::Disassembler;
use gruesome::instruction::OperandType;
use gruesome::vm::Game;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;
    let disasm = Disassembler::new(&game);

    println!("=== Analyzing Timer System in Zork I ===\n");

    // Search for QUEUE routine by looking at the routine table
    println!("1. Searching for QUEUE routine...");

    // The routine table starts at the static memory boundary
    let routines_start = game.header.base_static_mem;
    println!("Routines start at: 0x{routines_start:04x}");

    // Look for routines that might be QUEUE based on pattern
    // QUEUE typically takes 2-3 arguments and manipulates a queue structure

    // Search for timed sread instructions
    println!("\n2. Finding all timed sread instructions (with 4+ operands):");
    let mut timed_sreads = Vec::new();

    for addr in 0x5000..game.header.base_static_mem {
        if let Ok((inst, _)) = disasm.disassemble_instruction(addr as u32) {
            if inst.opcode == 0x04 && inst.operands.len() >= 4 {
                let time = inst.operands[2];
                let routine = inst.operands[3];
                if time > 0 && routine > 0 {
                    timed_sreads.push((addr, time, routine));
                    println!("  0x{addr:04x}: sread with time={time} routine=0x{routine:04x}");
                }
            }
        }
    }

    // Analyze the interrupt routines
    println!("\n3. Analyzing interrupt routines:");
    for (sread_addr, _time, routine_packed) in &timed_sreads {
        let routine_addr = (*routine_packed as u32) * 2;
        println!("\n  Interrupt routine 0x{routine_addr:04x} (from sread at 0x{sread_addr:04x}):");

        // Disassemble first few instructions
        if let Ok(output) = disasm.disassemble_range(routine_addr, routine_addr + 30) {
            for line in output.lines().take(10) {
                println!("    {line}");
            }
        }

        // Look for global variable accesses in the routine
        println!("  Checking for timer global accesses:");
        for offset in 0..50 {
            if let Ok((inst, _)) = disasm.disassemble_instruction(routine_addr + offset) {
                // Check for load/store/dec/inc of globals 88-90 (0x58-0x5a)
                for (i, op) in inst.operands.iter().enumerate() {
                    if matches!(inst.operand_types[i], OperandType::Variable)
                        && *op >= 0x58
                        && *op <= 0x5a
                    {
                        println!(
                            "    Found access to G{} at +0x{:02x}: {}",
                            op - 0x10,
                            offset,
                            inst.opcode
                        );
                    }
                }
            }
        }
    }

    // Look for globals that might be timers
    println!("\n4. Checking timer-related globals:");
    println!("  G88 (0x58) - Likely lantern timer");
    println!("  G89 (0x59) - Likely match timer");
    println!("  G90 (0x5a) - Likely candle timer");

    // Search for DEC_CHK instructions on these globals
    println!("\n5. Searching for DEC_CHK on timer globals:");
    for addr in 0x5000..game.header.base_static_mem {
        if let Ok((inst, _)) = disasm.disassemble_instruction(addr as u32) {
            if inst.opcode == 0x04 {
                // dec_chk
                if !inst.operands.is_empty() {
                    let var = inst.operands[0];
                    if (0x58..=0x5a).contains(&var) {
                        println!("  0x{:04x}: dec_chk G{}", addr, var - 0x10);

                        // Show context
                        if let Ok(output) =
                            disasm.disassemble_range((addr - 5) as u32, (addr + 10) as u32)
                        {
                            for line in output.lines() {
                                println!("    {line}");
                            }
                        }
                    }
                }
            }
        }
    }

    // Look for the QUEUE routine by searching for specific patterns
    println!("\n6. Searching for QUEUE routine pattern:");
    println!("  (Looking for routines that manipulate a queue structure)");

    // QUEUE routines typically:
    // - Take 2-3 arguments (routine, time, optional flag)
    // - Access a table/array structure
    // - Loop through entries
    // - Store routine and time values

    for addr in routines_start..routines_start + 0x5000 {
        if let Ok((inst, _)) = disasm.disassemble_instruction(addr as u32) {
            // Look for routines that start with typical queue manipulation
            if inst.opcode == 0x0d {
                // store
                // Check if storing to a table-like structure
                if let Ok(output) = disasm.disassemble_range(addr as u32, (addr + 50) as u32) {
                    let lines: Vec<&str> = output.lines().collect();

                    // Look for patterns like loops and table access
                    let mut has_loop = false;
                    let mut has_table_access = false;

                    for line in &lines {
                        if line.contains("jump") || line.contains("je") {
                            has_loop = true;
                        }
                        if line.contains("get_prop")
                            || line.contains("put_prop")
                            || line.contains("storew")
                            || line.contains("loadw")
                        {
                            has_table_access = true;
                        }
                    }

                    if has_loop && has_table_access {
                        println!("\n  Potential QUEUE routine at 0x{addr:04x}:");
                        for line in lines.iter().take(15) {
                            println!("    {line}");
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
