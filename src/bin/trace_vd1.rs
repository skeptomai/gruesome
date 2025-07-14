use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data).map_err(|e| format!("Game load error: {}", e))?;
    let mut vm = VM::new(game);
    
    println!("Tracing variable Vd1 (0xd1) to see when it's set...\n");
    
    let mut count = 0;
    let mut last_vd1 = 0xFFFF;
    
    loop {
        count += 1;
        let pc = vm.pc;
        
        // Check Vd1 value
        let vd1 = vm.read_variable(0xd1).unwrap_or(0xFFFF);
        if vd1 != last_vd1 {
            println!("\n[{:04}] Vd1 changed from {} to {} at PC {:05x}", count, last_vd1, vd1, pc);
            last_vd1 = vd1;
        }
        
        // Decode instruction
        if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
            // Check if this instruction stores to Vd1
            if let Some(store_var) = inst.store_var {
                if store_var == 0xd1 {
                    println!("[{:04}] {:05x}: {} -> Vd1", count, pc, 
                             inst.format_with_version(vm.game.header.version));
                }
            }
            
            // Update PC
            vm.pc += inst.size as u32;
            
            // Execute
            let mut interpreter = Interpreter::new(vm);
            match interpreter.execute_instruction(&inst) {
                Ok(_) => vm = interpreter.vm,
                Err(e) => {
                    eprintln!("Error at PC {:05x}: {}", pc, e);
                    break;
                }
            }
        }
        
        // Stop when we reach the problematic comparison
        if pc == 0x08cc7 {
            println!("\n*** Reached the comparison at 0x08cc7 ***");
            println!("Vd1 = {}", vd1);
            println!("Comparing: 5 < {} = {}", vd1, 5 < vd1);
            
            // Check some other variables for context
            println!("\nOther variables at this point:");
            for var in [0x00, 0x01, 0x52, 0x6f, 0x90, 0xd0, 0xd2].iter() {
                if let Ok(val) = vm.read_variable(*var) {
                    println!("  V{:02x}: {}", var, val);
                }
            }
            
            break;
        }
        
        if count > 5000 {
            println!("\nStopped after 5000 instructions");
            break;
        }
    }

    Ok(())
}