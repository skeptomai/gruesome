use gruesome::interpreter::Interpreter;
use gruesome::vm::{Game, VM};
use log::{debug, info};
use std::env;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Get command line arguments
    let args: Vec<String> = env::args().collect();

    // Display help information if no game file provided
    // Exit with success status since user is requesting help, not encountering an error
    if args.len() < 2 {
        println!("gruesome - Z-Machine interpreter for Infocom text adventure games");
        println!();
        println!(
            "Usage: {} <game_file.dat> [--step start_pc end_pc]",
            args[0]
        );
        println!("Examples:");
        println!("  {} resources/test/zork1/DATA/ZORK1.DAT", args[0]);
        println!(
            "  {} resources/test/zork1/DATA/ZORK1.DAT --step 0x577c 0x5880",
            args[0]
        );
        println!();
        println!("The --step option enables single-step debugging for instructions");
        println!("in the specified PC range (hex values with or without 0x prefix)");
        return Ok(());
    }

    let game_path = &args[1];

    // Check for --step option
    let mut step_range = None;
    if args.len() >= 5 && args[2] == "--step" {
        let start = u32::from_str_radix(args[3].trim_start_matches("0x"), 16)
            .map_err(|_| format!("Invalid start PC: {}", args[3]))?;
        let end = u32::from_str_radix(args[4].trim_start_matches("0x"), 16)
            .map_err(|_| format!("Invalid end PC: {}", args[4]))?;
        step_range = Some((start, end));
        info!(
            "Single-stepping enabled for PC range 0x{:04x}-0x{:04x}",
            start, end
        );
    }

    // Load the game file with user-friendly error handling
    // Use explicit match instead of ? operator to provide clean, formatted error messages
    // that guide users to solve common problems like incorrect paths or wrong directories
    debug!("Loading Z-Machine game: {}", game_path);
    let mut file = match File::open(game_path) {
        Ok(file) => file,
        Err(e) => {
            match e.kind() {
                std::io::ErrorKind::NotFound => {
                    eprintln!("Error: Game file not found: {}", game_path);
                    eprintln!();
                    eprintln!("Please check:");
                    eprintln!("• File path is correct");
                    eprintln!("• You're running from the right directory");
                    eprintln!("• File exists and is readable");
                }
                std::io::ErrorKind::PermissionDenied => {
                    eprintln!(
                        "Error: Permission denied accessing game file: {}",
                        game_path
                    );
                    eprintln!();
                    eprintln!("Please check file permissions.");
                }
                _ => {
                    eprintln!("Error: Cannot open game file '{}': {}", game_path, e);
                }
            }
            std::process::exit(1);
        }
    };
    let mut game_data = Vec::new();
    if let Err(e) = file.read_to_end(&mut game_data) {
        eprintln!("Error: Cannot read game file '{}': {}", game_path, e);
        std::process::exit(1);
    }

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
    let result = match interpreter.run_with_limit(Some(1000000)) {
        Ok(()) => {
            debug!("\nGame ended normally.");
            Ok(())
        }
        Err(e) => {
            eprintln!("\nError during execution: {e}");
            Err(e)
        }
    };

    // Always clean up terminal state before exit
    interpreter.cleanup();

    // Return the result (will exit with error code if there was an error)
    result.map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>)
}
