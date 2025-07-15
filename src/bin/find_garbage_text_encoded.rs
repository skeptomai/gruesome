use infocom::vm::{Game, VM};
use infocom::text;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("=== Searching for garbage text ===");
    println!("Looking for: 'can you attack a spirit with material objects?'");
    println!();
    
    // Search through possible text addresses
    // Start from a reasonable offset (after header and tables)
    let start = 0x1000;
    let end = vm.game.memory.len().min(0x20000);
    
    let mut found_count = 0;
    
    for addr in start..end {
        // Try to decode as Z-string
        match text::decode_string(&vm.game.memory[addr..], vm.game.header.abbrev_table, vm.game.header.version as usize) {
            Ok((text, _)) => {
                if text.contains("can you attack") || text.contains("spirit with material") {
                    println!("FOUND at 0x{:04x}: '{}'", addr, text);
                    
                    // Also show as packed address
                    let packed = addr / 2;
                    println!("  As packed address: 0x{:04x}", packed);
                    
                    found_count += 1;
                    if found_count >= 5 {
                        break;
                    }
                }
            }
            Err(_) => {} // Ignore decode errors
        }
    }
    
    if found_count == 0 {
        println!("Not found in the searched range.");
        println!("\nThe text might be:");
        println!("1. In a different memory region");
        println!("2. Constructed dynamically");
        println!("3. Part of a larger string that's being indexed incorrectly");
    }
    
    Ok(())
}