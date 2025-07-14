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

    println!("Tracing execution to find bad jump to 0xfffffb3d...\n");
    
    let mut count = 0;
    let mut last_few_instructions = Vec::new();
    
    loop {
        count += 1;
        let pc = debugger.interpreter.vm.pc;
        let call_depth = debugger.interpreter.vm.call_stack.len();
        
        // Get current instruction
        if let Ok(disasm) = debugger.disassemble_current() {
            // Keep track of last 10 instructions
            last_few_instructions.push((pc, disasm.clone(), call_depth));
            if last_few_instructions.len() > 10 {
                last_few_instructions.remove(0);
            }
            
            // Show calls, jumps, branches, and returns
            if disasm.contains("call") || disasm.contains("jump") || 
               disasm.contains("ret") || disasm.contains("branch") ||
               disasm.contains("[TRUE") || disasm.contains("[FALSE") {
                println!("[{:03}] Depth:{} PC:{:05x} - {}", count, call_depth, pc, disasm);
            }
            
            // Check if PC is suspiciously high
            if pc > 0x20000 {
                println!("\n*** SUSPICIOUS PC DETECTED ***");
                println!("PC jumped to: 0x{:08x}", pc);
                println!("\nLast 10 instructions before bad jump:");
                for (i, (addr, inst, depth)) in last_few_instructions.iter().enumerate() {
                    println!("  [{:02}] Depth:{} PC:{:05x} - {}", i, depth, addr, inst);
                }
                
                println!("\nCurrent call stack:");
                for (i, frame) in debugger.interpreter.vm.call_stack.iter().enumerate() {
                    println!("  Frame {}: return PC 0x{:05x}", i, frame.return_pc);
                }
                
                break;
            }
        }
        
        // Step one instruction and catch the error
        match debugger.step() {
            Ok(_) => {
                // Continue
            }
            Err(e) => {
                println!("\n*** ERROR DETECTED ***");
                println!("Error: {}", e);
                println!("PC when error occurred: 0x{:05x}", pc);
                
                // Check if this is the decode error
                if e.contains("Failed to decode") || e.contains("out of bounds") {
                    println!("\n*** THIS IS THE DECODE ERROR ***");
                    println!("\nLast 10 instructions before error:");
                    for (i, (addr, inst, depth)) in last_few_instructions.iter().enumerate() {
                        println!("  [{:02}] Depth:{} PC:{:05x} - {}", i, depth, addr, inst);
                    }
                    
                    println!("\nCurrent call stack:");
                    for (i, frame) in debugger.interpreter.vm.call_stack.iter().enumerate() {
                        println!("  Frame {}: return PC 0x{:05x}, locals: {}", 
                                i, frame.return_pc, frame.num_locals);
                        
                        // Show which routine this is
                        if i == debugger.interpreter.vm.call_stack.len() - 1 {
                            // This is the current routine - find its start
                            let mut routine_start = pc;
                            // Go back to find routine header (rough estimate)
                            while routine_start > frame.return_pc.saturating_sub(1000) && routine_start > 0x4f00 {
                                routine_start -= 1;
                                // Look for routine header pattern
                                if (routine_start as usize) < debugger.interpreter.vm.game.memory.len() {
                                    let byte = debugger.interpreter.vm.game.memory[routine_start as usize];
                                    // V3 routine header: num_locals byte
                                    if byte <= 15 && routine_start > 0x4f00 {
                                        // Check if this looks like a valid routine start
                                        if routine_start == frame.return_pc || 
                                           (routine_start > 0x4f00 && routine_start < 0x20000) {
                                            println!("    Current routine likely starts around: 0x{:05x}", routine_start);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                break;
            }
        }
        
        if count > 500 {
            println!("No error found in first 500 instructions");
            break;
        }
    }
    
    Ok(())
}