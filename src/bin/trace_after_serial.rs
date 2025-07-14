use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use std::fs::File;
use std::io::prelude::*;
use env_logger;

fn main() -> std::io::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    interpreter.set_debug(true);

    // Track when we've printed the serial number
    let mut serial_printed = false;
    let mut post_serial_instructions = 0;

    loop {
        let pc = interpreter.vm.pc;
        
        // Check if serial number has been printed by looking for the print_num at 0x06f8c
        if pc == 0x06f8c {
            serial_printed = true;
            println!("\n=== SERIAL NUMBER PRINTED, NOW TRACING EXECUTION ===");
        }
        
        if serial_printed {
            post_serial_instructions += 1;
            
            // Dump some state info
            if post_serial_instructions % 50 == 1 {
                println!("\n=== STATE AFTER {} INSTRUCTIONS ===", post_serial_instructions);
                
                // Current location (global 0)
                if let Ok(location) = interpreter.vm.read_variable(16) {
                    println!("Current location (G00): {}", location);
                    
                    // Try to get object name
                    if location > 0 && location < 256 {
                        print!("Location name: ");
                        if let Err(e) = interpreter.vm.print_object_name(location) {
                            println!("Error: {}", e);
                        } else {
                            println!();
                        }
                    }
                }
                
                // LIT variable
                if let Ok(lit) = interpreter.vm.read_variable(0x52 + 16) {
                    println!("LIT (G52): {}", lit);
                }
                
                println!("PC: {:05x}", pc);
                println!();
            }
            
            // Stop after 500 instructions to avoid too much output
            if post_serial_instructions > 500 {
                println!("\n=== STOPPING TRACE AFTER 500 POST-SERIAL INSTRUCTIONS ===");
                break;
            }
        }

        match interpreter.step() {
            Ok(infocom::interpreter::ExecutionResult::Quit) => {
                println!("Game quit normally");
                break;
            }
            Ok(infocom::interpreter::ExecutionResult::GameOver) => {
                println!("Game over");
                break;
            }
            Ok(_) => {
                // Continue execution
            }
            Err(e) => {
                eprintln!("Execution error at PC {:05x}: {}", pc, e);
                
                // Dump some context
                println!("\nError context:");
                if let Ok(inst) = infocom::instruction::Instruction::decode(
                    &interpreter.vm.game.memory,
                    pc as usize,
                    interpreter.vm.game.header.version,
                ) {
                    println!("Instruction: {}", inst.format_with_version(interpreter.vm.game.header.version));
                }
                
                break;
            }
        }
    }

    Ok(())
}