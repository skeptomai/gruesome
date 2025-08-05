use std::env;
use gruesome::vm::Game;
use gruesome::disasm_txd::TxdDisassembler;
use log::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    // Test with both v3 and v4 games
    let v3_file = "resources/test/zork1/DATA/ZORK1.DAT";
    let v4_file = "resources/test/amfv/amfv-r79-s851122.z4";
    
    // Test v3 first to ensure no regression
    info!("Testing V3 game (Zork I)...");
    let memory = std::fs::read(v3_file)?;
    let game = Game::from_memory(memory)?;
    let mut disasm = TxdDisassembler::new(&game);
    
    let routines = disasm.discover_routines()?;
    info!("V3: Found {} routines (expected ~448)", routines.len());
    
    // Test v4
    info!("\nTesting V4 game (AMFV)...");
    let memory = std::fs::read(v4_file)?;
    let game = Game::from_memory(memory)?;
    let mut disasm = TxdDisassembler::new(&game);
    
    let routines = disasm.discover_routines()?;
    info!("V4: Found {} routines (target: 982, current: ~624)", routines.len());
    
    Ok(())
}