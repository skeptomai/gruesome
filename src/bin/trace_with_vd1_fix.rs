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
    
    println!("Initial Vd1 value: {}", vm.read_variable(0xd1)?);
    println!("Tracing execution after Vd1 fix...\n");
    
    let mut count = 0;
    
    loop {
        count += 1;
        let pc = vm.pc;
        
        if count % 100 == 0 {
            println!("[{:04}] PC: {:05x}, Stack size: {}", count, pc, vm.stack.len());
        }
        
        // Check for the critical comparison
        if pc == 0x08cc7 {
            println!("\n*** REACHED COMPARISON AT 0x08cc7 ***");
            let vd1 = vm.read_variable(0xd1)?;
            println!("Vd1 = {}", vd1);
            println!("5 < {} = {}", vd1, 5 < vd1);
            break;
        }
        
        // Check stack overflow
        if vm.stack.len() > 500 {
            println!("\nStack size getting large: {}", vm.stack.len());
            println!("Last 10 stack values:");
            for i in 0..10.min(vm.stack.len()) {
                println!("  [{}] {}", vm.stack.len() - 1 - i, vm.stack[vm.stack.len() - 1 - i]);
            }
            break;
        }
        
        // Decode and execute instruction
        if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
            let name = inst.name(vm.game.header.version);
            
            // Log calls that might cause stack issues
            if name.starts_with("call") && count > 1000 {
                println!("[{:04}] {:05x}: {} (stack: {})", count, pc, 
                        inst.format_with_version(vm.game.header.version), vm.stack.len());
            }
            
            // Update PC
            vm.pc += inst.size as u32;
            
            // Execute
            let mut interpreter = Interpreter::new(vm);
            match interpreter.execute_instruction(&inst) {
                Ok(_) => vm = interpreter.vm,
                Err(e) => {
                    println!("Error at PC {:05x}: {}", pc, e);
                    break;
                }
            }
        } else {
            println!("Failed to decode instruction at PC {:05x}", pc);
            break;
        }
        
        if count > 2000 {
            println!("Stopped after 2000 instructions");
            break;
        }
    }
    
    Ok(())
}