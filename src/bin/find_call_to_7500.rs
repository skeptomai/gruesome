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

    println!("Looking for calls to routine at 0x07500...\n");
    
    // Set a breakpoint when we're about to call to 0x07500
    // Since this would be packed address, let's calculate what the packed address would be
    // For V3: packed = unpacked / 2
    let packed_7500 = 0x07500 / 2;
    println!("Packed address for 0x07500 would be: 0x{:04x}", packed_7500);
    
    // Also set breakpoints on the DESCRIBE-ROOM routine
    debugger.add_breakpoint(0x8c9a); // DESCRIBE-ROOM
    debugger.add_breakpoint(0x8d32); // Near end of DESCRIBE-ROOM
    
    // Enable single-stepping once we get to DESCRIBE-ROOM
    let mut in_describe_room = false;
    let mut last_10_pcs = Vec::new();
    
    loop {
        let pc = debugger.interpreter.vm.pc;
        
        // Keep track of last 10 PCs
        last_10_pcs.push(pc);
        if last_10_pcs.len() > 10 {
            last_10_pcs.remove(0);
        }
        
        if pc == 0x8c9a {
            println!("Entered DESCRIBE-ROOM at 0x8c9a");
            in_describe_room = true;
        }
        
        if in_describe_room {
            // Check current instruction
            if let Ok(inst) = debugger.interpreter.vm.game.memory.get(pc as usize..)
                .ok_or("PC out of bounds")
                .and_then(|mem| infocom::instruction::Instruction::decode(mem, 0, 3)
                    .map_err(|e| e.as_str())) {
                
                // Check if this is a call instruction
                if inst.name(3).starts_with("call") {
                    if !inst.operands.is_empty() {
                        let target = inst.operands[0];
                        let unpacked = (target as u32) * 2; // V3 unpacking
                        
                        println!("  {:05x}: {} -> unpacked address 0x{:05x}", 
                                pc, inst.format_with_version(3), unpacked);
                        
                        if unpacked == 0x07500 {
                            println!("\n*** FOUND IT! ***");
                            println!("Call to 0x07500 from PC 0x{:05x}", pc);
                            println!("Instruction: {}", inst.format_with_version(3));
                            println!("\nLast 10 PCs before this:");
                            for &prev_pc in &last_10_pcs {
                                println!("  0x{:05x}", prev_pc);
                            }
                            break;
                        }
                    }
                }
            }
        }
        
        // Step to next instruction
        match debugger.step() {
            Ok(_) => {},
            Err(e) => {
                if e.contains("requires 2 operands") && pc == 0x07554 {
                    println!("\nHit the MOD error at 0x07554");
                    println!("Last 10 PCs before error:");
                    for &prev_pc in &last_10_pcs {
                        println!("  0x{:05x}", prev_pc);
                    }
                    
                    // Check if we somehow jumped into the middle of data
                    println!("\nChecking if 0x07500-0x07560 might be data, not code...");
                    break;
                } else {
                    println!("Error: {}", e);
                    break;
                }
            }
        }
        
        if pc > 0x10000 {
            println!("PC went too high, stopping");
            break;
        }
    }
    
    Ok(())
}