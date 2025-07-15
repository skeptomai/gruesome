use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("=== Static Analysis of 'w' Dictionary Entry ===\n");
    
    // The 'w' entry has type 0x32 with data 0xa1 0x1d
    println!("Dictionary entry for 'w' has:");
    println!("  Type byte (byte 4): 0x32");
    println!("  Data bytes: 0xa1 0x1d");
    
    // Theory 1: It's an object number (0xa1) and property (0x1d)
    println!("\nTheory 1: Object 0xa1 (161), Property 0x1d (29)");
    println!("  This would mean: Get property 29 of object 161");
    
    // Theory 2: It's a packed address
    println!("\nTheory 2: Packed address");
    let packed = (0xa1 << 8) | 0x1d;
    println!("  As big-endian word: 0x{:04x} = {}", packed, packed);
    let packed_rev = (0x1d << 8) | 0xa1;
    println!("  As little-endian word: 0x{:04x} = {}", packed_rev, packed_rev);
    
    // Theory 3: The bytes are separate values
    println!("\nTheory 3: Separate values");
    println!("  0xa1 = {} (decimal)", 0xa1);
    println!("  0x1d = {} (decimal)", 0x1d);
    
    // Let's check what a "normal" direction looks like
    println!("\n=== Comparison with working directions ===");
    println!("'e' has: type=0x13, data=0x1e 0x00");
    println!("'n' has: type=0x13, data=0x1f 0x00");
    println!("These are simple action numbers (30 and 31)");
    
    // Search for where type 0x32 might be handled
    println!("\n=== Looking for type 0x32 handling code ===");
    println!("We need to find code that:");
    println!("1. Loads byte 4 from a dictionary entry");
    println!("2. Compares it with 0x32");
    println!("3. Takes a different path for type 0x32");
    
    // The garbage text suggests wrong memory is being read
    println!("\n=== Garbage text analysis ===");
    println!("User reported: \"w can you attack a spirit with material objects?\"");
    println!("This suggests text is being read from the wrong address");
    println!("Possibly because 0xa11d is being used as a text pointer");
    
    println!("\nTo debug this, please run the game and when you see the prompt:");
    println!("1. Type 'w' and press enter");
    println!("2. When you see garbage text, press Ctrl+C");
    println!("3. Report back what garbage text you saw");
    println!("\nAlso helpful: Can you run the game with a working interpreter");
    println!("and check what happens when you type 'w' from West of House?");
    println!("Does it say something like \"You can't go that way\"?");
    
    Ok(())
}