use infocom::vm::{Game, VM};
use infocom::interpreter::{Interpreter, ExecutionResult};
use infocom::instruction::Instruction;
use log::{debug, info};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    // Run game and look for SREAD
    info!("Starting game, looking for SREAD instructions...");
    let max_steps = 1000000;
    let mut found_sread = false;
    
    for step in 0..max_steps {
        let pc = interpreter.vm.pc;
        
        // Progress indicator every 10000 steps
        if step % 10000 == 0 && step > 0 {
            info!("Step {}, PC={:05x}", step, pc);
        }
        
        // Check for SREAD by looking at the instruction
        let inst = Instruction::decode(&interpreter.vm.game.memory, pc as usize, interpreter.vm.game.header.version)?;
        
        // SREAD is VAR opcode 246 (0xF6)
        if inst.opcode == 246 {
            info!("=== FOUND SREAD at PC {:05x} after {} steps ===", pc, step);
            info!("Instruction: {:?}", inst);
            
            // Show operands
            if inst.operands.len() >= 2 {
                info!("Text buffer address: {:04x}", inst.operands[0]);
                info!("Parse buffer address: {:04x}", inst.operands[1]);
            }
            
            found_sread = true;
            break;
        }
        
        // Execute instruction
        match interpreter.execute_instruction(&inst)? {
            ExecutionResult::Continue | ExecutionResult::Branched | ExecutionResult::Called => {},
            ExecutionResult::Quit | ExecutionResult::GameOver => {
                info!("Game quit at PC {:05x} after {} steps", pc, step);
                break;
            }
            ExecutionResult::Returned(_) => {},
            ExecutionResult::Error(e) => {
                info!("Error at PC {:05x}: {}", pc, e);
                break;
            }
        }
    }
    
    if !found_sread {
        info!("No SREAD found in {} steps", max_steps);
    }
    
    Ok(())
}