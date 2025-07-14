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

    println!("Looking for what causes jump to 0x07554...\n");
    
    // Keep a history of the last N PCs
    let mut pc_history: Vec<u32> = Vec::new();
    let history_size = 50;
    
    loop {
        let pc = debugger.interpreter.vm.pc;
        
        // Add to history
        pc_history.push(pc);
        if pc_history.len() > history_size {
            pc_history.remove(0);
        }
        
        // Check if we're at or near the bad address
        if pc == 0x07554 || pc == 0x07553 {
            println!("Reached PC 0x{:05x}!", pc);
            
            println!("\nLast {} PCs:", pc_history.len());
            for (i, &hist_pc) in pc_history.iter().enumerate() {
                println!("  [{}] 0x{:05x}", i, hist_pc);
                
                // Check for big jumps
                if i > 0 {
                    let prev_pc = pc_history[i-1];
                    let diff = (hist_pc as i32) - (prev_pc as i32);
                    if diff.abs() > 100 {
                        println!("      ^^^ BIG JUMP: {:+} bytes", diff);
                        
                        // What instruction was at prev_pc?
                        if let Ok(inst) = infocom::instruction::Instruction::decode(
                            &debugger.interpreter.vm.game.memory, prev_pc as usize, 3) {
                            println!("      From: {}", inst.format_with_version(3));
                            
                            if inst.branch.is_some() {
                                let branch = inst.branch.as_ref().unwrap();
                                println!("      Branch offset: {:+}", branch.offset);
                            }
                        }
                    }
                }
            }
            
            break;
        }
        
        // Single step
        match debugger.step() {
            Ok(_) => {},
            Err(e) => {
                println!("Error at PC 0x{:05x}: {}", pc, e);
                
                if e.contains("requires 2 operands") {
                    println!("\nThis is our error!");
                    println!("Last {} PCs before error:", pc_history.len());
                    for (i, &hist_pc) in pc_history.iter().enumerate() {
                        println!("  [{}] 0x{:05x}", i, hist_pc);
                    }
                }
                
                break;
            }
        }
    }
    
    Ok(())
}