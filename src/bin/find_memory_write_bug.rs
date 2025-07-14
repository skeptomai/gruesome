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

    println!("Tracing execution to find instruction causing memory write error...\n");
    
    let mut count = 0;
    
    loop {
        count += 1;
        let pc = debugger.interpreter.vm.pc;
        
        // Show current instruction
        if let Ok(disasm) = debugger.disassemble_current() {
            println!("[{:03}] {}", count, disasm);
        }
        
        // Step one instruction and catch the error
        match debugger.step() {
            Ok(_) => {
                // Continue
            }
            Err(e) => {
                println!("\n*** ERROR FOUND ***");
                println!("Error: {}", e);
                println!("PC when error occurred: 0x{:05x}", pc);
                
                // Show the problematic instruction
                if let Ok(disasm) = debugger.disassemble_at(pc) {
                    println!("Problematic instruction: {}", disasm);
                }
                
                // Show some context around the instruction
                println!("\nContext (5 instructions before and after):");
                let start_pc = pc.saturating_sub(20); // Rough estimate
                let context = debugger.disassemble_range(start_pc, 10);
                for (i, line) in context.iter().enumerate() {
                    let marker = if line.starts_with(&format!("{:05x}", pc)) { " --> " } else { "     " };
                    println!("{}{}", marker, line);
                }
                
                // Show VM state
                println!("\nVM State when error occurred:");
                debugger.show_state();
                
                // Check if this is the 0x4f15 error
                if e.contains("4f15") {
                    println!("\n*** THIS IS THE 0x4f15 ERROR ***");
                    println!("The instruction above is being misinterpreted!");
                    println!("It should NOT be writing to memory address 0x4f15");
                    
                    // Try to analyze what the instruction should actually be
                    let memory = &debugger.interpreter.vm.game.memory;
                    println!("\nRaw bytes at PC 0x{:05x}:", pc);
                    for i in 0..8 {
                        if pc as usize + i < memory.len() {
                            print!("{:02x} ", memory[pc as usize + i]);
                        }
                    }
                    println!();
                    
                    println!("\nThis instruction is likely being decoded incorrectly.");
                    println!("Check the instruction decoding logic for this opcode.");
                }
                
                break;
            }
        }
        
        // Check for call instructions specifically 
        if let Ok(disasm) = debugger.disassemble_at(pc) {
            if disasm.contains("call") {
                println!(">>> CALL INSTRUCTION DETECTED: {}", disasm);
                
                // Check call stack before and after
                println!("Call stack depth before: {}", debugger.interpreter.vm.call_stack.len());
            }
        }
        
        if count > 200 {
            println!("No error found in first 200 instructions");
            break;
        }
    }
    
    Ok(())
}