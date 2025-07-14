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

    println!("Tracing the bad call value...\n");
    
    // Set a breakpoint at the get_prop instruction
    debugger.add_breakpoint(0x8d32); // get_prop V10, #0011 -> V00
    
    println!("Running to get_prop instruction...");
    match debugger.run() {
        Ok(_) => {
            println!("\nAt get_prop instruction (0x8d32)");
            
            // Check current values
            let v10 = debugger.interpreter.vm.read_variable(0x10).unwrap_or(0);
            println!("V10 (object) = {}", v10);
            
            // Single step through the get_prop
            println!("\nStepping through get_prop...");
            match debugger.step() {
                Ok(_) => {
                    // Now check V00
                    let v00 = debugger.interpreter.vm.read_variable(0x00).unwrap_or(0);
                    println!("After get_prop: V00 = 0x{:04x}", v00);
                    
                    if v00 != 0 {
                        let unpacked = (v00 as u32) * 2;
                        println!("  This would unpack to: 0x{:05x}", unpacked);
                        
                        if unpacked == 0x07500 {
                            println!("  *** This is our problematic address! ***");
                        }
                    }
                    
                    // Check what property 0x11 of object V10 contains
                    println!("\nChecking property 0x11 of object {}:", v10);
                    
                    // Get object address
                    let obj_table_addr = debugger.interpreter.vm.game.header.object_table_addr as usize;
                    let property_defaults = obj_table_addr;
                    let obj_tree_base = property_defaults + 31 * 2;
                    
                    if v10 > 0 && v10 <= 255 {
                        let obj_addr = obj_tree_base + ((v10 - 1) as usize * 9);
                        let prop_table_addr = debugger.interpreter.vm.read_word((obj_addr + 7) as u32) as usize;
                        
                        println!("  Object {} property table at: 0x{:04x}", v10, prop_table_addr);
                        
                        // The issue might be in how we're reading property 0x11
                        // Let's see what the next instruction (call) will do
                        println!("\nNext instruction should be call V00...");
                        
                        match debugger.step() {
                            Ok(_) => {
                                println!("Stepped to next instruction");
                                
                                // Where are we now?
                                let pc = debugger.interpreter.vm.pc;
                                println!("Current PC: 0x{:05x}", pc);
                                
                                if pc == 0x07500 || pc == 0x07554 {
                                    println!("*** We jumped to the bad address! ***");
                                }
                            }
                            Err(e) => {
                                println!("Error on call: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Error on get_prop: {}", e);
                }
            }
        }
        Err(e) => {
            println!("Error reaching breakpoint: {}", e);
        }
    }
    
    Ok(())
}