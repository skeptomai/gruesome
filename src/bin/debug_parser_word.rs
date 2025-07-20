use gruesome::interpreter::Interpreter;
use gruesome::vm::{Game, VM};
use log::info;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("gruesome=debug"))
        .format_timestamp(None)
        .init();

    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;

    // Let's check the parse buffer structure after parsing "move leaves"
    println!("=== Parse Buffer Structure ===");
    println!("When the game parses 'move leaves', it stores:");
    println!("- The dictionary addresses of recognized words");
    println!("- The length and position of each word");
    println!();

    // The parse buffer is at 0x2551 (from our trace)
    let parse_buffer = 0x2551;
    println!("Parse buffer at: 0x{parse_buffer:04x}");

    // Parse buffer format:
    // Byte 0: Max words
    // Byte 1: Word count
    // Then for each word (4 bytes):
    //   2 bytes: Dictionary address
    //   1 byte: Word length
    //   1 byte: Position in text buffer

    println!("\nAfter parsing 'move leaves':");
    println!("The word 'leaves' should be stored with:");
    println!("- Its dictionary address (if found)");
    println!("- Length: 6");
    println!("- Position in input");

    // The issue might be that the game is using the dictionary form
    // Let's also check what happens with text printing

    println!("\n=== Text Reconstruction ===");
    println!("When printing an error, the game might:");
    println!("1. Use the original typed text from text buffer");
    println!("2. Use the dictionary word");
    println!("3. Reconstruct from parse data");

    // Run the game to see what actually happens
    let vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);

    info!("Starting game to trace parser behavior...");
    if let Err(e) = interpreter.run() {
        eprintln!("Error: {e}");
    }

    Ok(())
}
