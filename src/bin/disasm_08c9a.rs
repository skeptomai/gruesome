use infocom::disassembler::Disassembler;
use infocom::vm::{Game, VM};
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let disasm = Disassembler::new(&game);
    
    println!("Disassembling routine at 0x08c9a...\n");
    
    // First show the specific area we're interested in
    println!("Area around the problematic call_2s:");
    match disasm.disassemble_range(0x08ca0, 0x30) {
        Ok(output) => {
            println!("{}", output);
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    
    // Also show a bit before to see the routine header
    println!("\n\nShowing bytes before 0x08c9a to find routine start:");
    for addr in (0x08c90..0x08c9a).rev() {
        if game.memory[addr] <= 15 {
            println!("Possible routine start at 0x{:05x} with {} locals", addr, game.memory[addr]);
            
            // Show disassembly from this point
            match disasm.disassemble_range(addr as u32, 0x08cd0 - addr as u32) {
                Ok(output) => {
                    println!("\nDisassembly from 0x{:05x}:", addr);
                    println!("{}", output);
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
            break;
        }
    }
    
    Ok(())
}