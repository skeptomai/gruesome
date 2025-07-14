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

    println!("Investigating the MOD instruction error...\n");
    
    // The problem: We're executing at 0x07554 which is in the middle of an instruction
    // This suggests we jumped here incorrectly
    
    // Let's set a breakpoint just after "West of House" is printed
    debugger.add_breakpoint(0x8d33); // After the print in DESCRIBE-ROOM
    
    println!("Running to breakpoint...");
    match debugger.run() {
        Ok(_) => {
            println!("Reached breakpoint after 'West of House' print");
            
            // Enable single stepping
            debugger.single_step(true);
            
            println!("\nSingle stepping to find the bad jump...");
            
            let mut step_count = 0;
            let mut last_pc = 0;
            
            loop {
                let pc = debugger.interpreter.vm.pc;
                
                // Check if we made a suspicious jump
                if pc >= 0x07000 && pc <= 0x08000 && (pc as i32 - last_pc as i32).abs() > 100 {
                    println!("\nSUSPICIOUS JUMP: from 0x{:05x} to 0x{:05x}", last_pc, pc);
                    
                    // Show what instruction caused this
                    if last_pc > 0 {
                        if let Ok(inst) = infocom::instruction::Instruction::decode(
                            &debugger.interpreter.vm.game.memory, last_pc as usize, 3) {
                            println!("  Last instruction: {}", inst.format_with_version(3));
                        }
                    }
                }
                
                // Disassemble current
                match debugger.disassemble_current() {
                    Ok(disasm) => {
                        // Only show calls, jumps, and returns
                        if disasm.contains("call") || disasm.contains("jump") || 
                           disasm.contains("ret") || pc == 0x07554 {
                            println!("{:05x}: {}", pc, disasm);
                        }
                        
                        last_pc = pc;
                    }
                    Err(_) => {}
                }
                
                // Step
                match debugger.step() {
                    Ok(_) => {
                        step_count += 1;
                        if step_count > 100 {
                            println!("\nStopped after 100 steps");
                            break;
                        }
                    }
                    Err(e) => {
                        println!("\nError at PC 0x{:05x}: {}", pc, e);
                        
                        if pc == 0x07554 {
                            println!("\nThis is the problematic MOD instruction!");
                            
                            // What called us here?
                            println!("\nCall stack:");
                            for (i, frame) in debugger.interpreter.vm.call_stack.iter().enumerate() {
                                println!("  Level {}: return to 0x{:05x}", i, frame.return_pc);
                                
                                // What routine did we call from?
                                if i > 0 {
                                    // Try to find the routine start
                                    let mut search_pc = frame.return_pc.saturating_sub(100);
                                    while search_pc < frame.return_pc {
                                        if let Ok(locals) = debugger.interpreter.vm.game.memory
                                            .get(search_pc as usize)
                                            .map(|&b| b) {
                                            if locals <= 15 {
                                                // Might be a routine start
                                                println!("    (possibly from routine at 0x{:05x})", search_pc);
                                                break;
                                            }
                                        }
                                        search_pc += 1;
                                    }
                                }
                            }
                        }
                        
                        break;
                    }
                }
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    
    Ok(())
}