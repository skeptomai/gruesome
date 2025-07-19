use env_logger;
use gruesome::interpreter::Interpreter;
use gruesome::vm::{Game, VM};
use log::{debug, info};
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger to see debug output
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;
    let mut vm = VM::new(game);
    let mut interpreter = Interpreter::new(vm);

    // Run the game up to the "move leaves" command
    // This is a hacky way to get there but works for testing
    println!("Running game to 'move leaves' command...");

    // We need to get to the point where the text buffer contains "move leaves"
    // Let's check the text buffer address from the logs
    let text_buffer = 0x2641; // From the debug output

    // Show what's in the text buffer when "move leaves" is parsed
    println!("\n=== Text buffer contents at 0x{:04x} ===", text_buffer);

    // Simulate having "move leaves" in the buffer
    let text = "move leaves";
    interpreter.vm.write_byte(text_buffer, text.len() as u8)?;
    interpreter
        .vm
        .write_byte(text_buffer + 1, text.len() as u8)?;

    for (i, ch) in text.bytes().enumerate() {
        interpreter.vm.write_byte(text_buffer + 2 + i as u32, ch)?;
    }

    // Display the buffer
    print!("Text buffer: \"");
    for i in 0..text.len() {
        let ch = interpreter.vm.read_byte(text_buffer + 2 + i as u32);
        print!("{}", ch as char);
    }
    println!("\"");

    // Show positions
    println!("\nPositions (1-based):");
    for i in 0..text.len() {
        println!("  Position {}: '{}'", i + 1, text.chars().nth(i).unwrap());
    }

    println!("\nThe word 'leaves' starts at position 6 (1-based)");
    println!("But position 6 is the space character!");
    println!("\nThis suggests the parse buffer position might be off by one.");

    Ok(())
}
