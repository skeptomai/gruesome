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

    println!("Running game to check error...\n");
    
    // Just run without breakpoints and see what happens
    match debugger.run() {
        Ok(_) => {
            println!("Game completed successfully?");
        }
        Err(e) => {
            println!("Error: {}", e);
            
            let pc = debugger.interpreter.vm.pc;
            println!("Error occurred at PC: 0x{:05x}", pc);
            
            if pc == 0x07554 {
                println!("\nThis is the MOD instruction error we're investigating!");
                
                // Show call stack
                println!("\nCall stack:");
                for (i, frame) in debugger.interpreter.vm.call_stack.iter().enumerate() {
                    println!("  Level {}: return to 0x{:05x}, locals: {}", 
                            i, frame.return_pc, frame.num_locals);
                    
                    // If this frame has suspiciously many locals, it's probably wrong
                    if frame.num_locals > 15 {
                        println!("    WARNING: {} locals is too many!", frame.num_locals);
                    }
                }
                
                // Check if we're in a bad routine
                let memory = &debugger.interpreter.vm.game.memory;
                
                // Look backwards from PC to find routine start
                for back in 1..100 {
                    let check_addr = pc.saturating_sub(back);
                    if (check_addr as usize) < memory.len() {
                        let num_locals = memory[check_addr as usize];
                        if num_locals <= 15 {
                            println!("\nPossible routine start at 0x{:05x} with {} locals", 
                                    check_addr, num_locals);
                            break;
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}