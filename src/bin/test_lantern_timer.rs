use gruesome::interpreter::Interpreter;
use gruesome::vm::{Game, VM};
use log::info;
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
    
    // Hook into read_global to track G88 changes
    let mut last_g88 = 0u16;
    let mut turn_count = 0;
    
    // Run game and monitor G88
    loop {
        // Check G88 before each turn
        if let Ok(g88) = interpreter.vm.read_global(0x58) {
            if g88 != last_g88 {
                info!("Turn {}: G88 changed from {} to {} (delta: {})", 
                      turn_count, last_g88, g88, g88 as i16 - last_g88 as i16);
                last_g88 = g88;
            }
        }
        
        // Get a single command
        turn_count += 1;
        info!("Turn {}: Waiting for input...", turn_count);
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        
        if input.trim() == "quit" {
            break;
        }
        
        // For testing: manually simulate what the game loop does
        // This is a simplified version - real game is more complex
        info!("Processing: {}", input.trim());
    }
    
    Ok(())
}