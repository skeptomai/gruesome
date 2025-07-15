use infocom::interpreter::{Interpreter, ExecutionResult};
use infocom::vm::{Game, VM};
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("=== Simple Quit Trace ===");
    println!("This tool runs the game normally and looks for quit-related text\n");

    // Load Zork 1
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    // Run the game normally
    println!("Running game... (this will output directly to console)");
    println!("Type 'quit' when you see the prompt\n");
    
    match interpreter.run() {
        Ok(()) => {
            println!("\nGame completed normally");
        }
        Err(e) => {
            println!("\nGame error: {}", e);
        }
    }
    
    Ok(())
}