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
    
    println!("Tracing branch execution at 0x08cc7...\n");
    
    let mut count = 0;
    let mut watch_branch = false;
    
    loop {
        count += 1;
        let pc = vm.pc;
        
        // Start watching when we get close to the branch
        if pc >= 0x08cc0 && pc <= 0x08cd0 {
            watch_branch = true;
        }
        
        if watch_branch {
            // Decode instruction
            if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
                println!("\n[{:04}] PC = 0x{:05x}", count, pc);
                println!("  Instruction: {}", inst.format_with_version(vm.game.header.version));
                
                // If this is the critical branch
                if pc == 0x08cc7 {
                    println!("\n*** CRITICAL BRANCH ***");
                    
                    // Check the operands
                    if inst.operands.len() >= 2 {
                        let op1 = inst.operands[0]; // Should be 5
                        let op2_var = inst.operands[1]; // Should be Vd1
                        
                        // Read Vd1
                        let vd1_value = vm.read_variable(op2_var as u8).unwrap_or(0xFFFF);
                        
                        println!("  Comparing: {} < {} (V{:02x})", op1, vd1_value, op2_var);
                        println!("  Condition: {} < {} = {}", op1, vd1_value, op1 < vd1_value);
                        
                        if let Some(branch) = &inst.branch {
                            let should_branch = (op1 < vd1_value) == branch.on_true;
                            println!("  Branch on: {}", if branch.on_true { "TRUE" } else { "FALSE" });
                            println!("  Should branch: {}", should_branch);
                            
                            if should_branch {
                                let new_pc = (pc as i32 + inst.size as i32 + branch.offset as i32 - 2) as u32;
                                println!("  Will jump to: 0x{:05x}", new_pc);
                            } else {
                                println!("  Will continue to next instruction");
                            }
                        }
                    }
                }
                
                // Update PC first
                vm.pc += inst.size as u32;
                
                // Execute instruction
                let mut interpreter = Interpreter::new(vm);
                match interpreter.execute_instruction(&inst) {
                    Ok(_) => {
                        vm = interpreter.vm;
                        
                        // Check if PC changed (branch taken)
                        if vm.pc != pc + inst.size as u32 {
                            println!("  -> PC changed to 0x{:05x}", vm.pc);
                        }
                    },
                    Err(e) => {
                        eprintln!("Error at PC {:05x}: {}", pc, e);
                        break;
                    }
                }
                
                // Check what's at the new PC
                if pc == 0x08cc7 {
                    println!("\nAfter branch, PC = 0x{:05x}", vm.pc);
                    
                    // Decode next instruction
                    if let Ok(next_inst) = Instruction::decode(&vm.game.memory, vm.pc as usize, vm.game.header.version) {
                        println!("Next instruction: {}", next_inst.format_with_version(vm.game.header.version));
                    }
                }
                
                // Stop after we've seen what happens
                if pc > 0x08cd0 {
                    break;
                }
            }
        }
        
        // Execute normally when not watching
        if !watch_branch {
            if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
                vm.pc += inst.size as u32;
                let mut interpreter = Interpreter::new(vm);
                match interpreter.execute_instruction(&inst) {
                    Ok(_) => vm = interpreter.vm,
                    Err(e) => {
                        eprintln!("Error at PC {:05x}: {}", pc, e);
                        break;
                    }
                }
            }
        }
        
        if count > 10000 {
            println!("\nStopped after 10000 instructions");
            break;
        }
    }

    Ok(())
}