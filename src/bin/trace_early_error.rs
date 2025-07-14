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
    
    println!("Tracing early execution to find write to 0x4f15...\n");
    
    let mut count = 0;
    
    loop {
        count += 1;
        let pc = vm.pc;
        
        println!("[{:03}] PC: 0x{:05x}", count, pc);
        
        // Decode instruction
        if let Ok(inst) = Instruction::decode(&vm.game.memory, pc as usize, vm.game.header.version) {
            let name = inst.name(vm.game.header.version);
            println!("     {}", inst.format_with_version(vm.game.header.version));
            
            // Check if this is a call instruction
            if name.starts_with("call") {
                println!("     *** This is a call instruction ***");
                if !inst.operands.is_empty() {
                    let packed = inst.operands[0];
                    let unpacked = packed * 2;
                    println!("     Calling routine at 0x{:05x}", unpacked);
                }
            }
            
            // Update PC
            vm.pc += inst.size as u32;
            
            // Execute with detailed error reporting
            let mut interpreter = Interpreter::new(vm);
            match interpreter.execute_instruction(&inst) {
                Ok(_) => vm = interpreter.vm,
                Err(e) => {
                    println!("\n*** ERROR OCCURRED ***");
                    println!("PC: 0x{:05x}", pc);
                    println!("Instruction: {}", inst.format_with_version(interpreter.vm.game.header.version));
                    println!("Error: {}", e);
                    
                    // Check if this relates to 0x4f15
                    if e.contains("4f15") {
                        println!("\n*** THIS IS THE 0x4f15 ERROR ***");
                        println!("The instruction that caused the error is above");
                        
                        // If it's a call, show details
                        if name.starts_with("call") {
                            println!("Call details:");
                            if !inst.operands.is_empty() {
                                let packed = inst.operands[0];
                                let unpacked = packed * 2;
                                println!("  Packed: 0x{:04x}", packed);
                                println!("  Unpacked: 0x{:05x}", unpacked);
                                println!("  Target + 1: 0x{:05x}", unpacked + 1);
                                
                                // Check the target routine header
                                let num_locals = interpreter.vm.game.memory[unpacked as usize];
                                println!("  Locals: {}", num_locals);
                                
                                // Show where the PC would read local values
                                let read_addr = unpacked + 1;
                                for i in 0..num_locals {
                                    let value_addr = read_addr + (i as u16 * 2);
                                    println!("  Local {} initial value at: 0x{:05x}", i, value_addr);
                                    if value_addr as u32 == 0x4f15 {
                                        println!("    *** THIS IS 0x4f15! ***");
                                    }
                                }
                            }
                        }
                    }
                    break;
                }
            }
        } else {
            println!("Failed to decode instruction at PC {:05x}", pc);
            break;
        }
        
        if count > 20 {
            println!("Stopped after 20 instructions");
            break;
        }
    }
    
    Ok(())
}