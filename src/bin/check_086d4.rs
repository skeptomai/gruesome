use std::fs::File;
use std::io::prelude::*;
use infocom::disassembler::Disassembler;
use infocom::vm::Game;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data).map_err(|e| format!("Game load error: {}", e))?;
    let disasm = Disassembler::new(&game);

    println!("Checking what's at 0x086d4 (where branch should go)...\n");
    
    println!("Disassembly from 0x086d4:");
    match disasm.disassemble_range(0x086d4, 0x08700) {
        Ok(output) => {
            for line in output.lines().take(10) {
                println!("{}", line);
            }
        }
        Err(e) => println!("Error: {}", e),
    }
    
    println!("\n\nFor comparison, STAND routine at 0x086ca:");
    match disasm.disassemble_range(0x086ca, 0x086e0) {
        Ok(output) => println!("{}", output),
        Err(e) => println!("Error: {}", e),
    }
    
    // Let's also check the exact bytes
    println!("\n\nRaw bytes comparison:");
    println!("At 0x086d4: {:02x} {:02x} {:02x} {:02x}", 
             game.memory[0x086d4], game.memory[0x086d5], 
             game.memory[0x086d6], game.memory[0x086d7]);
    println!("At 0x086ca: {:02x} {:02x} {:02x} {:02x}", 
             game.memory[0x086ca], game.memory[0x086cb], 
             game.memory[0x086cc], game.memory[0x086cd]);

    Ok(())
}