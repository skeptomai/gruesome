use gruesome::interpreter::Interpreter;
use gruesome::vm::{Game, VM};
use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enable debug logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("gruesome=debug"))
        .init();
    
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    println!("Starting game with timer debug enabled...");
    println!("Type 'quit' to exit\n");
    
    // Run the interpreter normally
    if let Err(e) = interpreter.run() {
        eprintln!("Error: {}", e);
    }
    
    Ok(())
}