use infocom::debugger::Debugger;
use infocom::vm::{Game, VM};
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data.clone())?;
    let vm = VM::new(game);
    let mut debugger = Debugger::new(vm);

    println!("Looking for the path from DESCRIBE-ROOM to the error...\n");
    
    // The error happens in a routine that starts around 0x08d70
    // Let's find calls to that routine
    let target_packed = 0x08d70 / 2; // = 0x46b8
    println!("Looking for calls to 0x{:04x} (unpacks to 0x{:05x})", target_packed, 0x08d70);
    
    // Search for CALL instructions to this address
    for addr in (0x8c9a..0x8d40).step_by(1) {
        if addr + 3 < game_data.len() {
            // Check for Variable form CALL with this address
            let byte0 = game_data[addr];
            if byte0 == 0xe0 { // Variable form, opcode 0 (call)
                let byte1 = game_data[addr + 1];
                // Check if first operand is large constant
                if (byte1 >> 6) == 0 { // Large constant
                    let operand = ((game_data[addr + 2] as u16) << 8) | (game_data[addr + 3] as u16);
                    if operand == target_packed {
                        println!("\nFound CALL to 0x{:04x} at 0x{:05x}!", operand, addr);
                        
                        // Decode the full instruction
                        if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, addr, 3) {
                            println!("  Instruction: {}", inst.format_with_version(3));
                        }
                    }
                }
            }
        }
    }
    
    // Also check what routine 0x08d70 thinks it is
    println!("\n\nChecking what's at 0x08d70:");
    let addr = 0x08d70;
    
    // Look backwards to find a potential routine start
    println!("\nLooking backwards from 0x08d70 for a valid routine header:");
    
    for back in 1..50 {
        let check_addr = addr - back;
        if check_addr < game_data.len() {
            let num_locals = game_data[check_addr];
            if num_locals <= 15 {
                // This could be a routine start
                println!("\n  Potential routine at 0x{:05x} with {} locals", check_addr, num_locals);
                
                // Verify by checking if local initial values make sense
                let mut test_pc = check_addr + 1;
                let mut looks_valid = true;
                
                for _ in 0..num_locals {
                    if test_pc + 1 >= game_data.len() {
                        looks_valid = false;
                        break;
                    }
                    test_pc += 2;
                }
                
                if looks_valid && test_pc < game_data.len() {
                    // Try to decode first instruction
                    if let Ok(inst) = infocom::instruction::Instruction::decode(&game_data, test_pc, 3) {
                        println!("    First instruction at 0x{:05x}: {}", test_pc, inst.format_with_version(3));
                        
                        // If this looks reasonable, this might be the real routine start
                        if !inst.name(3).contains("unknown") {
                            println!("    This looks like a valid routine!");
                            
                            // Calculate what packed address would point here
                            let packed = check_addr / 2;
                            println!("    Packed address would be: 0x{:04x}", packed);
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}