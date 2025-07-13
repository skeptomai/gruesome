use std::env;
use std::process::exit;
use std::fs::File;
use std::io::{self, prelude::*};
use std::path::PathBuf;
use infocom::game::GameFile;
use infocom::zmachine::ZMachine;
use infocom::zrand::ZRand;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    
    env_logger::init();
    if args.len() < 2 {
        eprintln!("Gotta supply a game file by path");
        exit(-1);
    };

    let path = PathBuf::from(&args[1].clone());

    if !path.exists() {
        eprintln!("Error: Game file not found: {:?}", path.to_str());
        return Ok(());
    }
    
    // Load game file
    let mut f = File::open(&path)?;
    let mut all_bytes = Vec::new();
    f.read_to_end(&mut all_bytes)?;
    
    // Create random generator
    let mut zrg = ZRand::new_uniform();
    
    // Create game file structure
    let game = GameFile::new(&all_bytes, &mut zrg);
    // Create and run Z-Machine
    let mut zmachine = ZMachine::new(&game);
    
    println!("Z-Machine Interpreter");
    println!("Loading game: {:?}", path);
    println!("Game loaded successfully:");
    println!("  Version: {}", game.version());
    println!("  Initial PC: {:#06x}", game.header().initial_pc);
    
    // Run the game normally
    match zmachine.run_interactive() {
        Ok(()) => {
            println!("\nGame ended. Thanks for playing!");
        }
        Err(e) => {
            eprintln!("Z-Machine execution error: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}
