use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let mut vm = VM::new(game);
    
    println!("Tracing execution to find where memory write to 0x4f15 happens...\n");
    
    let mut count = 0;
    
    loop {
        count += 1;
        let pc = vm.pc;
        
        if count % 50 == 0 {
            println!("[{:04}] PC: 0x{:05x}", count, pc);
        }
        
        // Stop at high count to avoid infinite loops
        if count > 1000 {
            println!("Stopped at count 1000 - likely infinite loop");
            break;
        }
        
        // Decode instruction
        if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
            let name = inst.name(vm.game.header.version);
            
            // Log calls since that's where the error seems to occur
            if name.starts_with("call") {
                println!("[{:04}] {:05x}: {}", count, pc, 
                        inst.format_with_version(vm.game.header.version));
            }
            
            // Update PC
            vm.pc += inst.size as u32;
            
            // Execute with error handling
            let mut interpreter = Interpreter::new(vm);
            match interpreter.execute_instruction(&inst) {
                Ok(_) => vm = interpreter.vm,
                Err(e) => {
                    println!("Error at PC {:05x} (count {}): {}", pc, count, e);
                    
                    // Show the instruction that caused the error
                    println!("Instruction: {}", inst.format_with_version(interpreter.vm.game.header.version));
                    
                    // Show some context
                    println!("Call stack depth: {}", interpreter.vm.call_stack.len());
                    if let Some(frame) = interpreter.vm.call_stack.last() {
                        println!("Current routine locals: {}", frame.num_locals);
                    }
                    
                    break;
                }
            }
        } else {
            println!("Failed to decode instruction at PC {:05x}", pc);
            break;
        }
    }
    
    Ok(())
}