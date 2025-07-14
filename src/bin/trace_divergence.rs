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
    
    println!("Tracing to find where we diverge to STAND routine...\n");
    
    let mut count = 0;
    let mut after_serial = false;
    let mut trace_branches = false;
    
    loop {
        count += 1;
        let pc = vm.pc;
        
        // Start detailed tracing after serial
        if pc == 0x06f8c {
            after_serial = true;
            trace_branches = true;
            println!("\n*** SERIAL NUMBER PRINTED ***");
            println!("Now tracing all branches and calls...\n");
        }
        
        // Check for STAND routine
        if pc == 0x86ca {
            println!("\n!!! REACHED STAND ROUTINE !!!");
            println!("This should NOT happen - we want room description!");
            
            // Dump recent execution
            println!("\nHow did we get here?");
            break;
        }
        
        // Check for other important routines we might expect
        if after_serial && count < 300 {
            match pc {
                0x6e81 => println!("*** Hit routine at 0x6e81 (DESCRIBE-ROOM?)"),
                0x6f2a => println!("*** Hit routine at 0x6f2a (DESCRIBE-OBJECTS?)"),
                0x577c => println!("*** Hit routine at 0x577c (V-LOOK?)"),
                _ => {}
            }
        }
        
        // Decode instruction
        if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
            let name = inst.name(vm.game.header.version);
            
            // Trace key instructions after serial
            if trace_branches && count < 300 {
                // Always show branches
                if inst.branch.is_some() {
                    print!("[{:04}] {:05x}: {} ", count, pc, 
                        inst.format_with_version(vm.game.header.version));
                    
                    // Show branch decision
                    if let Some(branch) = &inst.branch {
                        let taken = match name {
                            "je" | "jl" | "jg" | "test_attr" | "jin" | "test" => {
                                // For these, we'd need to evaluate the condition
                                // For now, just show the branch info
                                print!("Branch: {} to ", if branch.on_true { "TRUE" } else { "FALSE" });
                                
                                match branch.offset {
                                    0 => print!("RFALSE"),
                                    1 => print!("RTRUE"),
                                    offset => {
                                        let target = ((pc + inst.size as u32) as i32 + offset as i32 - 2) as u32;
                                        print!("0x{:05x}", target);
                                    }
                                }
                            }
                            _ => {}
                        };
                    }
                    println!();
                }
                
                // Show calls
                if name.starts_with("call") && !inst.operands.is_empty() {
                    let packed = inst.operands[0];
                    let unpacked = packed * 2;
                    println!("[{:04}] {:05x}: {} to routine at 0x{:05x}", 
                             count, pc, name, unpacked);
                }
                
                // Show returns
                if name == "ret" || name == "rtrue" || name == "rfalse" {
                    println!("[{:04}] {:05x}: {}", count, pc, name);
                }
            }
            
            // Update PC
            vm.pc += inst.size as u32;
            
            // Execute
            let mut interpreter = Interpreter::new(vm);
            match interpreter.execute_instruction(&inst) {
                Ok(_) => {
                    vm = interpreter.vm;
                },
                Err(e) => {
                    eprintln!("Error at PC {:05x}: {}", pc, e);
                    break;
                }
            }
        } else {
            eprintln!("Failed to decode instruction at PC {:05x}", pc);
            break;
        }
        
        if count > 300 && after_serial {
            println!("\nStopped tracing after 300 instructions post-serial");
            break;
        }
        
        if count > 5000 {
            println!("\nStopped after 5000 total instructions");
            break;
        }
    }

    Ok(())
}