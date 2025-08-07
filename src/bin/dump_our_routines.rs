use gruesome::disasm_txd::TxdDisassembler;
use gruesome::vm::Game;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <game_file> <output_file>", args[0]);
        std::process::exit(1);
    }

    let game_file = &args[1];
    let output_file = &args[2];

    let memory = fs::read(game_file)?;
    let game = Game::from_memory(memory)?;
    let mut disasm = TxdDisassembler::new(&game);

    disasm.discover_routines()?;
    let mut routines = disasm.get_routine_addresses();
    routines.sort();

    let mut output = String::new();
    for &addr in &routines {
        output.push_str(&format!("{:x}\n", addr));
    }

    fs::write(output_file, output)?;
    eprintln!("Wrote {} routines to {}", routines.len(), output_file);

    Ok(())
}
