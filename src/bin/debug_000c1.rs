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
    let debugger = Debugger::new(vm);

    println!("Examining the area around 0x000c1 (header area!)...\n");
    
    // This is suspiciously low - in the header area!
    println!("Note: Address 0x000c1 is in the header area of the game file.");
    println!("This suggests V_b2 contained a very small value that was used as a routine address.\n");
    
    // Look at the raw bytes at this address
    let memory = &debugger.interpreter.vm.game.memory;
    println!("Raw bytes at 0x000bf-0x000c5:");
    for i in 0..7 {
        let addr = 0x000bf + i;
        if addr < memory.len() {
            print!("{:02x} ", memory[addr]);
        }
    }
    println!();
    
    // Try to decode instructions in this area
    println!("\nAttempting to decode as instructions:");
    for addr in 0x000bf..=0x000c3 {
        match infocom::instruction::Instruction::decode(memory, addr, 3) {
            Ok(inst) => {
                println!("{:05x}: {}", addr, inst.format_with_version(3));
                if let Some(ref branch) = inst.branch {
                    println!("       Branch info: on_true={}, offset={} (0x{:04x})", 
                            branch.on_true, branch.offset, branch.offset as u16);
                    
                    // Calculate where this would branch to
                    let pc_after_branch = addr as i32 + inst.size as i32;
                    let target = pc_after_branch + branch.offset as i32 - 2;
                    println!("       Would branch to: 0x{:08x}", target as u32);
                    
                    if branch.offset == -1415 {
                        println!("       *** THIS IS THE PROBLEMATIC BRANCH ***");
                        println!("       -1415 = 0x{:04x} (as signed i16)", branch.offset as u16);
                    }
                }
            }
            Err(e) => {
                println!("{:05x}: Error: {}", addr, e);
            }
        }
    }
    
    // Show what's actually at these addresses in the header
    println!("\nHeader interpretation of these bytes:");
    println!("0x00bf-0x00c0: Part of abbreviations table pointer");
    println!("0x00c1-0x00c2: Part of file length");
    
    // Show the actual routine that made the bad call
    println!("\nThe call_2s at 0x08cc4 called Vb2 with value #0000");
    println!("This suggests Vb2 contained a very small value (possibly 0x00bf)");
    println!("when it should have contained a valid routine address.");
    
    Ok(())
}