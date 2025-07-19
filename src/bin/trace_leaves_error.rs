use gruesome::interpreter::Interpreter;
use gruesome::vm::{Game, VM};
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

    // Look for the leaves object in the object table
    let obj_table = gruesome::zobject::ObjectTable::new(&game);
    for i in 1..=255 {
        if let Ok(name) = obj_table.get_object_name(i) {
            if name.contains("leave") {
                info!("Object {}: \"{}\"", i, name);
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
