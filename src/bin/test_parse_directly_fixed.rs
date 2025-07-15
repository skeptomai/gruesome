use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let mut vm = VM::new(game);
    
    // Use the addresses we found
    let text_buffer = 0x01f5;
    let parse_buffer = 0x0225;
    
    println!("Using text buffer at {:04x}, parse buffer at {:04x}", text_buffer, parse_buffer);
    
    // Test parsing 'n'
    println!("\n=== Testing parse of 'n' ===");
    
    // First, let's check what the parse_text function expects
    // Text buffer format: max_length, actual_length, then text
    // We need to check what's already at these addresses
    
    println!("Current text buffer contents:");
    for i in 0..16 {
        print!("{:02x} ", vm.read_byte(text_buffer + i));
    }
    println!();
    
    // Set up text buffer with 'n'
    // Don't overwrite the max length (first byte)
    let max_len = vm.read_byte(text_buffer);
    println!("Text buffer max length: {}", max_len);
    
    vm.write_byte(text_buffer + 1, 1)?;  // Length = 1
    vm.write_byte(text_buffer + 2, b'n')?;
    
    println!("Set text buffer to contain 'n'");
    
    // Call parse_text
    match vm.parse_text(text_buffer, parse_buffer) {
        Ok(_) => println!("Parse succeeded"),
        Err(e) => println!("Parse error: {}", e),
    }
    
    // Read parse results
    let max_words = vm.read_byte(parse_buffer);
    let word_count = vm.read_byte(parse_buffer + 1);
    println!("\nParse buffer: max_words={}, found {} words", max_words, word_count);
    
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
            
            // Also check dictionary lookup
            let lookup_result = vm.lookup_dictionary("n");
            println!("Direct dictionary lookup of 'n': {:04x}", lookup_result);
        }
    }
    
    // Also test 'w' and 'e' for comparison
    for &ch in &[b'w', b'e'] {
        println!("\n=== Testing parse of '{}' ===", ch as char);
        
        vm.write_byte(text_buffer + 1, 1)?;
        vm.write_byte(text_buffer + 2, ch)?;
        
        match vm.parse_text(text_buffer, parse_buffer) {
            Ok(_) => {
                let word_count = vm.read_byte(parse_buffer + 1);
                if word_count > 0 {
                    let dict_addr = vm.read_word(parse_buffer + 2);
                    println!("Found in dictionary at: {:04x}", dict_addr);
                } else {
                    println!("Not found in dictionary");
                }
            }
            Err(e) => println!("Parse error: {}", e),
        }
    }
    
    Ok(())
}