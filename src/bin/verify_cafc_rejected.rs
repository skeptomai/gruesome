use gruesome::disasm_txd::TxdDisassembler;
use gruesome::vm::Game;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <game_file>", args[0]);
        std::process::exit(1);
    }

    let game_file = &args[1];

    let memory = std::fs::read(game_file)?;
    let game = Game::from_memory(memory)?;
    let mut disasm = TxdDisassembler::new(&game);

    disasm.discover_routines()?;
    let routines = disasm.get_routine_addresses();

    println!("Total routines found: {}", routines.len());

    // Check for specific addresses
    let check_addrs = vec![0xcaf4, 0xcafc];
    for addr in check_addrs {
        let found = routines.contains(&addr);
        println!(
            "Address {:05x}: {}",
            addr,
            if found { "FOUND" } else { "NOT FOUND" }
        );
    }

    // Also check the count difference if cafc is rejected
    let has_caf4 = routines.contains(&0xcaf4);
    let has_cafc = routines.contains(&0xcafc);

    if has_caf4 && !has_cafc {
        println!("\n✓ SUCCESS: caf4 found but cafc correctly rejected!");
    } else if has_caf4 && has_cafc {
        println!("\n✗ FAILURE: Both caf4 and cafc found - rejection not working");
    }

    Ok(())
}
