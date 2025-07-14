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

    println!("Tracing execution to find stack underflow...\n");
    
    let mut count = 0;
    
    loop {
        count += 1;
        let pc = debugger.interpreter.vm.pc;
        let stack_len = debugger.interpreter.vm.stack.len();
        
        // Get current instruction
        if let Ok(disasm) = debugger.disassemble_current() {
            // Show instructions that manipulate the stack
            if disasm.contains("push") || disasm.contains("pull") || disasm.contains("call") || disasm.contains("ret") {
                println!("[{:03}] Stack:{:02} - {}", count, stack_len, disasm);
            }
            
            // Show unknown instructions
            if disasm.contains("unknown") {
                println!("[{:03}] UNKNOWN - {}", count, disasm);
            }
        }
        
        // Step one instruction and catch the error
        match debugger.step() {
            Ok(_) => {
                let new_stack_len = debugger.interpreter.vm.stack.len();
                
                // Show stack changes
                if new_stack_len != stack_len {
                    if let Ok(disasm) = debugger.disassemble_at(pc) {
                        println!("     Stack changed: {} -> {} after {}", stack_len, new_stack_len, disasm);
                    }
                }
            }
            Err(e) => {
                println!("\n*** STACK UNDERFLOW FOUND ***");
                println!("Error: {}", e);
                println!("PC when error occurred: 0x{:05x}", pc);
                println!("Stack length: {}", stack_len);
                
                // Show the problematic instruction
                if let Ok(disasm) = debugger.disassemble_at(pc) {
                    println!("Problematic instruction: {}", disasm);
                }
                
                // Show some context around the instruction
                println!("\nContext (5 instructions before and after):");
                let start_pc = pc.saturating_sub(25); // Rough estimate
                let context = debugger.disassemble_range(start_pc, 10);
                for line in context.iter() {
                    let marker = if line.starts_with(&format!("{:05x}", pc)) { " --> " } else { "     " };
                    println!("{}{}", marker, line);
                }
                
                // Show VM state
                println!("\nVM State when error occurred:");
                debugger.show_state();
                
                // Check if this is the stack underflow
                if e.contains("underflow") || e.contains("empty") {
                    println!("\n*** THIS IS THE STACK UNDERFLOW ***");
                    println!("The instruction above is trying to pop from an empty stack!");
                    
                    // Show call stack
                    println!("\nCall stack depth: {}", debugger.interpreter.vm.call_stack.len());
                    for (i, frame) in debugger.interpreter.vm.call_stack.iter().enumerate() {
                        println!("  Frame {}: return PC 0x{:05x}, locals: {}", 
                                i, frame.return_pc, frame.num_locals);
                    }
                }
                
                break;
            }
        }
        
        if count > 300 {
            println!("No error found in first 300 instructions");
            break;
        }
    }
    
    Ok(())
}