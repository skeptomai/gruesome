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
    
    println!("Debugging room description issue...\n");
    
    // Run the game until we get the initial room description
    println!("Running initial startup...");
    match interpreter.run_with_limit(Some(2000)) {
        Ok(()) => println!("Reached instruction limit"),
        Err(e) => println!("Error: {}", e),
    }
    
    Ok(())
}