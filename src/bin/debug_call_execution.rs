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
    
    println!("Debugging call execution to see what happens when we call 8c9a...\n");
    
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
                        println!("[{:04}] {:05x}: About to CALL 8c9a", count, pc);
                        println!("  Current PC: 0x{:05x}", vm.pc);
                        println!("  Target routine: 0x{:05x}", unpacked);
                        println!("  Argument: {}", inst.operands[1]);
                        
                        // Execute the call and see what happens
                        vm.pc += inst.size as u32;
                        let mut interpreter = Interpreter::new(vm);
                        match interpreter.execute_instruction(&inst) {
                            Ok(_) => {
                                vm = interpreter.vm;
                                println!("  After call - New PC: 0x{:05x}", vm.pc);
                                println!("  Call stack depth: {}", vm.call_stack.len());
                                
                                if let Some(frame) = vm.call_stack.last() {
                                    println!("  New frame locals: {}", frame.num_locals);
                                    for i in 0..frame.num_locals as usize {
                                        println!("    Local {}: {}", i + 1, frame.locals[i]);
                                    }
                                }
                                
                                // Check if we're now at the routine start
                                if vm.pc == 0x8c9a {
                                    println!("  *** SUCCESS: PC is now at routine start 0x8c9a ***");
                                } else {
                                    println!("  *** PROBLEM: PC is 0x{:05x}, not 0x8c9a ***", vm.pc);
                                }
                            }
                            Err(e) => {
                                println!("  Error executing call: {}", e);
                                break;
                            }
                        }
                        continue;
                    }
                }
                
                // Check each PC after the call
                if pc >= 0x8c9a && pc <= 0x8cd0 {
                    println!("[{:04}] {:05x}: {}", count, pc, 
                            inst.format_with_version(vm.game.header.version));
                    
                    // Special check for the comparison
                    if pc == 0x08cc7 {
                        println!("  *** THIS IS THE COMPARISON ***");
                        let vd1 = vm.read_variable(0xd1)?;
                        println!("  Vd1 = {}", vd1);
                        
                        // Check if local 1 has the argument value
                        if let Ok(local1) = vm.read_variable(0x01) {
                            println!("  Local 1 = {}", local1);
                            if local1 == 1 {
                                println!("  *** Local 1 has the argument, but Vd1 is still 0! ***");
                            }
                        }
                    }
                }
                
                // Update PC and execute
                vm.pc += inst.size as u32;
                let mut interpreter = Interpreter::new(vm);
                match interpreter.execute_instruction(&inst) {
                    Ok(_) => vm = interpreter.vm,
                    Err(e) => {
                        println!("Error at PC {:05x}: {}", pc, e);
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
        
        // Stop at the critical comparison
        if pc == 0x08cc7 {
            println!("\n*** REACHED COMPARISON ***");
            let vd1 = vm.read_variable(0xd1)?;
            println!("Final Vd1 value: {}", vd1);
            break;
        }
        
        if count > 300 && after_serial {
            break;
        }
    }
    
    Ok(())
}