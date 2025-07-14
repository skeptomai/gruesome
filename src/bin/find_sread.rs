use std::fs::File;
use std::io::prelude::*;
use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use infocom::instruction::Instruction;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Finding SREAD instruction that reads player input...\n");
    
    // Search for SREAD instruction (opcode 0xE4 in Variable form)
    for i in 0..game_data.len()-5 {
        let byte = game_data[i];
        
        // Check for SREAD instruction
        if byte == 0xE4 {
            println!("Found SREAD at 0x{:05x}", i);
            
            // Show context
            print!("  Context: ");
            for j in i..(i+10).min(game_data.len()) {
                print!("{:02x} ", game_data[j]);
            }
            println!();
            
            // Check if this is in a routine that might be called regularly
            // Look for routine header before this
            let mut search_pc = i;
            let mut found_routine = false;
            
            while search_pc > 0 && search_pc > i.saturating_sub(200) && !found_routine {
                search_pc -= 1;
                
                // Check for routine header pattern
                if search_pc > 0 && game_data[search_pc - 1] >= 0xB0 && game_data[search_pc - 1] <= 0xBF {
                    let locals = game_data[search_pc];
                    if locals <= 15 {
                        println!("    In routine starting at 0x{:05x} with {} locals", search_pc, locals);
                        found_routine = true;
                    }
                }
            }
            
            println!();
        }
    }
    
    println!("\nNow let's trace execution to see if we reach any SREAD...");
    
    let game = Game::from_memory(game_data).map_err(|e| format!("Game load error: {}", e))?;
    let mut vm = VM::new(game);
    
    let mut count = 0;
    let mut after_comparison = false;
    
    loop {
        count += 1;
        let pc = vm.pc;
        
        // Stop tracing after the problematic comparison
        if pc == 0x08cc7 {
            println!("Reached comparison at 0x08cc7, stopping trace");
            break;
        }
        
        // Decode instruction
        if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
            let name = inst.name(vm.game.header.version);
            
            // Check for SREAD
            if name == "sread" {
                println!("[{:04}] {:05x}: SREAD instruction found!", count, pc);
                println!("  Instruction: {}", inst.format_with_version(vm.game.header.version));
                
                // This is where the game would normally read input
                // Let's check what happens after it
                println!("  This is where player input would be read and parsed");
                break;
            }
            
            // Also check for any instruction that stores to Vd1
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
        
        if count > 1000 {
            println!("Stopped after 1000 instructions - no SREAD found in main flow");
            break;
        }
    }
    
    println!("\nConclusion: The game expects player input to set action codes,");
    println!("but we're not reaching the SREAD instruction in the normal flow.");
    println!("The game should wait for input and parse it into action codes.");

    Ok(())
}