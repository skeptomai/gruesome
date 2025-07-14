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
    
    println!("=== Tracing Lighting Check Failure ===\n");
    
    // From the original trace, we have:
    // 05f70: 01 52 01 00 06    je g52, #0001 [FALSE +6]
    // This means if g52 != 1, PC jumps to 0x5f70 + 5 (instruction size) + 6 = 0x5f7b
    
    println!("The lighting check instruction:");
    match disasm.disassemble_range(0x5f70, 0x5f75) {
        Ok(output) => println!("{}", output),
        Err(e) => println!("Error: {}", e),
    }
    
    // Actually, looking at the disassembly output, the instruction at 0x5f70 is showing as:
    // 05f70: 46 7a 40 e0              jin V7a, #0040 [TRUE -32]
    // This doesn't match what we expected. Let me look at the exact bytes
    
    let bytes_at_5f70 = &interpreter.vm.game.memory[0x5f70..0x5f76];
    println!("\nRaw bytes at 0x5f70: {:02x?}", bytes_at_5f70);
    
    // The original trace showed: 01 52 01 00 06
    // But the disassembler shows: 46 7a 40 e0
    // This suggests we might be looking at the wrong address or there's been a misunderstanding
    
    // Let me search for the actual je g52, #0001 instruction
    println!("\nSearching for je g52, #0001 pattern (01 52 01)...");
    
    for addr in 0x5f00..0x6000 {
        if interpreter.vm.game.memory[addr] == 0x01 &&
           interpreter.vm.game.memory[addr + 1] == 0x52 &&
           interpreter.vm.game.memory[addr + 2] == 0x01 {
            println!("Found at 0x{:04x}", addr);
            match disasm.disassemble_range(addr as u32, (addr + 10) as u32) {
                Ok(output) => println!("{}", output),
                Err(_) => {}
            }
        }
    }
    
    // Let's also look for where the "Only bats can see in the dark" message might be printed
    // This would likely be a call to a print routine
    println!("\n\nLooking for the routine that prints the darkness message...");
    
    // From earlier analysis, we know that when lighting fails, something prints
    // "Only bats can see in the dark. In fact, only"
    // Let's look for calls in the area after the lighting check
    
    println!("\nDisassembling from 0x5f76 (after the lighting check):");
    match disasm.disassemble_range(0x5f76, 0x5fa0) {
        Ok(output) => println!("{}", output),
        Err(e) => println!("Error: {}", e),
    }
    
    // Look for specific call instructions
    println!("\n\nLooking for call instructions that might print the message:");
    for addr in 0x5f70..0x5fc0 {
        let opcode = interpreter.vm.game.memory[addr];
        
        // call_2n opcode is 0x1A
        if opcode == 0x1A {
            println!("\nFound call_2n at 0x{:04x}", addr);
            match disasm.disassemble_instruction(addr as u32) {
                Ok((instr, output)) => {
                    println!("{}", output);
                    
                    // Try to determine what routine is being called
                    if instr.operands.len() >= 1 {
                        println!("  Calling routine at operand: {:?}", instr.operands[0]);
                    }
                }
                Err(_) => {}
            }
        }
        
        // call (long form) opcodes: 0xE0-0xE7, 0x88, 0x8F
        if (opcode >= 0xE0 && opcode <= 0xE7) || opcode == 0x88 || opcode == 0x8F {
            match disasm.disassemble_instruction(addr as u32) {
                Ok((instr, output)) => {
                    if output.contains("call") {
                        println!("\nFound call at 0x{:04x}: {}", addr, output);
                        
                        // Check if this might be calling routine 0x3770
                        for op in &instr.operands {
                            if *op == 0x3770 {
                                println!("  >>> This calls routine 0x3770!");
                            }
                        }
                    }
                }
                Err(_) => {}
            }
        }
    }

    Ok(())
}