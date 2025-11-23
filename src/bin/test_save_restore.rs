use gruesome::interpreter::core::vm::{Game, VM};
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

    // Create a save game using the non-interactive API (for testing)
    use gruesome::interpreter::quetzal::save::SaveGame;
    let save = SaveGame::from_vm(&vm).expect("Failed to create SaveGame");
    match save.save_to_file(std::path::Path::new("game.sav")) {
        Ok(()) => println!("Save completed successfully!"),
        Err(e) => println!("Save failed: {e}"),
    }

    // List saved file
    if std::path::Path::new("game.sav").exists() {
        let metadata = std::fs::metadata("game.sav")?;
        println!("Created save file: game.sav ({} bytes)", metadata.len());

        // Test restore using the non-interactive API
        let mut vm2 = vm;
        use gruesome::interpreter::quetzal::restore::RestoreGame;
        match RestoreGame::from_file(std::path::Path::new("game.sav")) {
            Ok(restore) => match restore.restore_to_vm(&mut vm2) {
                Ok(()) => println!("Restore completed successfully!"),
                Err(e) => println!("Restore failed: {e}"),
            },
            Err(e) => println!("Failed to load save file: {e}"),
        }
    } else {
        println!("Save file was not created");
    }

    Ok(())
}
