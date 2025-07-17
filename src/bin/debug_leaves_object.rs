use gruesome::vm::Game;
use gruesome::text;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;
    
    let game = Game::from_memory(memory)?;
    
    println!("=== Searching for 'leaves' object ===\n");
    
    // Get object table info
    let obj_table_addr = game.header.object_table_addr as usize;
    let property_defaults = obj_table_addr;
    let obj_tree_base = property_defaults + 31 * 2;
    let abbrev_addr = game.header.abbrev_table as usize;
    
    // Search all objects
    for obj_num in 1..=255u16 {
        let obj_addr = obj_tree_base + ((obj_num - 1) as usize * 9);
        
        // Get property table address
        let prop_table_addr = ((game.memory[obj_addr + 7] as usize) << 8) 
                            | (game.memory[obj_addr + 8] as usize);
        
        if prop_table_addr == 0 || prop_table_addr >= game.memory.len() {
            continue;
        }
        
        // The first byte is the text-length
        let text_len = game.memory[prop_table_addr] as usize;
        
        if text_len > 0 && text_len < 10 {
            // Try to decode the object name
            let name_addr = prop_table_addr + 1;
            
            if let Ok((name, _)) = text::decode_string(&game.memory, name_addr, abbrev_addr) {
                if name.to_lowercase().contains("leave") {
                    println!("Object {}: \"{}\"", obj_num, name);
                    println!("  Property table at: 0x{:04x}", prop_table_addr);
                    println!("  Text length: {} words", text_len);
                    println!("  Name address: 0x{:04x}", name_addr);
                    
                    // Show raw bytes
                    print!("  Raw Z-string words: ");
                    for i in 0..text_len {
                        let addr = name_addr + i * 2;
                        if addr + 1 < game.memory.len() {
                            let word = ((game.memory[addr] as u16) << 8) | game.memory[addr + 1] as u16;
                            print!("{:04x} ", word);
                        }
                    }
                    println!();
                    
                    // Decode each word to see the Z-characters
                    for i in 0..text_len {
                        let addr = name_addr + i * 2;
                        if addr + 1 < game.memory.len() {
                            let word = ((game.memory[addr] as u16) << 8) | game.memory[addr + 1] as u16;
                            let z1 = (word >> 10) & 0x1f;
                            let z2 = (word >> 5) & 0x1f;
                            let z3 = word & 0x1f;
                            let is_end = (word & 0x8000) != 0;
                            println!("    Word {}: z-chars [{}, {}, {}] {}", 
                                     i, z1, z2, z3, if is_end { "(END)" } else { "" });
                        }
                    }
                    
                    // Check if there's padding
                    if name.trim() != name {
                        println!("  WARNING: Name has whitespace!");
                        println!("  Name bytes: {:?}", name.as_bytes());
                    }
                    
                    println!();
                }
            }
        }
    }
    
    Ok(())
}