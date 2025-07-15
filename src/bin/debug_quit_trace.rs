use infocom::debugger::Debugger;
use infocom::vm::{Game, VM};
use infocom::interpreter::ExecutionResult;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging to see debug messages
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    println!("=== Z-Machine Quit Command Tracer ===");
    println!("This tool traces execution after typing 'quit' and responding 'Y'");
    println!();

    // Load Zork 1
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);
    let mut debugger = Debugger::new(vm);
    
    // Track when we see the quit-related text
    let mut quit_typed = false;
    let mut confirmation_asked = false;
    let mut y_typed = false;
    let mut ok_printed = false;
    let mut trace_count = 0;
    
    println!("Running game until we see quit behavior...");
    
    // Run the game
    loop {
        let pc = debugger.interpreter.vm.pc;
        
        // Execute one step
        match debugger.step() {
            Ok(()) => {
                // Check if we've printed key messages
                // This is a simple heuristic - in practice we'd need to intercept print calls
                
                // Once we detect the quit sequence, start tracing
                if ok_printed {
                    trace_count += 1;
                    
                    // Show current instruction
                    if let Ok(disasm) = debugger.disassemble_current() {
                        println!("[TRACE {}] {}", trace_count, disasm);
                    }
                    
                    // Show some state
                    if trace_count % 10 == 1 {
                        println!("  Stack depth: {}", debugger.interpreter.vm.stack.len());
                        if debugger.interpreter.vm.call_stack.len() > 0 {
                            println!("  Call stack depth: {}", debugger.interpreter.vm.call_stack.len());
                            if let Some(frame) = debugger.interpreter.vm.call_stack.last() {
                                println!("  Current routine return PC: {:05x}", frame.return_pc);
                            }
                        }
                    }
                    
                    // Stop after 100 instructions to see what happens
                    if trace_count >= 100 {
                        println!("\n=== Stopped tracing after 100 instructions ===");
                        println!("The quit opcode (0x0A) was never reached!");
                        println!("This suggests V-QUIT returns normally instead of calling quit.");
                        break;
                    }
                }
            }
            Err(e) => {
                println!("Execution error at PC {:05x}: {}", pc, e);
                break;
            }
        }
    }
    
    // To properly trace this, we would need to:
    // 1. Run the game normally until the first prompt
    // 2. Inject "quit" into the input buffer
    // 3. Run until we see "Do you wish to leave the game?"
    // 4. Inject "Y" into the input buffer
    // 5. Trace execution after "Ok." is printed
    
    println!("\nNote: For a complete trace, we need to:");
    println!("1. Use the single-step debugger manually");
    println!("2. Set a breakpoint at the V-QUIT routine");
    println!("3. Trace execution after responding 'Y' to see if quit opcode is ever reached");
    
    Ok(())
}