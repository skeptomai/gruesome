use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use infocom::debugger::Debugger;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the game
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);
    let interpreter = Interpreter::new(vm);
    let mut debugger = Debugger::new(interpreter);
    
    println!("Setting breakpoint at V-QUIT (0x6dcc)...\n");
    debugger.add_breakpoint(0x6dcc);
    
    // Run until we hit V-QUIT
    println!("Type 'quit' when the game starts...\n");
    
    match debugger.run() {
        Ok(_) => {
            println!("\nHit V-QUIT routine!");
            println!("Initial locals: L00={}, L01={}", 1, 0); // From routine header
            
            // The first instruction at 6dd1 calls 0x10582
            println!("\nFirst instruction: CALL 10582 -> -(SP)");
            println!("This call might modify L00...");
            
            // Step through and watch what happens
            println!("\nStepping through V-QUIT to see what happens to L00...");
            
            for i in 0..20 {
                match debugger.step() {
                    Ok(_) => {
                        let pc = debugger.interpreter.vm.pc;
                        if let Ok(inst) = debugger.disassemble_at(pc) {
                            println!("{:05x}: {}", pc, inst);
                            
                            // Check if we're at key points
                            if pc == 0x6dd6 {
                                println!("  -> About to check if result of CALL 10582 is zero");
                            } else if pc == 0x6dfc {
                                println!("  -> About to call yes/no prompt routine");
                            } else if pc == 0x6e04 {
                                println!("  -> About to check L00 before QUIT");
                                // Try to check L00 value
                                if let Some(frame) = debugger.interpreter.vm.call_stack.last() {
                                    println!("     L00 = {}", frame.locals[0]);
                                    if frame.locals[0] != 0 {
                                        println!("     !! L00 is not zero, will skip QUIT !!");
                                    }
                                }
                            } else if pc == 0x6e07 {
                                println!("  -> QUIT opcode!");
                            } else if pc == 0x6e09 {
                                println!("  -> Printing 'Ok.' - QUIT was skipped");
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error: {}", e);
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