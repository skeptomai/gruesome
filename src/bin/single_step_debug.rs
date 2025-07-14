use infocom::debugger::Debugger;
use infocom::vm::{Game, VM};
use std::env;
use std::fs::File;
use std::io::{self, prelude::*, BufRead};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <game_file.dat>", args[0]);
        eprintln!("Example: {} resources/test/zork1/DATA/ZORK1.DAT", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    println!("Loading Z-Machine game: {}", filename);

    // Read the game file
    let mut f = File::open(filename)?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    // Create VM and debugger
    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);
    let mut debugger = Debugger::new(vm);

    println!("Z-Machine Single-Step Debugger v0.1.0");
    println!("Game version: {}", debugger.interpreter.vm.game.header.version);
    println!("Initial PC: {:05x}", debugger.interpreter.vm.game.header.initial_pc);
    println!();
    println!("Commands:");
    println!("  n, next, step - Execute next instruction");
    println!("  c, continue   - Continue execution until breakpoint or error");
    println!("  s, state      - Show VM state (variables, stack, etc)");
    println!("  d, disasm     - Show disassembly around current PC");
    println!("  b <addr>      - Set breakpoint at address (hex)");
    println!("  rb <addr>     - Remove breakpoint at address (hex)");
    println!("  bl            - List all breakpoints");
    println!("  h, history    - Show instruction history");
    println!("  q, quit       - Quit debugger");
    println!();
    println!("Starting in single-step mode...");
    println!();

    // Start in single-step mode
    debugger.set_single_step(true);
    
    let stdin = io::stdin();
    let mut step_count = 0;
    
    loop {
        step_count += 1;
        let pc = debugger.interpreter.vm.pc;
        
        // Show current instruction
        match debugger.disassemble_current() {
            Ok(disasm) => {
                println!("[{:04}] {}", step_count, disasm);
            }
            Err(e) => {
                println!("Error disassembling current instruction: {}", e);
                break;
            }
        }
        
        // Wait for user input
        print!("(step) ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        match stdin.lock().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim().to_lowercase();
                
                match input.as_str() {
                    "" | "n" | "next" | "step" => {
                        // Execute one step
                        match debugger.step() {
                            Ok(_) => {
                                // Continue to next iteration
                            }
                            Err(e) => {
                                println!("Execution error: {}", e);
                                break;
                            }
                        }
                    }
                    "c" | "continue" => {
                        // Turn off single-step and continue
                        debugger.set_single_step(false);
                        println!("Continuing execution...");
                        match debugger.run() {
                            Ok(()) => {
                                println!("Execution completed.");
                                break;
                            }
                            Err(e) => {
                                println!("Execution error: {}", e);
                                // Turn single-step back on and show current state
                                debugger.set_single_step(true);
                                println!("Returned to single-step mode.");
                            }
                        }
                    }
                    "s" | "state" => {
                        debugger.show_state();
                    }
                    "d" | "disasm" => {
                        let current_pc = debugger.interpreter.vm.pc;
                        let start_pc = current_pc.saturating_sub(40);
                        let lines = debugger.disassemble_range(start_pc, 20);
                        println!("Disassembly around PC {:05x}:", current_pc);
                        for line in lines {
                            let marker = if line.starts_with(&format!("{:05x}", current_pc)) { 
                                " --> " 
                            } else { 
                                "     " 
                            };
                            println!("{}{}", marker, line);
                        }
                    }
                    input if input.starts_with("b ") => {
                        let addr_str = &input[2..];
                        match u32::from_str_radix(addr_str, 16) {
                            Ok(addr) => {
                                debugger.add_breakpoint(addr);
                                println!("Breakpoint added at 0x{:05x}", addr);
                            }
                            Err(_) => {
                                println!("Invalid address: {}", addr_str);
                            }
                        }
                    }
                    input if input.starts_with("rb ") => {
                        let addr_str = &input[3..];
                        match u32::from_str_radix(addr_str, 16) {
                            Ok(addr) => {
                                debugger.remove_breakpoint(addr);
                                println!("Breakpoint removed from 0x{:05x}", addr);
                            }
                            Err(_) => {
                                println!("Invalid address: {}", addr_str);
                            }
                        }
                    }
                    "bl" => {
                        debugger.list_breakpoints();
                    }
                    "h" | "history" => {
                        debugger.show_history(20);
                    }
                    "q" | "quit" => {
                        println!("Goodbye!");
                        break;
                    }
                    _ => {
                        println!("Unknown command: {}", input);
                        println!("Type 'h' for help or 'q' to quit");
                    }
                }
            }
            Err(e) => {
                println!("Error reading input: {}", e);
                break;
            }
        }
    }

    Ok(())
}