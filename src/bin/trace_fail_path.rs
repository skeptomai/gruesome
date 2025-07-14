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
    
    println!("=== Tracing Lighting Check Failure Path ===\n");
    
    // The lighting check at 0x5f70: je g52, #0001 [FALSE +6]
    // If g52 != 1, it jumps to 0x5f7b
    
    println!("When lighting check fails (g52 != 1), execution jumps to 0x5f7b");
    println!("\nDisassembling the failure path starting at 0x5f7b:");
    
    // Let's trace through the failure path more carefully
    let mut pc = 0x5f7b;
    let mut steps = 0;
    
    while steps < 20 && pc < 0x6000 {
        println!("\nStep {}: PC = 0x{:04x}", steps, pc);
        
        match disasm.disassemble_instruction(pc) {
            Ok((instr, output)) => {
                println!("{}", output);
                
                // Check if this is a print instruction
                let opcode = interpreter.vm.game.memory[pc as usize];
                if opcode == 0x02 || opcode == 0x03 || opcode == 0xb2 {
                    println!("  >>> This is a PRINT instruction!");
                    
                    // If it's a literal print, try to decode the string
                    if opcode == 0x02 || opcode == 0x03 {
                        let string_start = pc + 1;
                        println!("  >>> String data starts at 0x{:04x}", string_start);
                        
                        // Show raw bytes
                        let raw_bytes = &interpreter.vm.game.memory[string_start as usize..(string_start + 20) as usize];
                        println!("  >>> Raw bytes: {:02x?}", raw_bytes);
                    }
                } else if opcode == 0x8C {
                    println!("  >>> This is a JUMP instruction!");
                }
                
                // Update PC based on instruction
                pc += instr.size as u32;
                
                // If it's a jump, follow it
                if opcode == 0x8C {
                    // Jump instruction format: 8C offset (offset is from after the instruction)
                    // The offset bytes are at pc+1 and pc+2 (we've already moved pc past the opcode)
                    let offset_byte1 = interpreter.vm.game.memory[(pc + 1) as usize];
                    let offset_byte2 = interpreter.vm.game.memory[(pc + 2) as usize];
                    let offset = ((offset_byte1 as u16) << 8) | (offset_byte2 as u16);
                    let signed_offset = if offset >= 0x8000 {
                        (offset as i32) - 0x10000
                    } else {
                        offset as i32
                    };
                    // Jump offset is from the address after the full instruction
                    let new_pc = ((pc + 3) as i32 + signed_offset - 2) as u32;
                    println!("  >>> Jump offset: {} (0x{:04x}), jumping to 0x{:04x}", 
                             signed_offset, offset, new_pc);
                    pc = new_pc;
                    continue;
                }
                
                // Check for return instructions
                if opcode == 0xB0 || opcode == 0xB1 || (opcode >= 0x99 && opcode <= 0x9B) {
                    println!("  >>> This is a RETURN instruction - end of routine");
                    break;
                }
            }
            Err(e) => {
                println!("Error decoding instruction at 0x{:04x}: {}", pc, e);
                break;
            }
        }
        
        steps += 1;
    }
    
    // Let's also search for the "Only bats" string by looking for print instructions
    // in the general area
    println!("\n\n=== Searching for print instructions in the area ===");
    
    for addr in (0x5f00..0x6100).step_by(1) {
        let opcode = interpreter.vm.game.memory[addr];
        
        // Look for print_paddr (0xAD or 0xB2) which prints from a packed address
        if opcode == 0xAD || opcode == 0xB2 {
            println!("\nFound print_paddr at 0x{:04x}", addr);
            match disasm.disassemble_instruction(addr as u32) {
                Ok((_, output)) => println!("{}", output),
                Err(_) => {}
            }
        }
        
        // Also look for call instructions that might call a print routine
        if opcode == 0xE0 || opcode == 0x88 || opcode == 0x8F || opcode == 0x80 {
            match disasm.disassemble_instruction(addr as u32) {
                Ok((_instr, output)) => {
                    if output.contains("call") || output.contains("print") {
                        println!("\nFound call at 0x{:04x}: {}", addr, output);
                    }
                }
                Err(_) => {}
            }
        }
    }

    Ok(())
}