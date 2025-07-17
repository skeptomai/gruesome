use gruesome::vm::Game;
use gruesome::text;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;
    
    let game = Game::from_memory(memory)?;
    
    println!("Searching for 'leave' text patterns...\n");
    
    // Search for text containing "leave" in various forms
    let patterns = vec!["leave", "leaves", " leave", "  leave"];
    let abbrev_table = game.header.abbrev_table as usize;
    
    for addr in 0..memory.len() - 10 {
        if let Ok(text) = text::decode_string(&memory, addr, abbrev_table) {
            for pattern in &patterns {
                if text.contains(pattern) && text.len() < 100 {
                    println!("Found at 0x{:04x}: \"{}\"", addr, text);
                    
                    // Show the raw bytes
                    print!("  Raw bytes: ");
                    for i in 0..10.min(memory.len() - addr) {
                        print!("{:02x} ", memory[addr + i]);
                    }
                    println!();
                    
                    // If it looks like our problematic text
                    if text.contains("can't see any") || text.contains(" leave") {
                        println!("  >>> This might be our problematic text!");
                        
                        // Decode the Z-characters in detail
                        println!("  Detailed decode:");
                        let mut pos = addr;
                        for _ in 0..5 { // Look at first 5 words
                            let word = ((memory[pos] as u16) << 8) | memory[pos + 1] as u16;
                            let z1 = (word >> 10) & 0x1f;
                            let z2 = (word >> 5) & 0x1f;
                            let z3 = word & 0x1f;
                            println!("    Word at 0x{:04x}: {:04x} => z-chars: {}, {}, {}", 
                                     pos, word, z1, z2, z3);
                            pos += 2;
                            if word & 0x8000 != 0 {
                                break;
                            }
                        }
                    }
                    break;
                }
            }
        }
    }
    
    // Also check dictionary
    println!("\nChecking dictionary entries:");
    let dict_addr = game.header.dict_addr as usize;
    let sep_count = memory[dict_addr] as usize;
    let sep_start = dict_addr + 1;
    let entry_length = memory[sep_start + sep_count] as usize;
    let entry_count = ((memory[sep_start + sep_count + 1] as usize) << 8) 
                    | memory[sep_start + sep_count + 2] as usize;
    let entries_start = sep_start + sep_count + 3;
    
    for i in 0..entry_count {
        let entry_addr = entries_start + (i * entry_length);
        if let Ok(word) = text::decode_string(&memory, entry_addr, abbrev_table) {
            if word.contains("leave") {
                println!("Dictionary entry {}: \"{}\" at 0x{:04x}", i, word, entry_addr);
            }
        }
    }
    
    Ok(())
}