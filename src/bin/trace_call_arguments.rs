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
    
    println!("Tracing call arguments to see if 8c9a receives argument 1...\n");
    
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
                        println!("[{:04}] {:05x}: CALL to 8c9a", count, pc);
                        println!("  Instruction: {}", inst.format_with_version(vm.game.header.version));
                        println!("  Operands: {:?}", inst.operands);
                        
                        if inst.operands.len() > 1 {
                            println!("  Argument: {} (0x{:02x})", inst.operands[1], inst.operands[1]);
                        }
                        
                        // Check the current call stack
                        println!("  Current call stack depth: {}", vm.call_stack.len());
                        if let Some(frame) = vm.call_stack.last() {
                            println!("  Current frame locals: {:?}", &frame.locals[0..frame.num_locals as usize]);
                        }
                    }
                }
                
                // Check when we enter routine 8c9a
                if pc == 0x8c9a {
                    println!("\n*** ENTERED ROUTINE 8c9a ***");
                    
                    // Check locals in this routine
                    if let Some(frame) = vm.call_stack.last() {
                        println!("  Locals count: {}", frame.num_locals);
                        for i in 0..frame.num_locals as usize {
                            println!("    Local {}: {}", i + 1, frame.locals[i]);
                        }
                    }
                    
                    // Check if argument 1 is in local variable 1
                    if let Ok(local1) = vm.read_variable(0x01) {
                        println!("  Local 1 (L01): {}", local1);
                        if local1 == 1 {
                            println!("    *** ARGUMENT 1 RECEIVED CORRECTLY! ***");
                        }
                    }
                    
                    // Check Vd1 value
                    if let Ok(vd1) = vm.read_variable(0xd1) {
                        println!("  Vd1: {}", vd1);
                        if vd1 == 0 {
                            println!("    *** BUT Vd1 IS STILL 0! ***");
                        }
                    }
                }
                
                // Check for any instruction that might copy from local to Vd1
                if let Some(store_var) = inst.store_var {
                    if store_var == 0xd1 {
                        println!("[{:04}] {:05x}: {} -> Vd1", count, pc, 
                                inst.format_with_version(vm.game.header.version));
                        
                        // Check if this is copying from a local variable
                        if let Some(op) = inst.operands.get(0) {
                            if *op <= 0x0F {
                                println!("    This might be copying from local {} to Vd1", op);
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