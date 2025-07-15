use infocom::vm::{Game, VM};
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data)?;
    let mut vm = VM::new(game);
    
    println!("Testing dictionary lookup and parse buffer format\n");
    
    // Test dictionary lookup
    let test_words = ["quit", "look", "west", "q"];
    for word in &test_words {
        let addr = vm.lookup_dictionary(word);
        println!("Dictionary lookup '{}': 0x{:04x}", word, addr);
    }
    
    // Test text parsing
    println!("\nTesting parse_text with 'quit':");
    
    // Create test buffers
    let text_buffer = 0x2000;
    let parse_buffer = 0x2100;
    
    // Write text buffer header
    vm.write_byte(text_buffer, 80)?; // Max length
    vm.write_byte(text_buffer + 1, 4)?; // Length of "quit"
    vm.write_byte(text_buffer + 2, b'q')?;
    vm.write_byte(text_buffer + 3, b'u')?;
    vm.write_byte(text_buffer + 4, b'i')?;
    vm.write_byte(text_buffer + 5, b't')?;
    
    // Write parse buffer header
    vm.write_byte(parse_buffer, 10)?; // Max 10 words
    
    // Parse the text
    vm.parse_text(text_buffer, parse_buffer)?;
    
    // Read results
    let word_count = vm.read_byte(parse_buffer + 1);
    println!("Word count: {}", word_count);
    
    for i in 0..word_count {
        let offset = parse_buffer + 2 + (i as u32 * 4);
        let dict_addr = vm.read_word(offset);
        let word_len = vm.read_byte(offset + 2);
        let text_pos = vm.read_byte(offset + 3);
        
        println!("Word {}: dict_addr=0x{:04x}, len={}, pos={}", 
                i, dict_addr, word_len, text_pos);
    }
    
    Ok(())
}