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
    
    println!("Testing argument passing in Z-Machine V1-4...\n");
    
    // Run until we find the call to 8c9a
    let mut count = 0;
    let mut after_serial = false;
    
    loop {
        count += 1;
        let pc = vm.pc;
        
        if pc == 0x06f8c {
            after_serial = true;
            println!("*** SERIAL NUMBER PRINTED ***\n");
        }
        
        if after_serial {
            if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
                let name = inst.name(vm.game.header.version);
                
                // Watch for the call to 8c9a
                if name.starts_with("call") && !inst.operands.is_empty() {
                    let packed = inst.operands[0];
                    let unpacked = packed * 2;
                    
                    if unpacked == 0x8c9a {
                        println!("*** FOUND CALL TO 8c9a ***");
                        println!("PC: 0x{:05x}", pc);
                        println!("Instruction: {}", inst.format_with_version(vm.game.header.version));
                        println!("Arguments: {:?}", &inst.operands[1..]);
                        
                        // Show call stack before
                        println!("Call stack depth before: {}", vm.call_stack.len());
                        
                        // Execute the call
                        vm.pc += inst.size as u32;
                        let mut interpreter = Interpreter::new(vm);
                        match interpreter.execute_instruction(&inst) {
                            Ok(_) => {
                                vm = interpreter.vm;
                                println!("Call executed successfully");
                                
                                // Show call stack after
                                println!("Call stack depth after: {}", vm.call_stack.len());
                                
                                // Check the new frame
                                if let Some(frame) = vm.call_stack.last() {
                                    println!("New frame:");
                                    println!("  Locals count: {}", frame.num_locals);
                                    println!("  Return PC: 0x{:05x}", frame.return_pc);
                                    
                                    for i in 0..frame.num_locals as usize {
                                        println!("  Local {}: {}", i + 1, frame.locals[i]);
                                    }
                                    
                                    // Check if argument 1 is in local 1
                                    if frame.num_locals > 0 && frame.locals[0] == 1 {
                                        println!("  *** ARGUMENT PASSED CORRECTLY! ***");
                                    } else {
                                        println!("  *** ARGUMENT NOT PASSED - Local 1 should be 1 ***");
                                    }
                                }
                                
                                // Check current PC
                                println!("Current PC: 0x{:05x}", vm.pc);
                                
                                // Check if we're at the routine start
                                if vm.pc == 0x8c9a {
                                    println!("  *** PC is at routine start (header) ***");
                                } else if vm.pc > 0x8c9a {
                                    println!("  *** PC is past routine header (in code) ***");
                                } else {
                                    println!("  *** PC is before routine start (ERROR) ***");
                                }
                                
                                break;
                            }
                            Err(e) => {
                                println!("Error executing call: {}", e);
                                break;
                            }
                        }
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
        } else {
            // Execute silently before serial
            if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
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
        }
        
        if count > 300 && after_serial {
            println!("Stopped after 300 instructions post-serial");
            break;
        }
    }
    
    Ok(())
}