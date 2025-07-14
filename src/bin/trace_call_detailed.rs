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
    
    println!("Detailed trace of call instruction execution...\n");
    
    // Run until we reach the call to 8c9a
    let mut count = 0;
    
    loop {
        count += 1;
        let pc = vm.pc;
        
        if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
            let name = inst.name(vm.game.header.version);
            
            // Stop right before the call to 8c9a
            if name.starts_with("call") && !inst.operands.is_empty() {
                let packed = inst.operands[0];
                let unpacked = packed * 2;
                
                if unpacked == 0x8c9a {
                    println!("Found call to 8c9a at PC {:05x}", pc);
                    println!("Instruction: {}", inst.format_with_version(vm.game.header.version));
                    
                    // Manual step through the call execution
                    println!("\nStep-by-step call execution:");
                    println!("1. Packed address: 0x{:04x}", packed);
                    println!("2. Unpacked address: 0x{:05x}", unpacked);
                    
                    // Check the target routine
                    let target_addr = unpacked as u32;
                    println!("3. Target routine at 0x{:05x}", target_addr);
                    
                    // Check the routine header
                    let num_locals = vm.game.memory[target_addr as usize];
                    println!("4. Number of locals: {}", num_locals);
                    
                    // Check memory region
                    let static_mem_base = vm.game.header.base_static_mem as u32;
                    let high_mem_base = vm.game.header.base_high_mem as u32;
                    
                    if target_addr < static_mem_base {
                        println!("5. Target is in DYNAMIC memory (writable)");
                    } else if target_addr < high_mem_base {
                        println!("6. Target is in STATIC memory (read-only)");
                    } else {
                        println!("7. Target is in HIGH memory (read-only)");
                    }
                    
                    // Check what the PC would be set to
                    let new_pc = target_addr + 1;
                    println!("8. PC would be set to: 0x{:05x}", new_pc);
                    
                    // Check if the PC location has initial values
                    println!("9. Initial values at PC location:");
                    for i in 0..num_locals {
                        let value_addr = new_pc + (i as u32 * 2);
                        println!("   Local {}: addr 0x{:05x}", i, value_addr);
                        
                        // Check if this address would cause a write error
                        if value_addr == 0x4f15 {
                            println!("   *** ADDRESS 0x4f15 FOUND! ***");
                            println!("   This is where the error occurs!");
                        }
                    }
                    
                    break;
                }
            }
            
            // Execute normally
            vm.pc += inst.size as u32;
            let mut interpreter = Interpreter::new(vm);
            match interpreter.execute_instruction(&inst) {
                Ok(_) => vm = interpreter.vm,
                Err(e) => {
                    println!("Error: {}", e);
                    break;
                }
            }
        }
        
        if count > 200 {
            println!("Didn't find call to 8c9a in first 200 instructions");
            break;
        }
    }
    
    Ok(())
}