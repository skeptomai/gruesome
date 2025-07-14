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

    println!("Tracing changes to V04 to understand the calculation bug...\n");
    
    let mut count = 0;
    let mut prev_v04 = 0;
    let mut v04_initialized = false;
    
    loop {
        count += 1;
        let pc = debugger.interpreter.vm.pc;
        
        // Read V04 before executing
        let v04_before = debugger.interpreter.vm.read_variable(0x04).unwrap_or(0);
        
        // Check for changes in V04
        if !v04_initialized {
            prev_v04 = v04_before;
            v04_initialized = true;
        }
        
        // Get current instruction
        if let Ok(disasm) = debugger.disassemble_current() {
            // Check if this instruction involves V04
            if disasm.contains("V04") || disasm.contains("-> V04") {
                println!("[{:03}] {}", count, disasm);
                println!("     V04 before: {}", v04_before);
            }
        }
        
        // Step one instruction
        match debugger.step() {
            Ok(_) => {
                // Check V04 after execution
                let v04_after = debugger.interpreter.vm.read_variable(0x04).unwrap_or(0);
                
                if v04_after != v04_before {
                    println!("     V04 after:  {} (changed by {})", v04_after, v04_after as i32 - v04_before as i32);
                    
                    // Also show other relevant variables
                    if let Ok(v92) = debugger.interpreter.vm.read_variable(0x92) {
                        if let Ok(v94) = debugger.interpreter.vm.read_variable(0x94) {
                            println!("     V92={}, V94={}, V94+V92={}", v92, v94, v94.wrapping_add(v92));
                        }
                    }
                    println!();
                    
                    prev_v04 = v04_after;
                }
                
                // Stop once we've seen the first few V04 changes in the loop
                if pc == 0x05499 && v04_after != v04_before {
                    static mut LOOP_CHANGES: i32 = 0;
                    unsafe {
                        LOOP_CHANGES += 1;
                        if LOOP_CHANGES >= 3 {
                            println!("Stopping after tracking first few changes in the loop");
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                println!("Error: {}", e);
                break;
            }
        }
        
        if count > 200 {
            println!("Stopped after 200 instructions");
            break;
        }
    }
    
    Ok(())
}