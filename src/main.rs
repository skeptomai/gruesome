use infocom::interpreter::Interpreter;
use infocom::vm::{Game, VM};
use log::{info, debug};
use std::env;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Get command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <game_file.dat>", args[0]);
        eprintln!("Example: {} resources/test/zork1/DATA/ZORK1.DAT", args[0]);
        std::process::exit(1);
    }

    let game_path = &args[1];

    // Load the game file
    debug!("Loading Z-Machine game: {}", game_path);
    let mut file = File::open(game_path)?;
    let mut game_data = Vec::new();
    file.read_to_end(&mut game_data)?;

    // Create the game and VM
    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);

    debug!("Z-Machine Interpreter v0.1.0");
    debug!("Game version: {}", interpreter.vm.game.header.version);
    info!("Initial PC: {:04x}", interpreter.vm.game.header.initial_pc);
    debug!("Starting game...\n");

    // Run the interpreter with a limit to avoid crashes
    match interpreter.run_with_limit(Some(500000)) {
        Ok(()) => {
            debug!("\nGame ended normally.");
        }
        Err(e) => {
            eprintln!("\nError during execution: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
