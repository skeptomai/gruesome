use infocom::vm::Game;
use log::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    
    info!("Memory regions:");
    info!("  Dynamic memory: 0x0000 - 0x{:04x}", game.header.base_static_mem - 1);
    info!("  Static memory: 0x{:04x} - 0x{:04x}", game.header.base_static_mem, game.header.base_high_mem - 1);
    info!("  High memory: 0x{:04x} - end", game.header.base_high_mem);
    info!("  Globals start: 0x{:04x}", game.header.global_variables);
    
    // Check the text buffer address
    let text_buffer = 0x296b;
    info!("\nText buffer at 0x{:04x}:", text_buffer);
    if text_buffer < game.header.base_static_mem {
        info!("  ✓ In dynamic memory (writable)");
    } else {
        info!("  ✗ In static/high memory (read-only!)");
    }
    
    // Look at what's there
    info!("\nBuffer contents (first 80 bytes):");
    for i in 0..80 {
        if i % 16 == 0 {
            print!("  {:04x}: ", text_buffer + i);
        }
        print!("{:02x} ", game.memory[text_buffer + i]);
        if i % 16 == 15 {
            println!();
        }
    }
    println!();
    
    // Find a good parse buffer location
    // It should be after the text buffer
    let parse_buffer = text_buffer + 80; // After text buffer
    info!("\nSuggested parse buffer at 0x{:04x}:", parse_buffer);
    if parse_buffer < game.header.base_static_mem {
        info!("  ✓ In dynamic memory (writable)");
    } else {
        info!("  ✗ In static/high memory (read-only!)");
    }
    
    Ok(())
}