use gruesome::disasm_txd::TxdDisassembler;
use gruesome::vm::Game;
use log::debug;
use std::env;
use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <game-file>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];

    // Load game file
    let mut file = File::open(filename)?;
    let mut memory = Vec::new();
    file.read_to_end(&mut memory)?;

    debug!("Loaded {} bytes from {}", memory.len(), filename);

    // Create game and disassembler
    let game = Game::from_memory(memory)?;
    let mut disasm = TxdDisassembler::new(&game);

    debug!(
        "Created TXD disassembler for version {} game",
        game.header.version
    );

    // Run discovery process
    disasm.discover_routines()?;

    // Generate and print output
    let output = disasm.generate_output();
    println!("{}", output);

    Ok(())
}
