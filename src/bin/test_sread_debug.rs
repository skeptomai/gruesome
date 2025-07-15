use infocom::interpreter::Interpreter;
use infocom::vm::{Game, VM};
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    // Enable debug mode
    interpreter.set_debug(true);
    
    println!("Testing SREAD with debug enabled\n");
    
    // Run for a short time to get to the SREAD prompt
    match interpreter.run_with_limit(Some(5000)) {
        Ok(()) => println!("Reached instruction limit"),
        Err(e) => println!("Error: {}", e),
    }
    
    Ok(())
}