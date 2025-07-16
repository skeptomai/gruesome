use infocom::vm::{Game, VM};
use infocom::interpreter::{Interpreter, ExecutionResult};
use infocom::instruction::Instruction;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;
    
    println!("=== Testing Save/Restore in Game Context ===\n");
    
    // Create VM and run to first prompt
    let game = Game::from_memory(memory.clone())?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    // Run until first sread (input prompt)
    let mut count = 0;
    loop {
        let pc = interpreter.vm.pc;
        let inst = Instruction::decode(&interpreter.vm.game.memory, pc as usize, 3)?;
        
        if inst.opcode == 0x04 && matches!(inst.operand_count, infocom::instruction::OperandCount::VAR) {
            println!("Reached first prompt at PC 0x{:05x}", pc);
            break;
        }
        
        interpreter.vm.pc += inst.size as u32;
        interpreter.execute_instruction(&inst)?;
        
        count += 1;
        if count > 10000 {
            println!("Too many instructions");
            return Ok(());
        }
    }
    
    // Simulate entering "save"
    println!("\nSimulating SAVE command...");
    
    // Manually create save file with a test name
    let save = infocom::quetzal::save::SaveGame::from_vm(&interpreter.vm)?;
    save.save_to_file(std::path::Path::new("test_flow.sav"))?;
    println!("Save completed to test_flow.sav");
    
    // Show VM state
    println!("\nVM state at save:");
    println!("  PC: 0x{:05x}", interpreter.vm.pc);
    println!("  Stack: {} values", interpreter.vm.stack.len());
    println!("  Call frames: {}", interpreter.vm.call_stack.len());
    
    // Now create a fresh VM and restore
    println!("\nCreating fresh VM and restoring...");
    let game2 = Game::from_memory(memory)?;
    let mut vm2 = VM::new(game2);
    
    // Run the fresh VM to a different point (to prove restore works)
    let mut interpreter2 = Interpreter::new(vm2);
    for _ in 0..100 {
        let pc = interpreter2.vm.pc;
        let inst = Instruction::decode(&interpreter2.vm.game.memory, pc as usize, 3)?;
        interpreter2.vm.pc += inst.size as u32;
        interpreter2.execute_instruction(&inst)?;
    }
    
    println!("Fresh VM now at PC 0x{:05x}", interpreter2.vm.pc);
    
    // Restore
    let restore = infocom::quetzal::restore::RestoreGame::from_file(std::path::Path::new("test_flow.sav"))?;
    restore.restore_to_vm(&mut interpreter2.vm)?;
    
    println!("\nVM state after restore:");
    println!("  PC: 0x{:05x} (unchanged - correct for v3)", interpreter2.vm.pc);
    println!("  Stack: {} values", interpreter2.vm.stack.len());
    println!("  Call frames: {}", interpreter2.vm.call_stack.len());
    
    // Try to execute a few more instructions
    println!("\nExecuting 5 instructions after restore:");
    for i in 0..5 {
        let pc = interpreter2.vm.pc;
        let inst = match Instruction::decode(&interpreter2.vm.game.memory, pc as usize, 3) {
            Ok(inst) => inst,
            Err(e) => {
                println!("  Step {}: ERROR decoding at 0x{:05x}: {}", i, pc, e);
                break;
            }
        };
        
        println!("  Step {}: PC 0x{:05x}: {}", i, pc, inst.format_with_version(3));
        
        interpreter2.vm.pc += inst.size as u32;
        match interpreter2.execute_instruction(&inst) {
            Ok(_) => {},
            Err(e) => {
                println!("    ERROR: {}", e);
                break;
            }
        }
    }
    
    // Clean up
    std::fs::remove_file("test_flow.sav").ok();
    
    Ok(())
}