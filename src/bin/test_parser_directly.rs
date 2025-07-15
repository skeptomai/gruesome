use infocom::vm::{Game, VM};
use log::{debug, info};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let mut vm = VM::new(game);
    
    // Set up text buffer and parse buffer as if SREAD had run
    let text_buffer = 0x5635;
    let parse_buffer = 0x5665;
    
    // Test parsing 'n'
    println!("\n=== Testing parse of 'n' ===");
    vm.write_byte(text_buffer, 50)?;     // Max length
    vm.write_byte(text_buffer + 1, 1)?;  // Length = 1
    vm.write_byte(text_buffer + 2, b'n')?;
    
    // Call parse_text directly
    vm.parse_text(text_buffer, parse_buffer)?;
    
    // Read parse results
    let word_count = vm.read_byte(parse_buffer + 1);
    println!("Parse found {} words", word_count);
    
    if word_count > 0 {
        // Parse buffer format: max_words, word_count, then entries
        // Each entry is: dict_addr (2 bytes), length (1 byte), text_pos (1 byte)
        let entry_addr = parse_buffer + 2;
        let dict_addr = vm.read_word(entry_addr);
        let word_len = vm.read_byte(entry_addr + 2);
        let text_pos = vm.read_byte(entry_addr + 3);
        
        println!("Word 1: dict_addr={:04x}, len={}, pos={}", dict_addr, word_len, text_pos);
        
        // Check what's at the dictionary address
        if dict_addr != 0 {
            println!("Dictionary entry bytes:");
            for i in 0..7 {
                print!(" {:02x}", vm.read_byte(dict_addr as u32 + i));
            }
            println!();
        }
    }
    
    // Test parsing 'w'
    println!("\n=== Testing parse of 'w' ===");
    vm.write_byte(text_buffer, 50)?;     // Max length
    vm.write_byte(text_buffer + 1, 1)?;  // Length = 1
    vm.write_byte(text_buffer + 2, b'w')?;
    
    vm.parse_text(text_buffer, parse_buffer)?;
    
    let word_count = vm.read_byte(parse_buffer + 1);
    println!("Parse found {} words", word_count);
    
    if word_count > 0 {
        let entry_addr = parse_buffer + 2;
        let dict_addr = vm.read_word(entry_addr);
        let word_len = vm.read_byte(entry_addr + 2);
        let text_pos = vm.read_byte(entry_addr + 3);
        
        println!("Word 1: dict_addr={:04x}, len={}, pos={}", dict_addr, word_len, text_pos);
        
        if dict_addr != 0 {
            println!("Dictionary entry bytes:");
            for i in 0..7 {
                print!(" {:02x}", vm.read_byte(dict_addr as u32 + i));
            }
            println!();
        }
    }
    
    // Test parsing 'east'
    println!("\n=== Testing parse of 'east' ===");
    vm.write_byte(text_buffer, 50)?;     // Max length
    vm.write_byte(text_buffer + 1, 4)?;  // Length = 4
    vm.write_byte(text_buffer + 2, b'e')?;
    vm.write_byte(text_buffer + 3, b'a')?;
    vm.write_byte(text_buffer + 4, b's')?;
    vm.write_byte(text_buffer + 5, b't')?;
    
    vm.parse_text(text_buffer, parse_buffer)?;
    
    let word_count = vm.read_byte(parse_buffer + 1);
    println!("Parse found {} words", word_count);
    
    if word_count > 0 {
        let entry_addr = parse_buffer + 2;
        let dict_addr = vm.read_word(entry_addr);
        println!("Word 1: dict_addr={:04x}", dict_addr);
    }
    
    Ok(())
}