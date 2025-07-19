use gruesome::instruction::Instruction;
use gruesome::interpreter::Interpreter;
use gruesome::vm::{Game, VM};
use log::info;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);

    // Set a breakpoint-like check
    // We want to trace execution from 0x6340 to 0x6360

    info!("Setting up trace for leaves error...");

    // Run the game until we hit the area of interest
    let mut interpreter = Interpreter::new(vm);

    // We need to manually trace when we're in the error area
    let mut instruction_count = 0;
    let mut in_trace_area = false;

    loop {
        let pc = interpreter.vm.pc;

        // Check if we're in the area where the error happens
        if pc >= 0x6340 && pc <= 0x6360 {
            if !in_trace_area {
                info!("=== Entered trace area at PC 0x{:04x} ===", pc);
                in_trace_area = true;
            }

            // Decode and show instruction
            if let Ok(inst) = Instruction::decode(
                &interpreter.vm.game.memory,
                pc as usize,
                interpreter.vm.game.header.version,
            ) {
                info!("PC 0x{:04x}: {:?}", pc, inst.opcode);

                // Resolve operands
                if let Ok(operands) = interpreter.resolve_operands(&inst) {
                    info!("  Operands: {:?}", operands);

                    // Special tracking for loadb
                    if inst.opcode == 0x10 && operands.len() >= 2 {
                        let addr = operands[0] as u32 + operands[1] as u32;
                        let value = interpreter.vm.read_byte(addr);
                        info!(
                            "  loadb: addr=0x{:04x} (0x{:04x}+{}), value=0x{:02x} ({})",
                            addr, operands[0], operands[1], value, value
                        );
                    }
                }

                // Update PC
                interpreter.vm.pc += inst.size as u32;

                // Execute
                if let Err(e) = interpreter.execute_instruction(&inst) {
                    eprintln!("Error: {}", e);
                    break;
                }
            }
        } else if in_trace_area {
            info!("=== Left trace area at PC 0x{:04x} ===", pc);
            break;
        } else {
            // Normal execution
            if let Ok(inst) = Instruction::decode(
                &interpreter.vm.game.memory,
                pc as usize,
                interpreter.vm.game.header.version,
            ) {
                interpreter.vm.pc += inst.size as u32;

                match interpreter.execute_instruction(&inst) {
                    Ok(gruesome::interpreter::ExecutionResult::Quit) => break,
                    Ok(gruesome::interpreter::ExecutionResult::GameOver) => break,
                    Err(e) => {
                        eprintln!("Error at PC 0x{:04x}: {}", pc, e);
                        break;
                    }
                    _ => {}
                }
            }
        }

        instruction_count += 1;
        if instruction_count > 1000000 {
            info!("Instruction limit reached");
            break;
        }
    }

    Ok(())
}
