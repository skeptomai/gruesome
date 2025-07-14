use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data).map_err(|e| format!("Game load error: {}", e))?;
    let mut vm = VM::new(game);
    
    // Run the game manually, looking for STAND entry
    println!("Running game, looking for entry to STAND routine at 0x86ca...\n");
    
    let mut count = 0;
    let mut after_serial = false;
    
    loop {
        count += 1;
        let pc = vm.pc;
        
        // Progress indicator
        if count % 1000 == 0 {
            println!("... {} instructions executed, PC = {:05x}", count, pc);
        }
        
        // Check for serial number print
        if pc == 0x06f8c {
            after_serial = true;
            println!("\n*** SERIAL NUMBER PRINT at instruction {} ***", count);
        }
        
        // Check for STAND routine
        if pc == 0x86ca {
            println!("\n!!! FOUND STAND ROUTINE ENTRY at instruction {} !!!", count);
            println!("This happened {} instructions after serial print", 
                     if after_serial { "AFTER" } else { "BEFORE" });
            
            // Check G6f
            if let Ok(g6f_val) = vm.read_variable(16 + 0x6f) {
                println!("\nG6f = {} (0x{:02x})", g6f_val, g6f_val);
                
                // Get parent
                if g6f_val > 0 && g6f_val < 256 {
                    if let Ok(parent) = vm.get_parent(g6f_val) {
                        println!("Parent of G6f: {} (0x{:02x})", parent, parent);
                        
                        // Check attribute 0x1b
                        if parent > 0 {
                            if let Ok(has_attr) = vm.test_attribute(parent as u16, 0x1b) {
                                println!("Parent has attribute 0x1b (27): {}", has_attr);
                                if !has_attr {
                                    println!("=> This will print 'You are already standing'");
                                }
                            }
                        }
                    }
                }
            }
            
            // Show call stack to see how we got here
            println!("\nCall stack depth: {}", vm.call_stack.len());
            for (i, frame) in vm.call_stack.iter().enumerate() {
                println!("  [{}] Return to {:05x}", i, frame.return_pc);
            }
            
            break;
        }
        
        // Decode and execute instruction
        if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
            // Update PC first
            vm.pc += inst.size as u32;
            
            // Create interpreter just for instruction execution
            let mut interpreter = Interpreter::new(vm);
            match interpreter.execute_instruction(&inst) {
                Ok(_) => {
                    vm = interpreter.vm;
                },
                Err(e) => {
                    eprintln!("Error at PC {:05x}: {}", pc, e);
                    break;
                }
            }
        } else {
            eprintln!("Failed to decode instruction at PC {:05x}", pc);
            break;
        }
        
        if count > 20000 {
            println!("\nStopped after 20000 instructions without finding STAND");
            break;
        }
    }

    Ok(())
}