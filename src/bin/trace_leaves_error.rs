use gruesome::vm::Game;
use log::info;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("gruesome=debug"))
        .format_timestamp(None)
        .init();

    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    // First find the "leaves" object
    let game = Game::from_memory(memory)?;

    // Look for the leaves object by searching object names in memory
    let obj_table_addr = game.header.object_table_addr as usize;
    let property_defaults = obj_table_addr;
    let obj_tree_base = property_defaults + 31 * 2; // v3 has 31 default properties

    for obj_num in 1..=255 {
        let obj_addr = obj_tree_base + ((obj_num - 1) * 9);
        if obj_addr + 9 > game.memory.len() {
            break;
        }

        // Get property table address
        let prop_addr =
            ((game.memory[obj_addr + 7] as usize) << 8) | (game.memory[obj_addr + 8] as usize);
        if prop_addr == 0 || prop_addr >= game.memory.len() {
            continue;
        }

        // Object name is at the start of property table
        let name_len = game.memory[prop_addr] as usize;
        if name_len > 0 {
            let name_addr = prop_addr + 1;
            if let Ok((name, _)) = gruesome::text::decode_string(
                &game.memory,
                name_addr,
                game.header.abbrev_table as usize,
            ) {
                if name.contains("leave") {
                    info!("Object {}: \"{}\"", obj_num, name);
                }
            }
        }
    }

    // Also search for the error message pattern
    info!("\nSearching for error message patterns...");

    // The message is likely "You can't see any X here!" where X is object name
    // Let's look for "can't see any"
    let pattern = b"can't see any";
    for i in 0..game.memory.len() - pattern.len() {
        if &game.memory[i..i + pattern.len()] == pattern {
            info!("Found 'can't see any' at 0x{:04x}", i);
            // Show surrounding context
            let start = i.saturating_sub(20);
            let end = (i + 50).min(game.memory.len());
            info!("Context: {:?}", &game.memory[start..end]);
        }
    }

    Ok(())
}
