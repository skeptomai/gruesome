use infocom::vm::{Game, VM};
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);
    
    println!("Checking Z-Machine dictionary...\n");
    
    // Find dictionary location
    let dict_addr = vm.get_dictionary_addr();
    println!("Dictionary address: 0x{:04x}", dict_addr);
    
    // Read dictionary header
    let sep_count = vm.read_byte(dict_addr);
    println!("Word separator count: {}", sep_count);
    
    // Skip separators
    let entry_start = dict_addr + 1 + sep_count as u32;
    let entry_length = vm.read_byte(entry_start);
    let entry_count = vm.read_word(entry_start + 1);
    
    println!("Entry length: {} bytes", entry_length);
    println!("Entry count: {} words", entry_count);
    
    // First few entries
    println!("\nFirst 10 dictionary entries:");
    let entries_addr = entry_start + 3;
    
    for i in 0..10.min(entry_count) {
        let addr = entries_addr + (i as u32 * entry_length as u32);
        print!("Entry {}: 0x{:04x} = ", i, addr);
        
        // Read Z-string (4 bytes for V3)
        let b1 = vm.read_byte(addr);
        let b2 = vm.read_byte(addr + 1);
        let b3 = vm.read_byte(addr + 2);
        let b4 = vm.read_byte(addr + 3);
        
        println!("{:02x} {:02x} {:02x} {:02x}", b1, b2, b3, b4);
    }
    
    println!("\nLooking for common words:");
    
    // Look for specific words
    let test_words = ["quit", "look", "west", "north", "take", "go"];
    
    for word in &test_words {
        println!("\nSearching for '{}':", word);
        
        // Simple linear search through dictionary
        for i in 0..entry_count {
            let addr = entries_addr + (i as u32 * entry_length as u32);
            // Would need proper Z-string decoding here
            // For now just show if we found something that might match
            if i < 20 {
                // Show first 20 for manual inspection
                let b1 = vm.read_byte(addr);
                let b2 = vm.read_byte(addr + 1);
                if word == "quit" && b1 == 0x3b && b2 == 0x2a {
                    println!("  Possible match at entry {} (0x{:04x})", i, addr);
                }
            }
        }
    }
    
    Ok(())
}