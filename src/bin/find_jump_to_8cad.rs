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

    println!("Tracing to find what jumps to 0x08cad...\n");
    
    let mut count = 0;
    let mut last_pc = 0;
    let mut last_instruction = String::new();
    
    loop {
        count += 1;
        let pc = debugger.interpreter.vm.pc;
        
        // Check if we just jumped to 0x8cad
        if pc == 0x08cad {
            println!("\n*** FOUND JUMP TO 0x08cad ***");
            println!("Previous PC: 0x{:05x}", last_pc);
            println!("Previous instruction: {}", last_instruction);
            println!("This is in the middle of PUSH G47 at 0x8cac!");
            
            // Check what could have caused this
            if last_instruction.contains("TRUE") || last_instruction.contains("FALSE") {
                println!("Likely a branch instruction branched here");
            } else if last_instruction.contains("jump") {
                println!("A jump instruction jumped here");
            } else if last_instruction.contains("call") {
                println!("Possibly returning from a call?");
            }
            
            break;
        }
        
        // Get current instruction before executing
        if let Ok(disasm) = debugger.disassemble_current() {
            // Track branches and jumps that might go to 0x8cad
            if disasm.contains("8cad") || 
               (disasm.contains("TRUE") && disasm.contains("+1")) ||
               (disasm.contains("FALSE") && disasm.contains("+1")) {
                println!("[{:03}] PC:{:05x} - {} *** MIGHT BRANCH TO 0x8cad ***", 
                        count, pc, disasm);
            }
            
            last_pc = pc;
            last_instruction = disasm;
        }
        
        // Step one instruction
        match debugger.step() {
            Ok(_) => {},
            Err(e) => {
                println!("\nError at PC {:05x}: {}", pc, e);
                if pc == 0x08cad {
                    println!("Error occurred at 0x08cad - the bad address!");
                }
                break;
            }
        }
        
        if count > 300 {
            println!("Stopped after 300 instructions without finding jump to 0x8cad");
            break;
        }
    }
    
    Ok(())
}