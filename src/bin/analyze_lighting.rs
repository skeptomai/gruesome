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
    
    println!("=== Analyzing Lighting Check ===\n");
    
    // From our execution trace, we know that when the lighting check fails,
    // the game prints "Only bats can see in the dark"
    
    // Let's look at the calling sequence more carefully
    // The trace showed that call_2n 0x3770 was called
    
    // First, let's examine what call_2n 0x3770 would look like in memory
    let call_addr = 0x3770;
    let packed_addr = call_addr / 2;  // For Z3, routine addresses are packed
    
    println!("Looking for call_2n to routine 0x{:04x} (packed: 0x{:04x})", call_addr, packed_addr);
    
    // call_2n instruction format: 0x1A followed by the address
    // The address would be encoded as two bytes
    let high_byte = (packed_addr >> 8) as u8;
    let low_byte = (packed_addr & 0xFF) as u8;
    
    println!("Searching for pattern: 1A {:02X} {:02X}", high_byte, low_byte);
    
    for addr in 0x5000..0x7000 {
        if addr + 2 < interpreter.vm.game.memory.len() &&
           interpreter.vm.game.memory[addr] == 0x1A &&
           interpreter.vm.game.memory[addr + 1] == high_byte &&
           interpreter.vm.game.memory[addr + 2] == low_byte {
            println!("\nFound call_2n to 0x{:04x} at address 0x{:04x}", call_addr, addr);
            
            // Show context
            if addr >= 50 {
                println!("\nContext before the call:");
                match disasm.disassemble_range((addr - 50) as u32, (addr + 10) as u32) {
                    Ok(output) => println!("{}", output),
                    Err(_) => {}
                }
            }
        }
    }
    
    // Now let's examine the routine at 0x3770 itself
    println!("\n\n=== Examining routine at 0x3770 ===");
    
    // In Z-machine, routines start with a byte indicating the number of local variables
    let locals_count = interpreter.vm.game.memory[0x3770];
    println!("Number of locals: {}", locals_count);
    
    // The actual code starts after the locals byte
    let code_start = 0x3770 + 1;
    
    println!("\nDisassembling routine starting at 0x{:04x}:", code_start);
    match disasm.disassemble_range(code_start as u32, (code_start + 100) as u32) {
        Ok(output) => println!("{}", output),
        Err(e) => println!("Error: {}", e),
    }
    
    // Let's also look for print instructions in this routine
    println!("\n\nLooking for print instructions in the routine:");
    
    for offset in 0..200 {
        let addr = code_start + offset;
        if addr >= interpreter.vm.game.memory.len() {
            break;
        }
        
        let opcode = interpreter.vm.game.memory[addr];
        
        // Check for various print opcodes
        if opcode == 0x02 || opcode == 0x03 {  // print, print_ret
            println!("\nFound print at offset {}: 0x{:04x}", offset, addr);
            match disasm.disassemble_instruction(addr as u32) {
                Ok((_, output)) => println!("{}", output),
                Err(_) => {}
            }
            
            // Try to decode the string
            let string_start = addr + 1;
            println!("String bytes: {:02X?}", &interpreter.vm.game.memory[string_start..string_start.min(string_start + 10)]);
        } else if opcode == 0xB2 {  // print (as 0OP)
            println!("\nFound print (0OP) at offset {}: 0x{:04x}", offset, addr);
            match disasm.disassemble_instruction(addr as u32) {
                Ok((_, output)) => println!("{}", output),
                Err(_) => {}
            }
        }
    }
    
    // Let's also check what global 0x52 (LIT) contains
    println!("\n\n=== Checking LIT variable ===");
    match interpreter.vm.read_global(0x52) {
        Ok(val) => println!("Global 0x52 (LIT) = {} (0x{:04x})", val, val),
        Err(e) => println!("Error reading LIT: {}", e),
    }

    Ok(())
}