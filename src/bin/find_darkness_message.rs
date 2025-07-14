use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use infocom::disassembler::Disassembler;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the game file
    let mut file = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    file.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let interpreter = Interpreter::new(VM::new(game));
    let disasm = Disassembler::new(&interpreter.vm.game);
    
    println!("=== Finding the Darkness Message ===\n");
    
    // From our trace, we know the lighting check happens around 0x5f70
    // Let's look for what happens after the check fails
    
    // First, let's find the exact lighting check by looking for the pattern
    // that checks global variable 0x52 (LIT)
    
    println!("Looking for instructions that access global 0x52 (LIT variable)...");
    
    // In Z-machine, global variables are accessed with various opcodes:
    // - 0x01: je (jump if equal) can test globals
    // - 0x0F: loadw can load globals
    // - 0x15: load can load a global
    // - 0x2D: store can store to a global
    
    // Global 0x52 would be encoded as 0x52 in instructions
    
    for addr in 0x5f00..0x6000 {
        let opcode = interpreter.vm.game.memory[addr];
        
        // Look for je instructions (0x01)
        if opcode == 0x01 {
            // Check if the next byte is 0x52 (global variable)
            if addr + 1 < interpreter.vm.game.memory.len() && 
               interpreter.vm.game.memory[addr + 1] == 0x52 {
                println!("\nFound je instruction with g52 at 0x{:04x}", addr);
                match disasm.disassemble_range(addr as u32, (addr + 10) as u32) {
                    Ok(output) => {
                        println!("{}", output);
                        
                        // This is likely our lighting check!
                        // Let's see what happens when it fails
                        println!("\nDisassembling the failure path:");
                        match disasm.disassemble_range((addr + 10) as u32, (addr + 50) as u32) {
                            Ok(failure_output) => println!("{}", failure_output),
                            Err(_) => {}
                        }
                    }
                    Err(_) => {}
                }
            }
        }
    }
    
    // Also look for the pattern that was in our original trace
    println!("\n\nLooking for the exact pattern from trace (01 52 01 00 06)...");
    for addr in 0x5000..0x7000 {
        if addr + 4 < interpreter.vm.game.memory.len() &&
           interpreter.vm.game.memory[addr] == 0x01 &&
           interpreter.vm.game.memory[addr + 1] == 0x52 &&
           interpreter.vm.game.memory[addr + 2] == 0x01 &&
           interpreter.vm.game.memory[addr + 3] == 0x00 &&
           interpreter.vm.game.memory[addr + 4] == 0x06 {
            println!("Found exact pattern at 0x{:04x}!", addr);
            
            // Disassemble around this location
            match disasm.disassemble_range(addr as u32, (addr + 100) as u32) {
                Ok(output) => println!("{}", output),
                Err(_) => {}
            }
            
            // The failure case jumps +6 bytes from the end of the instruction
            let fail_addr = addr + 5 + 6;  // 5 byte instruction + 6 byte offset
            println!("\nWhen check fails, execution continues at 0x{:04x}:", fail_addr);
            match disasm.disassemble_range(fail_addr as u32, (fail_addr + 100) as u32) {
                Ok(output) => println!("{}", output),
                Err(_) => {}
            }
        }
    }
    
    // Let's also look for any call to routine 0x3770
    println!("\n\nLooking for calls to routine 0x3770...");
    
    // call_2n with address 0x3770 would be: 1A 37 70
    for addr in 0x5000..0x7000 {
        if addr + 2 < interpreter.vm.game.memory.len() &&
           interpreter.vm.game.memory[addr] == 0x1A &&
           interpreter.vm.game.memory[addr + 1] == 0x37 &&
           interpreter.vm.game.memory[addr + 2] == 0x70 {
            println!("Found call_2n 0x3770 at 0x{:04x}", addr);
            
            // Show context around this call
            if addr >= 20 {
                match disasm.disassemble_range((addr - 20) as u32, (addr + 20) as u32) {
                    Ok(output) => println!("{}", output),
                    Err(_) => {}
                }
            }
        }
    }

    Ok(())
}