use gruesome::disasm_txd::TxdDisassembler;
use gruesome::vm::Game;
use log::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <game_file>", args[0]);
        std::process::exit(1);
    }

    let game_file = &args[1];

    // Run our disassembler
    let memory = std::fs::read(game_file)?;
    let game = Game::from_memory(memory)?;
    let mut disasm = TxdDisassembler::new(&game);

    disasm.discover_routines()?;
    let all_routines: Vec<u32> = disasm.get_routine_addresses();

    info!("Found {} routines initially", all_routines.len());

    // Test cases from filter-nested-routines output
    let test_cases = vec![
        (0x0d198, 0x0d184), // at offset +20
        (0x0d6f4, 0x0d6e8), // at offset +12
        (0x0e6f8, 0x0e6e8), // at offset +16
        (0x0e96c, 0x0e960), // at offset +12
        (0x25564, 0x25550), // at offset +20
        (0x2b3b4, 0x2b384), // at offset +48
    ];

    info!("\n=== ANALYZING NESTED ROUTINE DIFFERENCES ===");

    for (nested, parent) in test_cases {
        info!("\nChecking {:05x} nested in {:05x}:", nested, parent);

        // Get parent routine info
        let parent_locals = game.memory[parent as usize];
        let parent_header_size = 1 + if game.header.version <= 4 {
            (parent_locals as usize) * 2
        } else {
            0
        };

        let offset = nested - parent;
        info!("  Parent locals: {}", parent_locals);
        info!("  Parent header size: {} bytes", parent_header_size);
        info!("  Nested offset: {} bytes", offset);

        if offset < parent_header_size as u32 {
            info!("  -> Inside header/locals area!");
        } else {
            info!("  -> Inside code body (beyond header)");

            // Let's decode what's at the nested address
            let nested_locals = game.memory[nested as usize];
            info!("  Nested address has locals count: {}", nested_locals);

            // Check if it looks like a valid routine header
            if nested_locals <= 15 {
                info!("  -> Looks like a valid routine header (locals <= 15)");
            } else {
                info!("  -> Suspicious locals count (> 15)");
            }
        }
    }

    // Also check 0cafc which we know is an alternate entry
    info!("\n\nChecking known alternate entry 0cafc:");
    let parent = 0x0caf4;
    let nested = 0x0cafc;
    let parent_locals = game.memory[parent as usize];
    let parent_header_size = 1 + if game.header.version <= 4 {
        (parent_locals as usize) * 2
    } else {
        0
    };
    let offset = nested - parent;
    info!("  Parent {:05x} locals: {}", parent, parent_locals);
    info!("  Parent header size: {} bytes", parent_header_size);
    info!("  Nested {:05x} offset: {} bytes", nested, offset);
    info!(
        "  -> {}",
        if offset < parent_header_size as u32 {
            "Inside header/locals area!"
        } else {
            "Inside code body"
        }
    );

    Ok(())
}
