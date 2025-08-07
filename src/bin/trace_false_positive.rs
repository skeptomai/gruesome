use gruesome::disasm_txd::TxdDisassembler;
use gruesome::vm::Game;
use log::info;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <game.z4> <address>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    let address = u32::from_str_radix(&args[2], 16)?;

    let memory = std::fs::read(filename)?;
    let game = Game::from_memory(memory)?;
    let mut disasm = TxdDisassembler::new(&game);

    info!("Tracing validation of address {:04x}", address);

    // Check if it's in our discovered routines
    let _ = disasm.discover_routines();
    let routines = disasm.get_routine_addresses();
    let is_found = routines.contains(&address);

    info!("Result: is_found = {}", is_found);

    Ok(())
}
