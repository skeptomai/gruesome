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

    println!("Tracing execution around the PRINT instructions in DESCRIBE-ROOM...\n");
    
    // Set a breakpoint at the start of DESCRIBE-ROOM
    debugger.add_breakpoint(0x08c9a);
    
    // Run until we hit it
    println!("Running until we reach DESCRIBE-ROOM at 0x08c9a...");
    match debugger.run() {
        Ok(_) => {},
        Err(e) => {
            println!("Hit error before reaching DESCRIBE-ROOM: {}", e);
            return Ok(());
        }
    }
    
    // Now single-step through the print area
    println!("\nSingle-stepping through the print area...");
    debugger.set_single_step(true);
    
    let mut count = 0;
    loop {
        count += 1;
        let pc = debugger.interpreter.vm.pc;
        
        // Show current instruction
        if let Ok(disasm) = debugger.disassemble_current() {
            println!("[{:03}] PC:{:05x} - {}", count, pc, disasm);
            
            // Stop if we hit the problematic area
            if pc >= 0x08cc0 && pc <= 0x08cd0 {
                println!("     *** IN CRITICAL AREA ***");
                
                // Show raw bytes
                let memory = &debugger.interpreter.vm.game.memory;
                print!("     Raw bytes: ");
                for i in 0..8 {
                    if pc as usize + i < memory.len() {
                        print!("{:02x} ", memory[pc as usize + i]);
                    }
                }
                println!();
            }
            
            // Check if this is around the bad instruction
            if pc == 0x08cc4 {
                println!("\n*** FOUND PC AT 0x08cc4 ***");
                println!("This should not happen - 0x08cc4 is in the middle of other instructions!");
                break;
            }
        }
        
        // Step one instruction
        match debugger.step() {
            Ok(_) => {},
            Err(e) => {
                println!("\nError at PC {:05x}: {}", pc, e);
                
                if pc == 0x08cc4 || e.contains("Failed to decode") {
                    println!("\n*** THIS IS THE DECODE ERROR ***");
                    println!("PC incorrectly jumped to 0x08cc4");
                }
                break;
            }
        }
        
        if count > 50 {
            println!("Stopped after 50 instructions in DESCRIBE-ROOM");
            break;
        }
    }
    
    Ok(())
}