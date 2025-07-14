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
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    let disasm = Disassembler::new(&interpreter.vm.game);
    
    println!("=== Understanding the Darkness Message ===\n");
    
    // From our execution trace, we saw:
    // 05f70: 01 52 01 00 06    je g52, #0001 [FALSE +6]
    // 05f75: 1a 37 70          call_2n #3770
    
    // This means:
    // - If g52 (LIT) == 1, the test succeeds and we call routine 0x3770
    // - If g52 (LIT) != 1, the test fails and we skip to 0x5f70 + 5 + 6 = 0x5f7b
    
    // So the actual pattern is INVERTED from what I thought!
    // When LIT == 1 (room is lit), we call 0x3770
    // When LIT != 1 (room is dark), we skip the call
    
    println!("Re-examining the lighting check logic:");
    println!("- If LIT == 1 (room is lit): call routine 0x3770");
    println!("- If LIT != 1 (room is dark): skip to failure path\n");
    
    // Let's trace what happens when we're in darkness (LIT != 1)
    let dark_path_start = 0x5f70 + 5 + 6; // Skip the je and call_2n instructions
    
    println!("When in darkness, execution continues at 0x{:04x}:", dark_path_start);
    
    // Disassemble the darkness handling path
    match disasm.disassemble_range(dark_path_start as u32, (dark_path_start + 200) as u32) {
        Ok(output) => println!("{}", output),
        Err(e) => println!("Error: {}", e),
    }
    
    // Look for print instructions in the darkness path
    println!("\n\nSearching for print instructions in the darkness handling:");
    
    for offset in 0..500 {
        let addr = dark_path_start + offset;
        if addr >= interpreter.vm.game.memory.len() {
            break;
        }
        
        let opcode = interpreter.vm.game.memory[addr];
        
        // Check for print_paddr
        if opcode == 0xAD || opcode == 0xB2 {
            println!("\nFound print_paddr at 0x{:04x}:", addr);
            match disasm.disassemble_instruction(addr as u32) {
                Ok((instr, output)) => {
                    println!("{}", output);
                    
                    // If it's print_paddr with an address, show what's at that address
                    if opcode == 0xAD && instr.operands.len() > 0 {
                        let packed_addr = instr.operands[0];
                        let string_addr = packed_addr as u32 * 2; // Unpack for V3
                        println!("  String is at packed address 0x{:04x} (unpacked: 0x{:04x})", 
                                 packed_addr, string_addr);
                    }
                }
                Err(_) => {}
            }
        }
        
        // Check for regular print
        if opcode == 0x02 || opcode == 0x03 {
            println!("\nFound print at 0x{:04x}:", addr);
            match disasm.disassemble_instruction(addr as u32) {
                Ok((_, output)) => println!("{}", output),
                Err(_) => {}
            }
        }
        
        // Check for calls that might print
        if opcode == 0xE0 || opcode == 0x88 || opcode == 0x8F {
            match disasm.disassemble_instruction(addr as u32) {
                Ok((_, output)) => {
                    if output.contains("call") {
                        println!("\nFound call at 0x{:04x}: {}", addr, output);
                    }
                }
                Err(_) => {}
            }
        }
    }
    
    // Let's also check what happens in the game's print routine
    // Common addresses for print routines in Zork are around 0x2000-0x3000
    
    println!("\n\n=== Checking common print routine addresses ===");
    
    // Look for the actual "Only bats" string by examining print calls
    for addr in [0x2f00, 0x2fd8, 0x2fed, 0x3000, 0x31cf].iter() {
        println!("\nChecking routine at 0x{:04x}:", addr);
        match disasm.disassemble_range(*addr, addr + 20) {
            Ok(output) => println!("{}", output),
            Err(_) => {}
        }
    }

    Ok(())
}