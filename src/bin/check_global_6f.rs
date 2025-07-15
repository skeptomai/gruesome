use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("=== Checking Global 0x6f (111) ===\n");
    
    // Global 0x6f is at index 111
    match vm.read_global(0x6f) {
        Ok(value) => {
            println!("Global 0x6f = {} (0x{:04x})", value, value);
            if value == 4 {
                println!("This is object 4 (cretin) - NO ACTION HANDLER!");
            }
        }
        Err(e) => println!("Error reading global 0x6f: {}", e),
    }
    
    println!("\nFrom routine 0x577c:");
    println!("- At 0x57f7: GET_PROP G6f,#11 -> -(SP)");
    println!("- At 0x57fb: CALL (SP)+ -> L03");
    println!();
    println!("This gets property 17 of whatever object is in G6f");
    println!("and calls it as a function.");
    println!();
    println!("If G6f contains object 4, property 17 = 00 00,");
    println!("causing the NULL call we see in the debug log.");
    
    Ok(())
}