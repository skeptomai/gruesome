use gruesome::disassembler::Disassembler;
use gruesome::vm::Game;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from("resources/test/amfv/amfv-r79-s851122.z4");

    // Load the game file
    let mut f = File::open(path)?;
    let mut all_bytes = Vec::new();
    f.read_to_end(&mut all_bytes)?;

    // Try to find strings like "Time:" in the game
    // This is a crude search but might help locate the status line code
    let search_bytes = b"Time:";
    for (i, window) in all_bytes.windows(search_bytes.len()).enumerate() {
        if window == search_bytes {
            println!("Found 'Time:' at offset 0x{:04x}", i);
        }
    }

    // Create game and disassembler
    let game = Game::from_memory(all_bytes)?;
    let disasm = Disassembler::new(&game);

    // Look for routines that might be drawing the status line
    // In v4 games, these often involve:
    // - set_window
    // - set_cursor
    // - print/print_char operations
    // - "Time:" string

    println!("=== AMFV Disassembly - Looking for Status Line Code ===\n");

    // First, let's find the main routine
    println!("Main routine at 0x{:04x}", game.header.initial_pc);

    // Disassemble the first part of the main routine
    match disasm.disassemble_range(
        game.header.initial_pc as u32,
        game.header.initial_pc as u32 + 0x100,
    ) {
        Ok(output) => println!("\n{}", output),
        Err(e) => println!("Error: {}", e),
    }

    Ok(())
}
