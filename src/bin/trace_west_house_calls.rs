use infocom::debugger::Debugger;
use infocom::vm::{Game, VM};
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);
    let mut debugger = Debugger::new(vm);
    
    println!("Tracing calls to WEST-HOUSE routine (0x953c)...\n");
    
    // Set breakpoints at key locations
    debugger.add_breakpoint(0x953c); // WEST-HOUSE routine
    debugger.add_breakpoint(0x8c9a); // DESCRIBE-ROOM routine
    
    println!("Running until DESCRIBE-ROOM or WEST-HOUSE...");
    
    let mut call_count = 0;
    loop {
        match debugger.run() {
            Ok(_) => {
                let pc = debugger.interpreter.vm.pc;
                call_count += 1;
                
                if pc == 0x8c9a {
                    println!("\n=== Hit DESCRIBE-ROOM at 0x8c9a ===");
                    
                    // Check current locals and stack
                    println!("Stack depth: {}", debugger.interpreter.vm.stack.len());
                    if !debugger.interpreter.vm.call_stack.is_empty() {
                        let frame = &debugger.interpreter.vm.call_stack.last().unwrap();
                        println!("Current frame locals: {:?}", &frame.locals[0..4]);
                    }
                    
                    // Continue and look for calls
                    debugger.set_single_step(true);
                    println!("Single-stepping to look for WEST-HOUSE call...");
                    
                    for _ in 0..100 {
                        if let Ok(disasm) = debugger.disassemble_current() {
                            let pc = debugger.interpreter.vm.pc;
                            
                            // Look for calls to WEST-HOUSE (0x953c)
                            if disasm.contains("call") && (disasm.contains("953c") || disasm.contains("2583")) {
                                println!("\nFound call at PC 0x{:05x}: {}", pc, disasm);
                                
                                // Check the argument being passed
                                if let Ok(inst) = infocom::instruction::Instruction::decode(
                                    &debugger.interpreter.vm.game.memory, pc as usize, 3) {
                                    println!("  Call operands: {:?}", inst.operands);
                                    if inst.operands.len() > 1 {
                                        println!("  Argument: {} (should be 3 for M-LOOK)", inst.operands[1]);
                                    }
                                }
                                break;
                            }
                            
                            // Look for any calls
                            if disasm.contains("call") {
                                println!("Call at PC 0x{:05x}: {}", pc, disasm);
                            }
                        }
                        
                        match debugger.step() {
                            Ok(_) => {},
                            Err(e) => {
                                println!("Step error: {}", e);
                                break;
                            }
                        }
                    }
                    break;
                    
                } else if pc == 0x953c {
                    println!("\n=== Hit WEST-HOUSE at 0x953c ===");
                    
                    // Check the argument passed to WEST-HOUSE
                    if !debugger.interpreter.vm.call_stack.is_empty() {
                        let frame = &debugger.interpreter.vm.call_stack.last().unwrap();
                        println!("WEST-HOUSE called with locals: {:?}", &frame.locals[0..4]);
                        println!("Local 0 (argument): {} (should be 3 for M-LOOK)", frame.locals[0]);
                    }
                    
                    if call_count > 2 {
                        break; // Don't loop forever
                    }
                }
                
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        }
    }
    
    Ok(())
}