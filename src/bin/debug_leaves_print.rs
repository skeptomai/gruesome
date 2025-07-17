use gruesome::interpreter::Interpreter;
use gruesome::vm::{Game, VM};
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("gruesome=info"))
        .format_timestamp(None)
        .init();
    
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    println!("=== Testing 'leaves' printing ===");
    println!("Running commands: w, n, move leaves, w, move leaves");
    println!("The error should show 'leaves' not ' leave'");
    println!();
    
    // Run the interpreter
    if let Err(e) = interpreter.run() {
        eprintln!("Error: {}", e);
    }
    
    Ok(())
}