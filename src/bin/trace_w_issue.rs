use infocom::vm::{Game, VM};
use infocom::instruction::Instruction;
use infocom::interpreter::{Interpreter, ExecutionResult};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Summary of the 'w' bug ===\n");
    
    println!("What we know:");
    println!("1. 'w' shows garbage: \"w can you attack a spirit with material objects?\"");
    println!("2. 'w' should show: \"Forest\" and move the player");
    println!("3. Dictionary entry for 'w' has type 0x32 with data 0xa1 0x1d");
    println!("4. Dictionary entry for 'e' has type 0x13 with data 0x1e 0x00");
    println!("\nThe bug appears to be that type 0x32 entries are handled incorrectly.");
    println!("The game code might be using 0xa11d as an action number or text address.\n");
    
    // Load the game to examine key routines
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    // Key hypothesis: The game expects dictionary entries to have different formats
    // Type 0x13: Simple verb with action number in byte 5
    // Type 0x32: Complex entry - possibly object/property reference
    
    println!("Suggested debugging approach:");
    println!("1. Add a breakpoint or log when dictionary entry type 0x32 is encountered");
    println!("2. Trace what the game does with the data bytes 0xa1 0x1d");
    println!("3. Check if there's special handling code for type 0x32 that we're missing");
    
    println!("\nTo debug this interactively:");
    println!("1. Run: cargo build");
    println!("2. Run: RUST_LOG=debug cargo run --bin infocom resources/test/zork1/DATA/ZORK1.DAT 2>debug.log");
    println!("3. Type 'w' at the prompt");
    println!("4. Look in debug.log for clues about what happens with the dictionary data");
    
    println!("\nThe fix likely involves:");
    println!("- Finding where the game checks dictionary entry type (byte 4)");
    println!("- Understanding how type 0x32 entries should be processed");
    println!("- Possibly implementing special handling in our interpreter");
    
    Ok(())
}