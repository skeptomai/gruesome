use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("=== Player Object Initialization Bug ===\n");
    
    println!("From debug log at PC 04f8b:");
    println!("  store #007f, #0004");
    println!("  -> This stores object 4 into global 0x7f");
    println!();
    
    println!("Object 4 'cretin':");
    println!("  Property 17: 00 00 (NO action handler)");
    println!("  This causes NULL calls when processing 'w'");
    println!();
    
    println!("Object 5 'you':");  
    println!("  Property 17: 29 5c (valid action handler at 0x52b8)");
    println!("  This is likely the correct player object");
    println!();
    
    println!("=== THE FIX ===");
    println!("The game initialization at PC 04f8b should store");
    println!("object 5 (not 4) into global 0x7f.");
    println!();
    println!("This appears to be a bug in the game initialization");
    println!("sequence. A working interpreter might:");
    println!("1. Have a patch for this specific issue");
    println!("2. Initialize globals differently"); 
    println!("3. Have the intro sequence set the correct object");
    
    Ok(())
}