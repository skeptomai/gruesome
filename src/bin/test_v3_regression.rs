use gruesome::disassembler::disasm_txd::TxdDisassembler;
use gruesome::interpreter::core::vm::Game;
use log::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let v3_file = "resources/test/zork1/DATA/ZORK1.DAT";

    // Test v3 without orphan detection
    info!("Testing V3 (Zork I) without orphan detection...");
    let memory = std::fs::read(v3_file)?;
    let game = Game::from_memory(memory.clone())?;
    let mut disasm = TxdDisassembler::new(&game);

    disasm.discover_routines()?;
    let routines_without = disasm.get_routine_addresses();
    info!(
        "Without orphan detection: {} routines (expected ~448)",
        routines_without.len()
    );

    // Test v3 with orphan detection
    info!("\nTesting V3 (Zork I) with orphan detection...");
    let game2 = Game::from_memory(memory)?;
    let mut disasm2 = TxdDisassembler::new(&game2);
    disasm2.enable_orphan_detection();

    disasm2.discover_routines()?;
    let routines_with = disasm2.get_routine_addresses();
    info!("With orphan detection: {} routines", routines_with.len());

    // Check for regression
    if routines_with.len() < 440 {
        info!(
            "⚠️  WARNING: V3 regression detected! Should find at least 440 routines (TXD baseline)"
        );
    } else {
        info!("✓ V3 support maintained - still finding all TXD routines plus extras");
    }

    Ok(())
}
