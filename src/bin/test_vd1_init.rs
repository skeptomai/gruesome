use infocom::vm::{Game, VM};
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);
    
    // Check if Vd1 is initialized correctly
    match vm.read_variable(0xd1) {
        Ok(value) => {
            println!("Vd1 initialized to: {}", value);
            if value == 6 {
                println!("SUCCESS: Vd1 contains LOOK action code (6)");
            } else {
                println!("FAIL: Vd1 should be 6, but is {}", value);
            }
        }
        Err(e) => {
            println!("Error reading Vd1: {}", e);
        }
    }
    
    Ok(())
}