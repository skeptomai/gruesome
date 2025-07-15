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
    
    // Run until we're about to execute 0x5491
    info!("Running until just before PC 0x5491...");
    
    loop {
        let pc = interpreter.vm.pc;
        
        // Decode the next instruction
        let inst = Instruction::decode(&interpreter.vm.game.memory, pc as usize, interpreter.vm.game.header.version)?;
        
        // If the NEXT instruction would be at 0x5491, stop
        if pc + inst.size as u32 == 0x5491 {
            info!("Stopping at PC {:05x}, next would be 0x5491", pc);
            info!("Current instruction: {:?}", inst);
            break;
        }
        
        // If we're AT 0x5491, also stop (shouldn't happen but just in case)
        if pc == 0x5491 {
            info!("Already at 0x5491!");
            break;
        }
        
        // Execute instruction
        match interpreter.execute_instruction(&inst)? {
            ExecutionResult::Continue | ExecutionResult::Branched | ExecutionResult::Called => {},
            ExecutionResult::Quit | ExecutionResult::GameOver => {
                info!("Game quit before reaching area");
                return Ok(());
            }
            ExecutionResult::Returned(_) => {},
            ExecutionResult::Error(e) => {
                info!("Error: {}", e);
                return Ok(());
            }
        }
    }
    
    // Now let's manually trace what happens when we execute 0x5491
    info!("\nNow at PC {:05x}, about to execute instruction at 0x5491", interpreter.vm.pc);
    
    // Show the state before
    let v148_before = interpreter.vm.read_variable(148)?;
    let v3_before = interpreter.vm.read_variable(3)?;
    info!("Before: v148 = {}, v3 = {}", v148_before, v3_before);
    
    // Manually step through the problematic instruction
    let pc = interpreter.vm.pc;
    let inst = Instruction::decode(&interpreter.vm.game.memory, pc as usize, interpreter.vm.game.header.version)?;
    info!("Instruction at {:05x}: {:?}", pc, inst);
    
    // The run() method would normally do: self.vm.pc += instruction.size
    info!("Instruction size: {}", inst.size);
    info!("PC before execution: {:05x}", interpreter.vm.pc);
    
    // Try executing it
    let result = interpreter.execute_instruction(&inst)?;
    info!("Execution result: {:?}", result);
    info!("PC after execution: {:05x}", interpreter.vm.pc);
    
    // Check variables after
    let v148_after = interpreter.vm.read_variable(148)?;
    let v3_after = interpreter.vm.read_variable(3)?;
    info!("After: v148 = {}, v3 = {}", v148_after, v3_after);
    
    // The issue might be that execute_instruction is being called without advancing PC
    // Let's see what happens if we manually advance and execute again
    info!("\nManually advancing PC by instruction size...");
    interpreter.vm.pc += inst.size as u32;
    info!("PC now: {:05x}", interpreter.vm.pc);
    
    // Decode next instruction
    let next_inst = Instruction::decode(&interpreter.vm.game.memory, interpreter.vm.pc as usize, interpreter.vm.game.header.version)?;
    info!("Next instruction: {:?}", next_inst);
    
    Ok(())
}