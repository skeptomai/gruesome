use gruesome::interpreter::Interpreter;
use gruesome::vm::{Game, VM};
use log::{debug, info};
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    info!("Starting interpreter...");
    info!("Initial PC: 0x{:04x}", interpreter.vm.pc);
    info!("Initial G88 (lantern timer): {}", interpreter.vm.read_global(0x58)?);
    
    // Run for a limited number of instructions to see timer behavior
    let mut instruction_count = 0;
    let max_instructions = 10000;
    let mut found_timer_sread = false;
    
    while instruction_count < max_instructions {
        instruction_count += 1;
        
        let pc = interpreter.vm.pc;
        let inst = match gruesome::instruction::Instruction::decode(
            &interpreter.vm.game.memory, 
            pc as usize,
            interpreter.vm.game.header.version
        ) {
            Ok(inst) => inst,
            Err(e) => {
                eprintln!("Error decoding at 0x{:04x}: {}", pc, e);
                break;
            }
        };
        
        // Check if this is an SREAD with timer
        if inst.opcode == 0x04 && inst.operands.len() >= 4 {
            let time = inst.operands[2];
            let routine = inst.operands[3];
            if time > 0 && routine > 0 {
                found_timer_sread = true;
                info!("Found SREAD with timer at PC 0x{:04x}:", pc);
                info!("  Time: {} ({}s)", time, time as f32 / 10.0);
                info!("  Routine: 0x{:04x}", routine);
                info!("  G88 before: {}", interpreter.vm.read_global(0x58)?);
                
                // We can't easily execute through the sread here,
                // but we've confirmed timer detection works
                break;
            }
        }
        
        // Update PC and execute
        interpreter.vm.pc += inst.size as u32;
        
        match interpreter.execute_instruction(&inst)? {
            gruesome::interpreter::ExecutionResult::Quit => break,
            gruesome::interpreter::ExecutionResult::GameOver => break,
            gruesome::interpreter::ExecutionResult::Error(e) => {
                eprintln!("Execution error: {}", e);
                break;
            }
            _ => {}
        }
    }
    
    if !found_timer_sread {
        info!("No timer SREAD found in first {} instructions", instruction_count);
        info!("This might mean the game needs more setup before timers are used");
    }
    
    Ok(())
}