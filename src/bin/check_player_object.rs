use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("=== Player Object Analysis ===\n");
    
    // Check common globals that might store the player object
    println!("Checking globals that might contain player object:");
    for i in 0..10 {
        if let Ok(value) = vm.read_global(i) {
            println!("Global {:02}: {} (0x{:04x})", i, value, value);
            if value == 4 {
                println!("  ^^^ This is object 4 (cretin - NO ACTION)!");
            } else if value == 5 {
                println!("  ^^^ This is object 5 (you - HAS ACTION)!");
            }
        }
    }
    
    println!("\n=== The Bug ===");
    println!("Object 4 'cretin' has property 17 = 00 00 (no action handler)");
    println!("Object 5 'you' has property 17 = 29 5c (valid action handler)");
    println!();
    println!("When processing 'w' (type 0x32), the game checks the");
    println!("player object's property 17. If it's using object 4,");
    println!("it gets NULL and fails!");
    println!();
    println!("The fix: Make sure the correct player object (5) is set.");
    
    Ok(())
}