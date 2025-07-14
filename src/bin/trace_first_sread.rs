use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data).map_err(|e| format!("Game load error: {}", e))?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);

    let mut count = 0;
    let mut first_sread_found = false;
    
    loop {
        count += 1;
        let pc = interpreter.vm.pc;
        
        // Decode instruction
        if let Ok(inst) = Instruction::decode(
            &interpreter.vm.game.memory,
            pc as usize,
            interpreter.vm.game.header.version,
        ) {
            let name = inst.name(interpreter.vm.game.header.version);
            
            // Track when we hit sread
            if name == "sread" && !first_sread_found {
                first_sread_found = true;
                println!("\n=== FIRST SREAD at PC {:05x}, instruction {} ===", pc, count);
                
                // Show operands
                println!("Operands: {:?}", inst.operands);
                if inst.operands.len() >= 2 {
                    let text_buffer = inst.operands[0] as u32;
                    let parse_buffer = inst.operands[1] as u32;
                    println!("Text buffer: 0x{:04x}", text_buffer);
                    println!("Parse buffer: 0x{:04x}", parse_buffer);
                    
                    // Check what's in those buffers
                    let text_max = interpreter.vm.read_byte(text_buffer);
                    println!("Text buffer max length: {}", text_max);
                    
                    let parse_max = interpreter.vm.read_byte(parse_buffer);
                    println!("Parse buffer max words: {}", parse_max);
                }
                
                // Show some context - what happened before this?
                println!("\nCall stack depth: {}", interpreter.vm.call_stack.len());
                if let Some(frame) = interpreter.vm.call_stack.last() {
                    println!("Current routine started at: 0x{:05x}", frame.start_pc);
                    println!("Return PC: 0x{:05x}", frame.return_pc);
                }
                
                // Check globals
                println!("\nKey globals:");
                if let Ok(val) = interpreter.vm.read_variable(16) {
                    println!("  G00 (location): {}", val);
                }
                if let Ok(val) = interpreter.vm.read_variable(16 + 111) {
                    println!("  G6f: {}", val);
                }
                
                println!("\nThis SREAD is too early - game should show room description first!");
                break;
            }
            
            // Also track print operations to see what's been output
            if name.contains("print") && count < 1000 {
                println!("[{:04}] {:05x}: {}", count, pc, name);
                if name == "print" || name == "print_ret" {
                    if let Some(text) = &inst.text {
                        println!("      \"{}\"", text);
                    }
                }
            }
            
            // Execute instruction
            match interpreter.execute_instruction(&inst) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Error at PC {:05x}: {}", pc, e);
                    break;
                }
            }
        } else {
            eprintln!("Failed to decode instruction at PC {:05x}", pc);
            break;
        }
        
        if count > 2000 {
            println!("\nStopped after 2000 instructions");
            break;
        }
    }

    Ok(())
}