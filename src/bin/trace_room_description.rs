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
    
    println!("Tracing execution to find DESCRIBE-ROOM and DESCRIBE-OBJECTS...\n");
    
    // Common routine addresses we might see (you mentioned these)
    let describe_room = 0x6e81;  // Guessing from typical Infocom patterns
    let describe_objects = 0x6f2a; // Another guess
    
    let mut count = 0;
    let mut after_serial = false;
    let mut last_jump_addr = 0u32;
    let mut last_call_addr = 0u32;
    
    loop {
        count += 1;
        let pc = vm.pc;
        
        // Track serial number
        if pc == 0x06f8c {
            after_serial = true;
            println!("\n*** SERIAL NUMBER at instruction {} ***", count);
        }
        
        // Check for STAND routine
        if pc == 0x86ca {
            println!("\n!!! ENTERED STAND ROUTINE at instruction {} !!!", count);
            println!("Last call was to: 0x{:05x}", last_call_addr);
            println!("Last jump was to: 0x{:05x}", last_jump_addr);
            
            // Show how we got here
            println!("\nCall stack depth: {}", vm.call_stack.len());
            for (i, frame) in vm.call_stack.iter().enumerate() {
                println!("  [{}] Return to {:05x}", i, frame.return_pc);
            }
            break;
        }
        
        // Decode instruction
        if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
            let name = inst.name(vm.game.header.version);
            
            // Track important routines after serial
            if after_serial {
                // Check for DESCRIBE-ROOM pattern
                if pc == describe_room {
                    println!("\n*** ENTERED DESCRIBE-ROOM at instruction {} ***", count);
                }
                
                if pc == describe_objects {
                    println!("\n*** ENTERED DESCRIBE-OBJECTS at instruction {} ***", count);
                }
                
                // Track all calls and jumps to understand flow
                if name.starts_with("call") {
                    if !inst.operands.is_empty() {
                        let packed = inst.operands[0];
                        let unpacked = packed * 2; // V3
                        if unpacked > 0x4000 {
                            println!("[{:04}] {:05x}: {} to 0x{:05x}", count, pc, name, unpacked);
                            last_call_addr = unpacked as u32;
                        }
                    }
                }
                
                if name == "jump" {
                    if !inst.operands.is_empty() {
                        // Jump uses offset from current position
                        let offset = inst.operands[0] as i16;
                        let target = ((pc + inst.size as u32) as i32 + offset as i32 - 2) as u32;
                        println!("[{:04}] {:05x}: jump to 0x{:05x} (offset {})", count, pc, target, offset);
                        last_jump_addr = target;
                    }
                }
                
                // Track indirect jumps (ret with value)
                if name == "ret" && !inst.operands.is_empty() {
                    println!("[{:04}] {:05x}: ret {}", count, pc, inst.operands[0]);
                }
                
                // Track any PC modifications through print_paddr
                if name == "print_paddr" {
                    println!("[{:04}] {:05x}: print_paddr (might contain jump table?)", count, pc);
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
        
        if count > 5000 {
            println!("\nStopped after 5000 instructions");
            break;
        }
    }

    Ok(())
}