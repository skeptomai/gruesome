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
    if args.len() < 2 {
        eprintln!("Usage: {} <game_file.dat> [--step start_pc end_pc]", args[0]);
        eprintln!("Examples:");
        eprintln!("  {} resources/test/zork1/DATA/ZORK1.DAT", args[0]);
        eprintln!("  {} resources/test/zork1/DATA/ZORK1.DAT --step 0x577c 0x5880", args[0]);
        eprintln!();
        eprintln!("The --step option enables single-step debugging for instructions");
        eprintln!("in the specified PC range (hex values with or without 0x prefix)");
        std::process::exit(1);
    }

    let game_path = &args[1];
    
    // Check for --step option
    let mut step_range = None;
    if args.len() >= 5 && args[2] == "--step" {
        let start = u32::from_str_radix(&args[3].trim_start_matches("0x"), 16)
            .unwrap_or_else(|_| {
                eprintln!("Invalid start PC: {}", args[3]);
                std::process::exit(1);
            });
        let end = u32::from_str_radix(&args[4].trim_start_matches("0x"), 16)
            .unwrap_or_else(|_| {
                eprintln!("Invalid end PC: {}", args[4]);
                std::process::exit(1);
            });
        step_range = Some((start, end));
        info!("Single-stepping enabled for PC range 0x{:04x}-0x{:04x}", start, end);
    }

    // Load the game file
    debug!("Loading Z-Machine game: {}", game_path);
    let mut file = File::open(game_path)?;
    let mut game_data = Vec::new();
    file.read_to_end(&mut game_data)?;

    // Create the game and VM
    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);
    
    // Enable single-stepping if requested
    if let Some((start, end)) = step_range {
        interpreter.enable_single_step(start, end);
    }

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
