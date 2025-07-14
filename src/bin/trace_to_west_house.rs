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

    println!("Tracing execution to find why we don't reach WEST-HOUSE at 0x953c...\n");
    
    // Set breakpoints at key locations
    debugger.add_breakpoint(0x8d32); // Near end of DESCRIBE-ROOM
    debugger.add_breakpoint(0x953c); // WEST-HOUSE routine
    debugger.add_breakpoint(0x7554); // Where the error occurs
    
    println!("Running until we hit a breakpoint...");
    
    match debugger.run() {
        Ok(_) => println!("Completed successfully"),
        Err(e) => {
            println!("Error: {}", e);
            
            // Show where we are
            println!("\nCurrent PC: 0x{:05x}", debugger.interpreter.vm.pc);
            
            // Show the instruction that failed
            if let Ok(disasm) = debugger.disassemble_current() {
                println!("Failed instruction: {}", disasm);
            }
            
            // Show some context
            println!("\nContext:");
            let pc = debugger.interpreter.vm.pc;
            let context = debugger.disassemble_range(pc.saturating_sub(20), 10);
            for line in context {
                let marker = if line.starts_with(&format!("{:05x}", pc)) { " --> " } else { "     " };
                println!("{}{}", marker, line);
            }
            
            // Check if this is the Variable 2OP error
            if e.contains("Variable 2OP") && e.contains("opcode: 18") {
                println!("\nThis is the MOD instruction error.");
                println!("Variable 2OP form of MOD needs 2 operands but only got 1.");
                
                // Decode the instruction manually
                let memory = &debugger.interpreter.vm.game.memory;
                if (pc as usize) < memory.len() {
                    println!("\nRaw bytes at 0x{:05x}:", pc);
                    for i in 0..8 {
                        if pc as usize + i < memory.len() {
                            print!("{:02x} ", memory[pc as usize + i]);
                        }
                    }
                    println!();
                    
                    // Check what form this instruction is
                    let opcode_byte = memory[pc as usize];
                    println!("\nOpcode byte: 0x{:02x}", opcode_byte);
                    if opcode_byte >= 0xC0 {
                        println!("This is Variable form (top 2 bits = 11)");
                        println!("Bottom 5 bits: 0x{:02x} = opcode", opcode_byte & 0x1F);
                        
                        if (opcode_byte & 0x1F) == 0x18 {
                            println!("Opcode 0x18 = MOD");
                            
                            // Check operand types
                            if pc as usize + 1 < memory.len() {
                                let types_byte = memory[pc as usize + 1];
                                println!("\nOperand types byte: 0x{:02x}", types_byte);
                                println!("  Op1: {:02b} = {}", (types_byte >> 6) & 3, 
                                        match (types_byte >> 6) & 3 {
                                            0 => "Large", 1 => "Small", 2 => "Variable", 3 => "Omitted", _ => "?"
                                        });
                                println!("  Op2: {:02b} = {}", (types_byte >> 4) & 3,
                                        match (types_byte >> 4) & 3 {
                                            0 => "Large", 1 => "Small", 2 => "Variable", 3 => "Omitted", _ => "?"
                                        });
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}