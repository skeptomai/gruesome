use infocom::vm::{Game, VM};
use infocom::interpreter::{Interpreter, ExecutionResult};
use infocom::instruction::Instruction;
use std::io::Read;
use log::{debug, info};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;
    
    // Create VM and run to first prompt
    let game = Game::from_memory(memory)?;
    let mut vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    println!("=== Running to first prompt ===");
    
    // Run until we hit the first sread (prompt)
    let mut instruction_count = 0;
    loop {
        let pc = interpreter.vm.pc;
        let inst = match Instruction::decode(&interpreter.vm.game.memory, pc as usize, 3) {
            Ok(inst) => inst,
            Err(e) => {
                println!("Failed to decode at {:04x}: {}", pc, e);
                break;
            }
        };
        
        // Stop at sread
        if inst.opcode == 0x04 && matches!(inst.operand_count, infocom::instruction::OperandCount::VAR) {
            println!("Reached sread at PC {:05x}", pc);
            break;
        }
        
        interpreter.vm.pc += inst.size as u32;
        match interpreter.execute_instruction(&inst) {
            Ok(ExecutionResult::Quit) => break,
            Err(e) => {
                println!("Execution error: {}", e);
                break;
            }
            _ => {}
        }
        
        instruction_count += 1;
        if instruction_count > 10000 {
            println!("Hit instruction limit");
            break;
        }
    }
    
    println!("\n=== VM State before save ===");
    println!("PC: 0x{:05x}", interpreter.vm.pc);
    println!("Stack depth: {}", interpreter.vm.stack.len());
    println!("Call stack depth: {}", interpreter.vm.call_stack.len());
    if let Some(frame) = interpreter.vm.call_stack.last() {
        println!("Current frame return PC: 0x{:05x}", frame.return_pc);
        println!("Current frame locals: {}", frame.num_locals);
        println!("Current frame stack base: {}", frame.stack_base);
    }
    
    // Save the game
    println!("\n=== Saving game ===");
    let save_result = infocom::quetzal::save::SaveGame::from_vm(&interpreter.vm);
    match save_result {
        Ok(save) => {
            save.save_to_file(std::path::Path::new("debug.sav"))?;
            println!("Save completed");
        }
        Err(e) => {
            println!("Save failed: {}", e);
            return Ok(());
        }
    }
    
    // Now try to restore
    println!("\n=== Restoring game ===");
    
    let mut vm2 = interpreter.vm;  // Move VM ownership
    
    let restore_result = infocom::quetzal::restore::RestoreGame::from_file(std::path::Path::new("debug.sav"));
    match restore_result {
        Ok(restore) => {
            match restore.restore_to_vm(&mut vm2) {
                Ok(()) => {
                    println!("Restore completed");
                }
                Err(e) => {
                    println!("Restore to VM failed: {}", e);
                    return Ok(());
                }
            }
        }
        Err(e) => {
            println!("Load save file failed: {}", e);
            return Ok(());
        }
    }
    
    println!("\n=== VM State after restore ===");
    println!("PC: 0x{:05x}", vm2.pc);
    println!("Stack depth: {}", vm2.stack.len());
    println!("Call stack depth: {}", vm2.call_stack.len());
    if let Some(frame) = vm2.call_stack.last() {
        println!("Current frame return PC: 0x{:05x}", frame.return_pc);
        println!("Current frame locals: {}", frame.num_locals);  
        println!("Current frame stack base: {}", frame.stack_base);
    }
    
    // Try to continue execution
    println!("\n=== Attempting to continue execution ===");
    let mut interpreter2 = Interpreter::new(vm2);
    
    for i in 0..10 {
        let pc = interpreter2.vm.pc;
        let inst = match Instruction::decode(&interpreter2.vm.game.memory, pc as usize, 3) {
            Ok(inst) => inst,
            Err(e) => {
                println!("Failed to decode at {:05x}: {}", pc, e);
                break;
            }
        };
        
        println!("Step {}: PC {:05x}: {}", i, pc, inst.format_with_version(3));
        
        interpreter2.vm.pc += inst.size as u32;
        match interpreter2.execute_instruction(&inst) {
            Ok(result) => {
                println!("  Result: {:?}", result);
            }
            Err(e) => {
                println!("  Execution error: {}", e);
                break;
            }
        }
    }
    
    // Clean up
    std::fs::remove_file("debug.sav").ok();
    
    Ok(())
}