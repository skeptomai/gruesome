use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let mut vm = VM::new(game);
    
    println!("Tracing execution after serial number to find where LOOK should be set up...\n");
    
    let mut count = 0;
    let mut after_serial = false;
    
    loop {
        count += 1;
        let pc = vm.pc;
        
        // Mark when we pass the serial number printing
        if pc == 0x06f8c {
            after_serial = true;
            println!("*** SERIAL NUMBER PRINTED - WATCHING FOR ACTION SETUP ***\n");
        }
        
        if after_serial {
            // Decode instruction
            if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
                let name = inst.name(vm.game.header.version);
                
                // Look for anything that might set up action codes
                if let Some(store_var) = inst.store_var {
                    if store_var == 0xd1 {
                        println!("[{:04}] {:05x}: {} -> Vd1 *** ACTION CODE SETUP ***", 
                                count, pc, inst.format_with_version(vm.game.header.version));
                    }
                }
                
                // Look for calls to routines that might set up the game state
                if name.starts_with("call") && !inst.operands.is_empty() {
                    let packed = inst.operands[0];
                    let unpacked = packed * 2;
                    
                    println!("[{:04}] {:05x}: CALL to 0x{:05x}", count, pc, unpacked);
                    
                    // Check if this might be the main game loop or action setup
                    match unpacked {
                        0x07e04 => println!("         ^ Main dispatch routine"),
                        0x08c9a => println!("         ^ This leads to STAND! (bad path)"),
                        _ => {}
                    }
                }
                
                // Look for stores to key variables
                if let Some(store_var) = inst.store_var {
                    if store_var >= 0xd0 && store_var <= 0xd5 {
                        println!("[{:04}] {:05x}: {} -> V{:02x}", 
                                count, pc, inst.format_with_version(vm.game.header.version), store_var);
                    }
                }
                
                // Check for SREAD which would read player input
                if name == "sread" {
                    println!("[{:04}] {:05x}: SREAD - This would read player input!", count, pc);
                }
                
                // Update PC
                vm.pc += inst.size as u32;
                
                // Execute
                let mut interpreter = Interpreter::new(vm);
                match interpreter.execute_instruction(&inst) {
                    Ok(_) => vm = interpreter.vm,
                    Err(e) => {
                        println!("Error at PC {:05x}: {}", pc, e);
                        break;
                    }
                }
            }
        } else {
            // Execute silently before serial
            if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
                vm.pc += inst.size as u32;
                let mut interpreter = Interpreter::new(vm);
                match interpreter.execute_instruction(&inst) {
                    Ok(_) => vm = interpreter.vm,
                    Err(e) => {
                        println!("Error: {}", e);
                        break;
                    }
                }
            }
        }
        
        // Stop when we reach the problematic comparison
        if pc == 0x08cc7 {
            println!("\n*** REACHED COMPARISON AT 0x08cc7 ***");
            let vd1 = vm.read_variable(0xd1)?;
            println!("Vd1 = {} (should be > 5 for LOOK)", vd1);
            break;
        }
        
        if count > 300 && after_serial {
            println!("Stopped after 300 instructions post-serial");
            break;
        }
        
        if count > 2000 {
            println!("Stopped after 2000 total instructions");
            break;
        }
    }
    
    Ok(())
}