use gruesome::text;
use gruesome::vm::Game;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;

    println!("=== Searching for 'leaves' object ===\n");

    // Search all objects for ones containing "leave"
    for obj_num in 1..=255 {
        // Get object name directly from memory
        let obj_table_addr = game.header.object_table_addr;
        let property_defaults = obj_table_addr;
        let obj_tree_base = property_defaults + 31 * 2;
        let obj_addr = obj_tree_base + ((obj_num - 1) as usize * 9);

        if obj_addr + 9 > game.memory.len() {
            continue;
        }

        let prop_table_addr = game.memory[(obj_addr + 7)..(obj_addr + 9)]
            .iter()
            .fold(0, |acc, &b| (acc << 8) | b as usize);

        if prop_table_addr == 0 || prop_table_addr >= game.memory.len() {
            continue;
        }

        // Decode object name
        let text_len = game.memory[prop_table_addr] as usize;
        if text_len > 0 {
            let name_addr = prop_table_addr + 1;
            let abbrev_addr = game.header.abbrev_table;
            if let Ok((name, _)) = text::decode_string(&game.memory, name_addr, abbrev_addr) {
                if name.to_lowercase().contains("leave") {
                    println!("Object {obj_num}: \"{name}\"");

                    // Get more details about this object
                    let obj_table_addr = game.header.object_table_addr;
                    let property_defaults = obj_table_addr;
                    let obj_tree_base = property_defaults + 31 * 2;
                    let obj_addr = obj_tree_base + ((obj_num - 1) as usize * 9);
                    let prop_table_addr = game.memory[(obj_addr + 7)..(obj_addr + 9)]
                        .iter()
                        .fold(0, |acc, &b| (acc << 8) | b as usize);

                    let text_len = game.memory[prop_table_addr] as usize;
                    println!("  Property table at: 0x{prop_table_addr:04x}");
                    println!("  Text length byte: {text_len} (0x{text_len:02x})");
                    println!("  Name starts at: 0x{:04x}", prop_table_addr + 1);

                    // Show the raw Z-string bytes
                    print!("  Raw name bytes: ");
                    for i in 0..text_len * 2 {
                        if prop_table_addr + 1 + i < game.memory.len() {
                            print!("{:02x} ", game.memory[prop_table_addr + 1 + i]);
                        }
                    }
                    println!();

                    // Decode manually to see what's happening
                    let name_addr = prop_table_addr + 1;
                    let abbrev_addr = game.header.abbrev_table;
                    match text::decode_string(&game.memory, name_addr, abbrev_addr) {
                        Ok((decoded, bytes_read)) => {
                            println!("  Decoded: \"{decoded}\" ({bytes_read} bytes read)");
                            // Check for issues
                            if decoded != name {
                                println!("  WARNING: Decoded differently!");
                            }
                        }
                        Err(e) => {
                            println!("  Decode error: {e}");
                        }
                    }
                    println!();
                }
            }
        }
    }

    Ok(())
}
