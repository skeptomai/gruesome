use gruesome::interpreter::Interpreter;
use gruesome::vm::{Game, VM};
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    // Create VM
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);

    println!("Testing Quetzal save/restore functionality...\n");

    // Create a save game directly
    match gruesome::quetzal::save::save_game(&vm) {
        Ok(()) => println!("Save completed successfully!"),
        Err(e) => println!("Save failed: {}", e),
    }

    // List saved file
    if std::path::Path::new("game.sav").exists() {
        let metadata = std::fs::metadata("game.sav")?;
        println!("Created save file: game.sav ({} bytes)", metadata.len());

        // Try to restore
        let mut vm2 = vm;
        match gruesome::quetzal::restore::restore_game(&mut vm2) {
            Ok(()) => println!("Restore completed successfully!"),
            Err(e) => println!("Restore failed: {}", e),
        }
    } else {
        println!("Save file was not created");
    }

    Ok(())
}
