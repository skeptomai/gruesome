use infocom::vm::{Game, VM};
use infocom::interpreter::Interpreter;
use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data).map_err(|e| format!("Game load error: {}", e))?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    interpreter.set_debug(true);

    // Run with a limit to avoid infinite loop
    match interpreter.run_with_limit(Some(5000)) {
        Ok(()) => println!("Game completed normally"),
        Err(e) => println!("Game error: {}", e),
    }

    Ok(())
}