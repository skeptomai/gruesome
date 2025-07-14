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
    
    println!("Tracing routine calls after serial number...\n");
    
    let mut count = 0;
    let mut after_serial = false;
    let mut call_depth = 0;
    
    loop {
        count += 1;
        let pc = vm.pc;
        
        if pc == 0x06f8c {
            after_serial = true;
            println!("\n*** SERIAL NUMBER PRINTED ***\n");
        }
        
        if after_serial && count < 200 {
            // Decode instruction
            if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
                let name = inst.name(vm.game.header.version);
                
                // Track calls
                if name.starts_with("call") && !inst.operands.is_empty() {
                    let packed = inst.operands[0];
                    let unpacked = packed * 2;
                    
                    println!("{:indent$}[{:04}] {:05x}: CALL to 0x{:05x}", "", count, pc, unpacked, indent = call_depth * 2);
                    call_depth += 1;
                    
                    // Special annotations for known routines
                    match unpacked {
                        0x07e04 => println!("{:indent$}      ^ This seems to be a main dispatch routine", "", indent = call_depth * 2),
                        0x08c9a => println!("{:indent$}      ^ This routine leads to STAND message!", "", indent = call_depth * 2),
                        0x086ca => println!("{:indent$}      ^ STAND routine", "", indent = call_depth * 2),
                        _ => {}
                    }
                }
                
                // Track returns
                if name == "ret" || name == "rtrue" || name == "rfalse" {
                    if call_depth > 0 {
                        call_depth -= 1;
                    }
                    println!("{:indent$}[{:04}] {:05x}: RETURN", "", count, pc, indent = call_depth * 2);
                }
                
                // Show branches for context
                if inst.branch.is_some() && call_depth > 0 {
                    println!("{:indent$}[{:04}] {:05x}: {} (branch)", "", count, pc, 
                             inst.format_with_version(vm.game.header.version), indent = call_depth * 2);
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
        } else {
            // Execute silently before serial
            if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
                vm.pc += inst.size as u32;
                let mut interpreter = Interpreter::new(vm);
                match interpreter.execute_instruction(&inst) {
                    Ok(_) => vm = interpreter.vm,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        break;
                    }
                }
            }
        }
        
        if count > 200 && after_serial {
            break;
        }
        
        if count > 5000 {
            break;
        }
    }

    Ok(())
}