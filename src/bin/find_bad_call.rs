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

    println!("Looking for the bad call that leads to 0x07500...\n");
    
    // Set breakpoints
    debugger.add_breakpoint(0x8d32); // Near end of DESCRIBE-ROOM
    
    // Run to breakpoint
    match debugger.run() {
        Ok(_) => {
            println!("At end of DESCRIBE-ROOM (0x8d32)");
            
            // Enable single stepping
            debugger.set_single_step(true);
            
            // Track calls
            let mut step_count = 0;
            
            loop {
                let pc = debugger.interpreter.vm.pc;
                
                // Get current instruction
                if let Ok(disasm) = debugger.disassemble_current() {
                    // Only print calls and branches
                    if disasm.contains("call") || disasm.contains("ret") {
                        println!("{:05x}: {}", pc, disasm);
                        
                        // Check for calls that might lead to bad addresses
                        if disasm.contains("call") {
                            // Extract the packed address from the instruction
                            if let Ok(inst) = debugger.interpreter.vm.game.memory.get(pc as usize..)
                                .ok_or("PC out of bounds")
                                .and_then(|mem| infocom::instruction::Instruction::decode(mem, 0, 3)
                                    .map_err(|e| e.as_str())) {
                                
                                if !inst.operands.is_empty() {
                                    let packed = inst.operands[0];
                                    let unpacked = (packed as u32) * 2;
                                    
                                    // Check if this unpacks to a suspicious address
                                    if unpacked >= 0x07000 && unpacked <= 0x08000 {
                                        println!("  -> Unpacks to 0x{:05x}", unpacked);
                                        
                                        // Check what's at that address
                                        if (unpacked as usize) < debugger.interpreter.vm.game.memory.len() {
                                            let first_byte = debugger.interpreter.vm.game.memory[unpacked as usize];
                                            if first_byte > 15 {
                                                println!("  -> WARNING: First byte is 0x{:02x} = {} locals (max 15!)", 
                                                        first_byte, first_byte);
                                                println!("  -> This is probably not a valid routine!");
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Step
                match debugger.step() {
                    Ok(_) => {
                        step_count += 1;
                        if step_count > 50 {
                            println!("\nStopped after 50 steps");
                            break;
                        }
                    }
                    Err(e) => {
                        println!("\nError at PC 0x{:05x}: {}", pc, e);
                        
                        // Show the call stack
                        println!("\nCall stack when error occurred:");
                        for (i, frame) in debugger.interpreter.vm.call_stack.iter().enumerate() {
                            println!("  Level {}: return to 0x{:05x}, {} locals", 
                                    i, frame.return_pc, frame.num_locals);
                        }
                        
                        break;
                    }
                }
            }
        }
        Err(e) => {
            println!("Error reaching breakpoint: {}", e);
        }
    }
    
    Ok(())
}