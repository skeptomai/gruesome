use gruesome::disassembler::disasm_txd::{OutputOptions, TxdDisassembler};
use gruesome::interpreter::core::vm::Game;
use log::debug;
use std::env;
use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let args: Vec<String> = env::args().collect();

    // Parse command line options
    let mut show_addresses = false;
    let mut dump_hex = false;
    let mut show_filter_rules = false;
    let mut filename = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-n" => show_addresses = true,
            "-d" => dump_hex = true,
            "--show-filter-rules" => show_filter_rules = true,
            "-h" | "--help" => {
                eprintln!("Usage: {} [options] <game-file>", args[0]);
                eprintln!("\nOptions:");
                eprintln!("  -n                   Use addresses instead of labels");
                eprintln!("  -d                   Dump hex bytes of instructions");
                eprintln!("  --show-filter-rules  Show which filtering rules each routine passed");
                eprintln!("  -h                   Show this help message");
                std::process::exit(0);
            }
            arg if !arg.starts_with('-') => {
                filename = Some(arg.to_string());
                break;
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    let filename = filename.unwrap_or_else(|| {
        eprintln!("Usage: {} [options] <game-file>", args[0]);
        eprintln!("Try '{} -h' for help", args[0]);
        std::process::exit(1);
    });

    // Load game file
    let mut file = File::open(&filename)?;
    let mut memory = Vec::new();
    file.read_to_end(&mut memory)?;

    debug!("Loaded {} bytes from {}", memory.len(), filename);

    // Create game and disassembler
    let game = Game::from_memory(memory)?;
    let mut disasm = TxdDisassembler::new(&game);

    // Set output options
    let options = OutputOptions {
        show_addresses,
        dump_hex,
        show_filter_rules,
    };
    disasm.set_output_options(options);

    debug!(
        "Created TXD disassembler for version {} game",
        game.header.version
    );
    debug!(
        "Output options: addresses={}, hex={}, show_filter_rules={}",
        show_addresses, dump_hex, show_filter_rules
    );

    // Run discovery process
    disasm.discover_routines()?;

    // Generate and print output
    let output = disasm.generate_output();
    println!("{output}");

    Ok(())
}
