use gruesome::text;
use gruesome::vm::Game;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;

    // We know "leaves" is at dictionary address 0x44a5
    let leaves_addr = 0x44a5;
    let abbrev_addr = game.header.abbrev_table;

    println!("Dictionary entry for 'leaves' at 0x{leaves_addr:04x}:");

    // In V3, dictionary entries are 7 bytes:
    // - 4 bytes: encoded word (2 words of Z-characters)
    // - 3 bytes: data

    // Show raw bytes
    print!("Raw bytes: ");
    for i in 0..7 {
        print!("{:02x} ", game.memory[leaves_addr + i]);
    }
    println!();

    // Decode the word
    if let Ok((word, _)) = text::decode_string(&game.memory, leaves_addr, abbrev_addr) {
        println!("Decoded word: \"{}\" (length {})", word, word.len());
    }

    // Also check "leave" if it exists
    println!("\nLooking for 'leave' in dictionary...");

    // Search dictionary
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
                println!("\nFound 'leave' at 0x{entry_addr:04x}:");
                print!("Raw bytes: ");
                for j in 0..7 {
                    print!("{:02x} ", game.memory[entry_addr + j]);
                }
                println!();
            }
        }
    }

    Ok(())
}
