use infocom::vm::{Game, VM};
use infocom::interpreter::{Interpreter, ExecutionResult};
use env_logger;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    // Enable debug mode
    interpreter.set_debug(true);
    
    // Enable single-step debugging for PERFORM
    interpreter.enable_single_step(0x577c, 0x5800);
    
    println!("=== Running game with single-step in PERFORM ===");
    println!("Type 'w' when prompted to see the issue.\n");
    
    // Run the game
    match interpreter.run() {
        Ok(_) => println!("\nGame ended normally"),
        Err(e) => eprintln!("\nGame error: {}", e),
    }
    
    Ok(())
}