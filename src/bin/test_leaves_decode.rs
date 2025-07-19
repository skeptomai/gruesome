use gruesome::text;
use gruesome::vm::Game;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;

    // We know object 144 is "pile of leaves"
    // Let's manually decode it
    let obj_num = 144u16;
    let obj_table_addr = game.header.object_table_addr as usize;
    let property_defaults = obj_table_addr;
    let obj_tree_base = property_defaults + 31 * 2;
    let obj_addr = obj_tree_base + ((obj_num - 1) as usize * 9);

    let prop_table_addr =
        ((game.memory[obj_addr + 7] as usize) << 8) | (game.memory[obj_addr + 8] as usize);

    let text_len = game.memory[prop_table_addr] as usize;
    let name_addr = prop_table_addr + 1;
    let abbrev_addr = game.header.abbrev_table as usize;

    println!("Object 144 details:");
    println!("  Property table: 0x{:04x}", prop_table_addr);
    println!("  Text length: {} words", text_len);
    println!("  Name address: 0x{:04x}", name_addr);

    // Full decode
    match text::decode_string(&game.memory, name_addr, abbrev_addr) {
        Ok((name, bytes_read)) => {
            println!("  Full name: \"{}\"", name);
            println!("  Bytes read: {}", bytes_read);
        }
        Err(e) => {
            println!("  Decode error: {}", e);
        }
    }

    // Now let's see what happens if we decode with different lengths
    println!("\nTesting partial decodes:");

    // Try decoding only part of the string
    for words in 1..=text_len {
        print!("  Decoding {} words: ", words);

        // Manually set the end bit on the last word we want to decode
        let mut test_memory = game.memory.clone();
        if words < text_len {
            let word_addr = name_addr + (words - 1) * 2;
            test_memory[word_addr] |= 0x80; // Set high bit to mark end
        }

        match text::decode_string(&test_memory, name_addr, abbrev_addr) {
            Ok((name, _)) => {
                println!("\"{}\"", name);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }

    // Check if "leave" appears anywhere
    println!("\nSearching for 'leave' (without 's') in object names:");
    for obj in 1..=255u16 {
        let obj_addr = obj_tree_base + ((obj - 1) as usize * 9);
        let prop_addr =
            ((game.memory[obj_addr + 7] as usize) << 8) | (game.memory[obj_addr + 8] as usize);

        if prop_addr > 0 && prop_addr < game.memory.len() {
            let tlen = game.memory[prop_addr] as usize;
            if tlen > 0 && tlen < 10 {
                let naddr = prop_addr + 1;
                if let Ok((name, _)) = text::decode_string(&game.memory, naddr, abbrev_addr) {
                    if name == "leave" || name == " leave" {
                        println!("  Object {}: \"{}\"", obj, name);
                    }
                }
            }
        }
    }

    Ok(())
}
