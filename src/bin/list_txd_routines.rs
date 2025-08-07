use gruesome::disasm_txd::TxdDisassembler;
use gruesome::vm::Game;
use std::env;
use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <z-code-file>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];

    // Load game file
    let mut file = File::open(filename)?;
    let mut memory = Vec::new();
    file.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;
    let mut disasm = TxdDisassembler::new(&game);

    // Run discovery
    disasm.discover_routines()?;

    // Output all routine addresses in hex
    let mut addrs: Vec<u32> = disasm.get_routine_addresses();
    addrs.sort();

    for addr in addrs {
        println!("{addr:x}");
    }

    Ok(())
}
