use gruesome::instruction::Instruction;
use gruesome::interpreter::Interpreter;
use gruesome::vm::{Game, VM};
use log::debug;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);

    println!("=== Tracing Timer/Queue System Initialization ===\n");

    // Set initial PC to start of game
    interpreter.vm.pc = 0x4f05;

    // Track globals 88-90 (0x58-0x5a) for timer values
    let mut timer_globals_set = false;
    let mut queue_routine_addr = None;
    let mut steps = 0;

    // Run until we see timer-related initialization
    while steps < 10000 {
        let pc = interpreter.vm.pc;

        if let Ok(inst) = Instruction::decode(
            &interpreter.vm.game.memory,
            pc as usize,
            interpreter.vm.game.header.version,
        ) {
            // Check for stores to timer globals
            if inst.opcode == 0x0d {
                // store
                if inst.operands.len() >= 2 {
                    let var = inst.operands[0];
                    let value = inst.operands[1];

                    if var >= 0x58 && var <= 0x5a {
                        println!(
                            "TIMER INIT at PC 0x{:04x}: store G{} = {}",
                            pc,
                            var - 0x10,
                            value
                        );
                        timer_globals_set = true;
                    }
                }
            }

            // Check for CALL instructions that might be QUEUE
            if matches!(inst.opcode, 0x19 | 0x00 | 0x0c) {
                if inst.operands.len() >= 3 {
                    // If we're calling with a routine and a time value
                    if inst.operands[1] > 50 && inst.operands[1] < 500 {
                        let routine_called = inst.operands[0];
                        println!("\nPotential QUEUE call at PC 0x{:04x}:", pc);
                        println!("  Routine called: 0x{:04x}", routine_called);
                        println!("  Arg1 (routine?): 0x{:04x}", inst.operands[1]);
                        println!("  Arg2 (time?): {}", inst.operands[2]);

                        if queue_routine_addr.is_none() {
                            queue_routine_addr = Some(routine_called);
                        }
                    }
                }
            }

            // Check for sread with timer
            if inst.opcode == 0x04 && inst.operands.len() >= 4 {
                let time = inst.operands[2];
                let routine = inst.operands[3];

                if time > 0 && routine > 0 {
                    println!("\nTIMED SREAD at PC 0x{:04x}:", pc);
                    println!("  Time: {} tenths of seconds", time);
                    println!("  Interrupt routine: 0x{:04x}", routine);

                    // Try to trace what the interrupt routine does
                    let int_addr = (routine as u32) * 2;
                    println!("  Interrupt routine unpacked address: 0x{:04x}", int_addr);
                }
            }

            // Step the interpreter
            let new_pc = pc + inst.size as u32;
            interpreter.vm.pc = new_pc;

            match interpreter.execute_instruction(&inst) {
                Ok(_) => {}
                Err(e) => {
                    debug!("Error at PC 0x{:04x}: {}", pc, e);
                    break;
                }
            }
        }

        steps += 1;

        // Stop if we've initialized timers and found QUEUE
        if timer_globals_set && queue_routine_addr.is_some() && steps > 100 {
            break;
        }
    }

    // Check final timer values
    println!("\n=== Timer Global Values After Init ===");
    println!("G88 (0x58): {}", interpreter.vm.read_global(0x58)?);
    println!("G89 (0x59): {}", interpreter.vm.read_global(0x59)?);
    println!("G90 (0x5a): {}", interpreter.vm.read_global(0x5a)?);

    if let Some(queue_addr) = queue_routine_addr {
        println!("\nLikely QUEUE routine at: 0x{:04x}", queue_addr);
    }

    Ok(())
}
