use std::env;
use std::fs::File;
use std::io::{self, prelude::*};
use std::path::PathBuf;

use infocom::game::GameFile;
use infocom::zmachine::ZMachine;
use infocom::zrand::ZRand;

fn main() -> io::Result<()> {
    env_logger::init();
    
    let args: Vec<String> = env::args().collect();
    
    let data_file = if args.len() > 1 {
        args[1].clone()
    } else {
        "resources/test/zork1/DATA/ZORK1.DAT".to_string()
    };
    
    println!("Loading Z-Machine game: {}", data_file);
    
    let mut path = PathBuf::from(&data_file);
    if !path.exists() {
        // Try relative to project root
        path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(&data_file);
    }
    
    if !path.exists() {
        eprintln!("Error: Game file not found: {}", data_file);
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
    
    println!("Game loaded successfully:");
    println!("  Version: {}", game.version());
    println!("  Initial PC: {:#06x}", game.header().initial_pc);
    println!("  Dictionary: {:#06x}", game.header().dictionary);
    println!("  Objects: {:#06x}", game.header().object_table_addr);
    println!();
    
    // Create and run Z-Machine
    let mut zmachine = ZMachine::new(&game);
    
    println!("Starting Z-Machine execution...");
    match zmachine.run() {
        Ok(()) => println!("Z-Machine execution completed."),
        Err(e) => eprintln!("Z-Machine execution error: {}", e),
    }
    
    Ok(())
}