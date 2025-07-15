use infocom::vm::{Game, VM};
use infocom::interpreter::{Interpreter, ExecutionResult};
use infocom::instruction::Instruction;
use log::{debug, info};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    // Run until we hit PC 0x5491
    info!("Running until PC 0x5491...");
    
    loop {
        let pc = interpreter.vm.pc;
        
        if pc == 0x5491 {
            info!("Reached PC 0x5491!");
            
            // Decode the instruction
            let inst = Instruction::decode(&interpreter.vm.game.memory, pc as usize, interpreter.vm.game.header.version)?;
            info!("Instruction at 0x5491: {:?}", inst);
            
            // Show the variables involved
            if inst.opcode == 0x14 { // ADD
                info!("ADD instruction: v{} + {} -> v{}", 
                    inst.operands[0], inst.operands[1], inst.store_var.unwrap_or(0));
                
                // Get the value of the variable being added
                let var_val = interpreter.vm.read_variable(inst.operands[0] as u8)?;
                info!("Variable v{} = {}", inst.operands[0], var_val);
                info!("Adding constant {}", inst.operands[1]);
                
                // Execute the instruction once
                interpreter.execute_instruction(&inst)?;
                
                // See where we go next
                info!("After ADD, PC = {:05x}", interpreter.vm.pc);
                
                // Execute a few more instructions to see the pattern
                for i in 0..10 {
                    let next_pc = interpreter.vm.pc;
                    let next_inst = Instruction::decode(&interpreter.vm.game.memory, next_pc as usize, interpreter.vm.game.header.version)?;
                    info!("Step {}: PC {:05x} - {:?}", i, next_pc, next_inst);
                    
                    if next_pc == 0x5491 {
                        info!("Back at 0x5491 - we're in a loop!");
                        
                        // Show branch condition
                        if let Some(prev_inst) = Instruction::decode(&interpreter.vm.game.memory, (next_pc - 2) as usize, interpreter.vm.game.header.version).ok() {
                            info!("Previous instruction might be: {:?}", prev_inst);
                        }
                        break;
                    }
                    
                    interpreter.execute_instruction(&next_inst)?;
                }
            }
            
            break;
        }
        
        // Execute normal instruction
        let inst = Instruction::decode(&interpreter.vm.game.memory, pc as usize, interpreter.vm.game.header.version)?;
        match interpreter.execute_instruction(&inst)? {
            ExecutionResult::Continue | ExecutionResult::Branched | ExecutionResult::Called => {},
            ExecutionResult::Quit | ExecutionResult::GameOver => {
                info!("Game quit before reaching 0x5491");
                break;
            }
            ExecutionResult::Returned(_) => {},
            ExecutionResult::Error(e) => {
                info!("Error: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}