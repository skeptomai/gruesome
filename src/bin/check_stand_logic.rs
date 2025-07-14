use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data).map_err(|e| format!("Game load error: {}", e))?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);

    let mut count = 0;
    let mut watch_g6f = false;
    
    println!("Tracing to understand STAND routine logic...\n");
    
    loop {
        count += 1;
        let pc = interpreter.vm.pc;
        
        // Start watching after serial
        if pc == 0x06f8c {
            watch_g6f = true;
        }
        
        // Monitor G6f changes
        if watch_g6f && count % 50 == 0 {
            if let Ok(g6f_val) = interpreter.vm.read_variable(16 + 0x6f) {
                println!("[{:04}] G6f = {} at PC {:05x}", count, g6f_val, pc);
                
                if g6f_val > 0 && g6f_val < 256 {
                    // Get parent of G6f
                    if let Ok(parent) = interpreter.vm.get_parent(g6f_val) {
                        println!("       Parent of G6f (obj {}): {}", g6f_val, parent);
                        
                        // Check attribute 0x1b (27) on parent
                        if parent > 0 {
                            if let Ok(has_attr) = interpreter.vm.test_attribute(parent as u16, 0x1b) {
                                println!("       Parent {} has attribute 0x1b: {}", parent, has_attr);
                            }
                        }
                    }
                }
            }
        }
        
        // Decode instruction
        if let Ok(inst) = Instruction::decode(
            &interpreter.vm.game.memory,
            pc as usize,
            interpreter.vm.game.header.version,
        ) {
            // Watch for writes to G6f
            if let Some(store_var) = inst.store_var {
                if store_var == 0x6f + 16 { // G6f in variable numbering
                    println!("\n[{:04}] WRITING TO G6f at PC {:05x}", count, pc);
                    println!("       Instruction: {}", inst.format_with_version(interpreter.vm.game.header.version));
                }
            }
            
            // Watch for get_parent on G6f
            let name = inst.name(interpreter.vm.game.header.version);
            if name == "get_parent" && inst.operands.len() > 0 {
                // Check if operand is G6f (would be variable reference)
                if inst.operand_types[0] == infocom::instruction::OperandType::Variable 
                    && inst.operands[0] == 0x6f + 16 {
                    println!("\n[{:04}] GET_PARENT of G6f at PC {:05x}", count, pc);
                    if let Ok(g6f_val) = interpreter.vm.read_variable(16 + 0x6f) {
                        println!("       G6f currently: {}", g6f_val);
                    }
                }
            }
            
            // Catch entry to STAND
            if pc == 0x86ca {
                println!("\n=== ENTERING STAND ROUTINE ===");
                
                // Dump G6f and its parent info
                if let Ok(g6f_val) = interpreter.vm.read_variable(16 + 0x6f) {
                    println!("G6f value: {}", g6f_val);
                    
                    if g6f_val > 0 && g6f_val < 256 {
                        if let Ok(parent) = interpreter.vm.get_parent(g6f_val) {
                            println!("Parent of G6f: {}", parent);
                            
                            if parent > 0 {
                                // Check the critical attribute
                                if let Ok(has_attr) = interpreter.vm.test_attribute(parent as u16, 0x1b) {
                                    println!("Parent has attribute 0x1b: {}", has_attr);
                                    if !has_attr {
                                        println!("This will cause 'You are already standing' message!");
                                    }
                                }
                                
                                // Show all attributes of parent
                                println!("\nAll attributes of parent object {}:", parent);
                                for attr in 0..32 {
                                    if let Ok(has) = interpreter.vm.test_attribute(parent as u16, attr) {
                                        if has {
                                            println!("  Attribute {}: SET", attr);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                break;
            }
            
            // Execute instruction
            match interpreter.execute_instruction(&inst) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Error at PC {:05x}: {}", pc, e);
                    break;
                }
            }
        } else {
            eprintln!("Failed to decode instruction at PC {:05x}", pc);
            break;
        }
        
        if count > 5000 {
            println!("\nStopped after 5000 instructions");
            break;
        }
    }

    Ok(())
}