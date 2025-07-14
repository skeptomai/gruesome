use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data).map_err(|e| format!("Game load error: {}", e))?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);

    // Run a limited number of instructions
    let mut count = 0;
    loop {
        count += 1;
        
        // Check interesting globals periodically
        if count % 100 == 0 {
            println!("\n=== After {} instructions ===", count);
            
            // G00 - Current location
            if let Ok(val) = interpreter.vm.read_variable(16) {
                println!("G00 (location): {}", val);
            }
            
            // G6f (111) - Referenced in STAND routine
            if let Ok(val) = interpreter.vm.read_variable(16 + 111) {
                println!("G6f (111): {}", val);
            }
            
            // G52 (82) - LIT
            if let Ok(val) = interpreter.vm.read_variable(16 + 82) {
                println!("G52 (LIT): {}", val);
            }
            
            println!("PC: {:05x}", interpreter.vm.pc);
        }
        
        // Try to capture when we hit the STAND routine
        if interpreter.vm.pc == 0x86ca {
            println!("\n!!! HIT STAND ROUTINE at instruction {} !!!", count);
            
            // Dump all globals
            println!("\nGlobal variables at STAND entry:");
            for i in 0..240 {
                if let Ok(val) = interpreter.vm.read_variable(16 + i) {
                    if val != 0 {
                        println!("  G{:02x}: {} (0x{:04x})", i, val, val);
                    }
                }
            }
            
            // Check parse buffer to see what command was parsed
            // Usually parse buffer is in low memory
            println!("\nChecking common parse buffer locations:");
            for addr in [0x70, 0x80, 0x90, 0x100, 0x200].iter() {
                let max_words = interpreter.vm.read_byte(*addr);
                let num_words = interpreter.vm.read_byte(*addr + 1);
                if max_words > 0 && max_words < 20 && num_words <= max_words {
                    println!("\nPossible parse buffer at 0x{:04x}:", addr);
                    println!("  Max words: {}, Actual words: {}", max_words, num_words);
                    
                    for i in 0..num_words {
                        let offset = addr + 2 + (i as u32 * 4);
                        let text_pos = interpreter.vm.read_byte(offset);
                        let word_len = interpreter.vm.read_byte(offset + 1);
                        let dict_addr = interpreter.vm.read_word(offset + 2)?;
                        println!("  Word {}: pos={}, len={}, dict=0x{:04x}", 
                                i, text_pos, word_len, dict_addr);
                    }
                }
            }
            
            break;
        }
        
        if count > 5000 {
            println!("\nStopped after 5000 instructions without hitting STAND");
            break;
        }
        
        // Execute one instruction
        let pc = interpreter.vm.pc;
        if let Ok(inst) = infocom::instruction::Instruction::decode(
            &interpreter.vm.game.memory,
            pc as usize,
            interpreter.vm.game.header.version,
        ) {
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
    }

    Ok(())
}