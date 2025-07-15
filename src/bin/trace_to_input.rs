use infocom::vm::{Game, VM};
use infocom::interpreter::{Interpreter, ExecutionResult};
use infocom::instruction::Instruction;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    println!("Running game and looking for SREAD...");
    
    let mut step_count = 0;
    let max_steps = 20000;
    let mut found_sread = false;
    
    while step_count < max_steps && !found_sread {
        let pc = interpreter.vm.pc;
        
        // Decode instruction
        let inst = Instruction::decode(&interpreter.vm.game.memory, pc as usize, interpreter.vm.game.header.version)?;
        
        // Check if this is SREAD (opcode 0xF6 = 246)
        if inst.opcode == 0xF6 {
            println!("\nFound SREAD at PC {:05x} after {} steps!", pc, step_count);
            println!("Instruction: {:?}", inst);
            println!("Operands: {:?}", inst.operands);
            
            // Resolve operands to get actual buffer addresses
            if inst.operands.len() >= 2 {
                println!("Text buffer operand: {:04x}", inst.operands[0]);
                println!("Parse buffer operand: {:04x}", inst.operands[1]);
            }
            
            found_sread = true;
            break;
        }
        
        // Execute the instruction
        match interpreter.execute_instruction(&inst)? {
            ExecutionResult::Continue | ExecutionResult::Branched | ExecutionResult::Called => {},
            ExecutionResult::Quit | ExecutionResult::GameOver => {
                println!("Game ended before finding SREAD");
                break;
            }
            ExecutionResult::Returned(_) => {},
            ExecutionResult::Error(e) => {
                println!("Error at step {}: {}", step_count, e);
                break;
            }
        }
        
        step_count += 1;
        
        // Print progress every 1000 steps
        if step_count % 1000 == 0 {
            println!("Step {}: PC = {:05x}", step_count, interpreter.vm.pc);
        }
    }
    
    if !found_sread {
        println!("\nDid not find SREAD in {} steps", step_count);
    }
    
    Ok(())
}