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
    
    println!("=== Checking packed address 0x4e1c ===\n");
    
    // Decode text at packed address 0x4e1c
    let packed_addr = 0x4e1c;
    // For Z3, multiply by 2
    let unpacked = (packed_addr as usize) * 2;
    
    println!("Packed address: 0x{:04x}", packed_addr);
    println!("Unpacked address: 0x{:04x}", unpacked);
    
    // Decode the text
    match text::decode_string(&vm.game.memory[unpacked..], vm.game.header.abbrev_table, vm.game.header.version as usize) {
        Ok((text, _)) => {
            println!("\nDecoded text: '{}'", text);
            
            // Check if this is our garbage text
            if text.contains("can you attack") {
                println!("\n*** FOUND THE GARBAGE TEXT! ***");
                println!("This is being printed from packed address 0x{:04x}", packed_addr);
            }
        }
        Err(e) => println!("Error decoding: {}", e),
    }
    
    // Let's also check what's happening at PC 08adc where print_paddr is called
    println!("\n=== Context at PC 08adc ===");
    
    // This is inside routine at 8aa4
    println!("This print_paddr is in routine at 0x8aa4");
    println!("The routine was called with obj=180 (West of House), prop=29");
    
    // Property 29 might be the 'west' exit property
    println!("\nChecking property 29 of West of House (object 180):");
    
    // The problem might be that the game is trying to print an error message
    // for why we can't go west, but it's using the wrong address
    
    Ok(())
}