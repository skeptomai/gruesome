use std::fs::File;
use std::io::prelude::*;
use infocom::disassembler::Disassembler;
use infocom::vm::Game;

fn main() -> std::io::Result<()> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = match Game::from_memory(game_data) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Failed to load game: {}", e);
            return Ok(());
        }
    };

    let disasm = Disassembler::new(&game);
    
    // Check the GO routine starting at initial PC
    println!("=== Disassembly of GO routine at 0x4f05 (initial PC) ===");
    match disasm.disassemble_range(0x4f05, 0x4fa0) {
        Ok(output) => println!("{}", output),
        Err(e) => eprintln!("Disassembly error: {}", e),
    }
    
    println!("\n=== Looking for where LIT (global 0x52) is set ===");
    match disasm.disassemble_range(0x4f80, 0x4f95) {
        Ok(output) => println!("{}", output),
        Err(e) => eprintln!("Disassembly error: {}", e),
    }

    Ok(())
}