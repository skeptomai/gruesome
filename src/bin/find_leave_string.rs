use gruesome::text;
use gruesome::vm::Game;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;
    let abbrev_addr = game.header.abbrev_table;

    println!("=== Searching for 'leave' string in memory ===\n");

    // Search for strings containing just "leave" (no 's')
    for addr in 0..game.memory.len() - 10 {
        if let Ok((text, _)) = text::decode_string(&game.memory, addr, abbrev_addr) {
            // Look for exact matches or with single space
            if (text == "leave" || text == " leave" || text == "leave ") && text.len() < 10 {
                println!("Found at 0x{:04x}: \"{}\"", addr, text.escape_debug());

                // Show context
                print!("  Bytes: ");
                for i in 0..8.min(game.memory.len() - addr) {
                    print!("{:02x} ", game.memory[addr + i]);
                }
                println!();

                // Check if this could be part of a print instruction
                if addr > 0 {
                    let prev_byte = game.memory[addr - 1];
                    if prev_byte == 0xb2 || (prev_byte & 0xc0) == 0x80 {
                        println!("  >>> Might be part of a print instruction!");
                    }
                }
            }
        }
    }

    // Also check if "leave" is stored as a dictionary word
    println!("\n=== Checking dictionary ===");
    let dict_addr = game.header.dictionary;
    let sep_count = game.memory[dict_addr] as usize;
    let sep_start = dict_addr + 1;
    let entry_length = game.memory[sep_start + sep_count] as usize;
    let entry_count = ((game.memory[sep_start + sep_count + 1] as usize) << 8)
        | game.memory[sep_start + sep_count + 2] as usize;
    let entries_start = sep_start + sep_count + 3;

    for i in 0..entry_count {
        let entry_addr = entries_start + (i * entry_length);
        if let Ok((word, _)) = text::decode_string(&game.memory, entry_addr, abbrev_addr) {
            if word.trim() == "leave" {
                println!("Dictionary word {i}: \"{word}\" at 0x{entry_addr:04x}");
            }
        }
    }

    Ok(())
}
