use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("=== Initial Global Variables ===\n");
    
    // Check first 10 globals
    for i in 0..10 {
        match vm.read_global(i) {
            Ok(value) => {
                println!("Global {:02}: 0x{:04x} ({})", i, value, value);
                
                // Special globals we know about
                match i {
                    0 => println!("  -> Current location"),
                    _ => {}
                }
            }
            Err(e) => println!("Global {:02}: Error: {}", i, e),
        }
    }
    
    println!("\n=== Key Observation ===");
    println!("The bug might be related to an incorrectly");
    println!("initialized global variable that affects");
    println!("command processing.");
    
    Ok(())
}