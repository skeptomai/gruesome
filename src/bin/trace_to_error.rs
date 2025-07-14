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

    println!("Tracing to find what leads to PC 0x07554...\n");
    
    // Run and see what happens
    debugger.add_breakpoint(0x8d33); // After print
    
    match debugger.run() {
        Ok(_) => {
            println!("At breakpoint 0x8d33");
            
            // Keep history of PCs
            let mut pc_history = Vec::new();
            
            loop {
                let pc = debugger.interpreter.vm.pc;
                pc_history.push(pc);
                
                // Keep only last 20
                if pc_history.len() > 20 {
                    pc_history.remove(0);
                }
                
                // Check if we're about to hit the error
                if pc == 0x07554 {
                    println!("\nAbout to execute problematic instruction at 0x07554!");
                    println!("\nLast 20 PCs:");
                    for &hist_pc in &pc_history {
                        println!("  0x{:05x}", hist_pc);
                    }
                    
                    println!("\nCall stack:");
                    for (i, frame) in debugger.interpreter.vm.call_stack.iter().enumerate() {
                        println!("  Level {}: return to 0x{:05x}, locals: {}", 
                                i, frame.return_pc, frame.num_locals);
                    }
                    
                    break;
                }
                
                // Step
                match debugger.step() {
                    Ok(_) => {},
                    Err(e) => {
                        println!("\nError at PC 0x{:05x}: {}", pc, e);
                        
                        if e.contains("requires 2 operands") {
                            println!("\nLast 20 PCs before error:");
                            for &hist_pc in &pc_history {
                                println!("  0x{:05x}", hist_pc);
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