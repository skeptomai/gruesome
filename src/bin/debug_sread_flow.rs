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

    println!("Debugging SREAD flow...\n");
    
    // Find SREAD instructions in the main loop
    println!("Looking for SREAD instructions after main loop starts...");
    
    // Set a breakpoint at the main command loop
    debugger.add_breakpoint(0x552a); // Main command loop from earlier analysis
    
    println!("Running to main command loop...");
    match debugger.run() {
        Ok(_) => {
            println!("Reached main command loop at 0x552a");
            
            // Now look for SREAD
            debugger.set_single_step(true);
            
            let mut instruction_count = 0;
            let mut found_sread = false;
            
            loop {
                let pc = debugger.interpreter.vm.pc;
                
                if let Ok(disasm) = debugger.disassemble_current() {
                    if disasm.contains("sread") {
                        println!("\nFound SREAD at PC 0x{:05x}: {}", pc, disasm);
                        found_sread = true;
                        
                        // Check the operands
                        if let Ok(inst) = infocom::instruction::Instruction::decode(
                            &debugger.interpreter.vm.game.memory, pc as usize, 3) {
                            println!("  Operands: {:?}", inst.operands);
                            if inst.operands.len() >= 2 {
                                let text_buffer = inst.operands[0];
                                let parse_buffer = inst.operands[1];
                                println!("  Text buffer: 0x{:04x}", text_buffer);
                                println!("  Parse buffer: 0x{:04x}", parse_buffer);
                                
                                // Check what's at these addresses
                                let max_len = debugger.interpreter.vm.read_byte(text_buffer as u32);
                                println!("  Max input length: {}", max_len);
                            }
                        }
                        
                        break;
                    }
                }
                
                // Step
                match debugger.step() {
                    Ok(_) => {
                        instruction_count += 1;
                        if instruction_count > 1000 && !found_sread {
                            println!("\nNo SREAD found in first 1000 instructions");
                            break;
                        }
                    }
                    Err(e) => {
                        println!("\nError: {}", e);
                        break;
                    }
                }
            }
        }
        Err(e) => {
            println!("Error reaching command loop: {}", e);
        }
    }
    
    Ok(())
}