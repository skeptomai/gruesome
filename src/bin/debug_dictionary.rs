use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("Dictionary address: {:04x}", vm.game.header.dictionary);
    
    // Read dictionary header
    let dict_addr = vm.game.header.dictionary as u32;
    let sep_count = vm.read_byte(dict_addr);
    
    // Skip separators
    let entry_start = dict_addr + 1 + sep_count as u32;
    let entry_length = vm.read_byte(entry_start);
    let entry_count = vm.read_word(entry_start + 1);
    
    println!("Dictionary has {} entries", entry_count);
    println!("Entry length: {} bytes", entry_length);
    println!("Entries start at: {:04x}", entry_start + 3);
    
    // Look for direction words
    println!("\nSearching for direction words:");
    
    let directions = ["n", "north", "s", "south", "e", "east", "w", "west"];
    
    for dir in &directions {
        println!("\nSearching for '{}':", dir);
        
        // Use VM's lookup function
        let addr = vm.lookup_dictionary(dir);
        
        if addr != 0 {
            println!("  Found at dictionary address: {:04x}", addr);
            
            // Show raw bytes
            print!("  Raw bytes:");
            for i in 0..entry_length {
                print!(" {:02x}", vm.read_byte(addr as u32 + i as u32));
            }
            println!();
            
            // Decode the entry
            let word1 = vm.read_word(addr as u32);
            let word2 = vm.read_word(addr as u32 + 2);
            println!("  Encoded as: {:04x} {:04x}", word1, word2);
        } else {
            println!("  NOT FOUND in dictionary!");
        }
    }
    
    // Also dump first few dictionary entries to see format
    println!("\nFirst 10 dictionary entries:");
    let entries_addr = entry_start + 3;
    
    for i in 0..10 {
        let addr = entries_addr + (i * entry_length as u32);
        
        // Read the encoded word
        let word1 = vm.read_word(addr);
        let word2 = vm.read_word(addr + 2);
        
        print!("Entry {}: addr={:04x}, encoded={:04x}{:04x}, ",
            i, addr, word1, word2);
        
        // Show extra data bytes
        print!("data=[");
        for j in 4..entry_length {
            print!("{:02x}", vm.read_byte(addr + j as u32));
            if j < entry_length - 1 { print!(" "); }
        }
        println!("]");
    }
    
    Ok(())
}