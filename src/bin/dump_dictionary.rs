use infocom::vm::Game;
use std::fs::File;
use std::io::prelude::*;

fn decode_dict_word(word1: u16, word2: u16) -> String {
    // Simple Z-string decoder for dictionary words (V3)
    // Each word is 5 chars packed into 2 16-bit words
    let mut result = String::new();
    let mut chars = Vec::new();
    
    // Extract 5-bit values
    chars.push(((word1 >> 10) & 0x1F) as u8);
    chars.push(((word1 >> 5) & 0x1F) as u8);
    chars.push((word1 & 0x1F) as u8);
    chars.push(((word2 >> 10) & 0x1F) as u8);
    chars.push(((word2 >> 5) & 0x1F) as u8);
    chars.push((word2 & 0x1F) as u8);
    
    for ch in chars {
        if ch == 0 {
            break; // padding
        } else if ch >= 6 && ch <= 31 {
            // A-Z (6-31 -> A-Z)
            result.push(((ch - 6) + b'a') as char);
        } else if ch == 1 {
            result.push_str("[abbrev]");
        } else if ch == 2 {
            result.push_str("[shift+]");
        } else if ch == 3 {
            result.push_str("[shift-]");
        } else if ch == 4 {
            result.push_str("[shift]");
        } else if ch == 5 {
            result.push(' ');
        }
    }
    
    result
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    
    println!("Z-Machine Dictionary Analysis\n");
    
    let dict_addr = game.header.dictionary;
    println!("Dictionary address: 0x{:04x}", dict_addr);
    
    // Read dictionary header
    let sep_count = game.memory[dict_addr];
    println!("Word separator count: {}", sep_count);
    
    // Print separators
    print!("Separators: ");
    for i in 0..sep_count {
        let sep = game.memory[dict_addr + 1 + i as usize];
        print!("'{}' (0x{:02x}) ", sep as char, sep);
    }
    println!();
    
    // Dictionary entries start after separators
    let entry_start = dict_addr + 1 + sep_count as usize;
    let entry_length = game.memory[entry_start];
    let entry_count = ((game.memory[entry_start + 1] as u16) << 8) | 
                      (game.memory[entry_start + 2] as u16);
    
    println!("\nEntry length: {} bytes", entry_length);
    println!("Entry count: {} words", entry_count);
    
    // For V3, dictionary entries are 7 bytes:
    // - 4 bytes: Z-encoded text (2 words)
    // - 3 bytes: data (usually flags and attributes)
    
    println!("\nFirst 50 dictionary entries:");
    let entries_addr = entry_start + 3;
    
    for i in 0..50.min(entry_count) {
        let addr = entries_addr + (i as usize * entry_length as usize);
        
        // Read the Z-encoded text (4 bytes = 2 words)
        let word1 = ((game.memory[addr] as u16) << 8) | (game.memory[addr + 1] as u16);
        let word2 = ((game.memory[addr + 2] as u16) << 8) | (game.memory[addr + 3] as u16);
        
        // Decode the Z-string
        let text = decode_dict_word(word1, word2);
        
        print!("Entry {:3}: 0x{:04x} = '{:9}' ", i, addr, text);
        // Show the raw bytes
        print!("[");
        for j in 0..entry_length {
            print!("{:02x}", game.memory[addr + j as usize]);
            if j < entry_length - 1 { print!(" "); }
        }
        println!("]");
    }
    
    // Now look for specific words we care about
    println!("\nSearching for command words:");
    let test_words = ["quit", "look", "west", "north", "take", "go", "open", "mailb"];
    
    for word in &test_words {
        for i in 0..entry_count {
            let addr = entries_addr + (i as usize * entry_length as usize);
            let word1 = ((game.memory[addr] as u16) << 8) | (game.memory[addr + 1] as u16);
            let word2 = ((game.memory[addr + 2] as u16) << 8) | (game.memory[addr + 3] as u16);
            let text = decode_dict_word(word1, word2);
            
            if text.trim() == *word {
                println!("Found '{}' at entry {} (address 0x{:04x})", word, i, addr);
                break;
            }
        }
    }
    
    Ok(())
}