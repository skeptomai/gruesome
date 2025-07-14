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

    println!("Tracing path to WEST-HOUSE routine at 0x953c...\n");
    
    // Set breakpoints at key locations
    debugger.add_breakpoint(0x8c9a); // DESCRIBE-ROOM start
    debugger.add_breakpoint(0x8d32); // Near end of DESCRIBE-ROOM  
    debugger.add_breakpoint(0x953c); // WEST-HOUSE routine
    
    // Run until we hit a breakpoint
    println!("Running to first breakpoint...");
    
    let mut hit_describe_room = false;
    let mut instruction_count = 0;
    
    loop {
        match debugger.run() {
            Ok(_) => {
                let pc = debugger.interpreter.vm.pc;
                
                if pc == 0x8c9a {
                    println!("\nHit DESCRIBE-ROOM at 0x8c9a");
                    hit_describe_room = true;
                    debugger.single_step(true); // Enable single stepping
                } else if pc == 0x8d32 {
                    println!("\nNear end of DESCRIBE-ROOM at 0x8d32");
                } else if pc == 0x953c {
                    println!("\n*** SUCCESS! Reached WEST-HOUSE at 0x953c ***");
                    break;
                }
                
                if hit_describe_room {
                    instruction_count = 0;
                    // Single step and watch for problems
                    println!("\nSingle-stepping from here...");
                    
                    loop {
                        let pc = debugger.interpreter.vm.pc;
                        
                        // Disassemble current instruction
                        if let Ok(disasm) = debugger.disassemble_current() {
                            println!("{:05x}: {}", pc, disasm);
                            
                            // Check if this is a suspicious call
                            if disasm.contains("call") && (disasm.contains("3a80") || disasm.contains("438e")) {
                                println!("  ^^^ Suspicious call detected!");
                                
                                // Check what those addresses unpack to
                                if disasm.contains("3a80") {
                                    let unpacked = 0x3a80u32 * 2;
                                    println!("  0x3a80 unpacks to 0x{:05x}", unpacked);
                                }
                                if disasm.contains("438e") {
                                    let unpacked = 0x438eu32 * 2;
                                    println!("  0x438e unpacks to 0x{:05x}", unpacked);
                                }
                            }
                        }
                        
                        instruction_count += 1;
                        
                        // Step
                        match debugger.step() {
                            Ok(_) => {},
                            Err(e) => {
                                println!("\nError at PC 0x{:05x}: {}", pc, e);
                                
                                if e.contains("requires 2 operands") {
                                    println!("\nThis is the MOD instruction error");
                                    
                                    // Show call stack
                                    println!("\nCall stack:");
                                    for (i, frame) in debugger.interpreter.vm.call_stack.iter().enumerate() {
                                        println!("  {}: return to 0x{:05x}", i, frame.return_pc);
                                    }
                                }
                                
                                return Ok(());
                            }
                        }
                        
                        // Stop if we've gone too far
                        if instruction_count > 100 {
                            println!("\nStopped after 100 instructions");
                            break;
                        }
                        
                        // Check if we reached WEST-HOUSE
                        if debugger.interpreter.vm.pc == 0x953c {
                            println!("\n*** SUCCESS! Reached WEST-HOUSE at 0x953c ***");
                            return Ok(());
                        }
                    }
                }
            }
            Err(e) => {
                println!("Error before reaching DESCRIBE-ROOM: {}", e);
                return Ok(());
            }
        }
    }
    
    Ok(())
}