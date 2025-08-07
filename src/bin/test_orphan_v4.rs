use gruesome::disasm_txd::TxdDisassembler;
use gruesome::vm::Game;
use log::info;
use std::collections::HashSet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let v4_file = "resources/test/amfv/amfv-r79-s851122.z4";

    // First test without orphan detection
    info!("Testing V4 without orphan detection...");
    let memory = std::fs::read(v4_file)?;
    let game = Game::from_memory(memory.clone())?;
    let mut disasm = TxdDisassembler::new(&game);

    disasm.discover_routines()?;
    let routines_without: HashSet<u32> = disasm.get_routine_addresses().into_iter().collect();
    info!(
        "Without orphan detection: {} routines",
        routines_without.len()
    );

    // Now test with orphan detection
    info!("\nTesting V4 with orphan detection...");
    let game2 = Game::from_memory(memory)?;
    let mut disasm2 = TxdDisassembler::new(&game2);
    disasm2.enable_orphan_detection();

    disasm2.discover_routines()?;
    let routines_with: HashSet<u32> = disasm2.get_routine_addresses().into_iter().collect();
    info!("With orphan detection: {} routines", routines_with.len());

    // Compare the differences
    let removed: Vec<u32> = routines_without
        .difference(&routines_with)
        .cloned()
        .collect();
    info!("\nRemoved by orphan detection: {} addresses", removed.len());
    if removed.len() <= 50 {
        for addr in &removed {
            info!("  - {:04x}", addr);
        }
    }

    // Check our known false positives
    let known_false_positives = vec![0xcaf8, 0xcafc, 0x33c04];
    for &addr in &known_false_positives {
        if removed.contains(&addr) {
            info!("✓ Correctly removed false positive: {:04x}", addr);
        } else if routines_with.contains(&addr) {
            info!("✗ Still includes false positive: {:04x}", addr);
        }
    }

    Ok(())
}
