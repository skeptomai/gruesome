use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let mut vm = VM::new(game);
    
    println!("=== Comparing Dictionary Entries ===\n");
    
    // Get dictionary structure
    let dict_base = vm.game.header.dictionary as u32;
    let sep_count = vm.read_byte(dict_base);
    let entry_start = dict_base + 1 + sep_count as u32;
    let entry_length = vm.read_byte(entry_start);
    let entry_count = vm.read_word(entry_start + 1);
    let entries_addr = entry_start + 3;
    
    println!("Dictionary structure:");
    println!("  Entry length: {} bytes", entry_length);
    println!("  Entry count: {}", entry_count);
    println!("  Entries start at: {:04x}", entries_addr);
    
    // Helper to show entry details
    let show_entry = |name: &str, entry_num: u32| {
        let addr = entries_addr + (entry_num * entry_length as u32);
        println!("\n{} (entry #{}):", name, entry_num + 1);
        println!("  Address: {:04x}", addr);
        
        // Read all bytes
        let mut bytes = Vec::new();
        for i in 0..entry_length {
            bytes.push(vm.read_byte(addr + i as u32));
        }
        
        // Show raw data
        print!("  Raw data: ");
        for b in &bytes {
            print!("{:02x} ", b);
        }
        println!();
        
        // Interpret the data
        println!("  Encoded word: {:02x}{:02x} {:02x}{:02x}", bytes[0], bytes[1], bytes[2], bytes[3]);
        println!("  Data byte 4: {:02x}", bytes[4]);
        println!("  Data bytes 5-6: {:02x} {:02x}", bytes[5], bytes[6]);
        
        // Interpret data bytes based on byte 4
        if bytes[4] == 0x13 {
            println!("  Type: Simple action (0x13)");
            let action = bytes[5];
            println!("  Action number: {} (0x{:02x})", action, action);
        } else if bytes[4] == 0x32 {
            println!("  Type: Complex/special (0x32)");
            let val1 = bytes[5];
            let val2 = bytes[6];
            println!("  Values: 0x{:02x} 0x{:02x}", val1, val2);
            
            // Check if it might be a packed address
            let packed = ((val1 as u16) << 8) | val2 as u16;
            println!("  As packed address: {:04x}", packed);
        } else {
            println!("  Type: Unknown (0x{:02x})", bytes[4]);
        }
    };
    
    // Show our key entries
    show_entry("'e'", 245);  // Approximate, let's find it
    show_entry("'go'", 258);
    show_entry("'n'", 430);  // Approximate
    show_entry("'w'", 662);
    show_entry("'west'", 672);
    
    // Let's find the exact entries by searching
    println!("\n=== Finding exact entries by lookup ===");
    
    let words = ["e", "east", "n", "north", "w", "west", "go"];
    for word in &words {
        let addr = vm.lookup_dictionary(word);
        if addr != 0 {
            // Calculate entry number
            let entry_num = (addr as u32 - entries_addr) / entry_length as u32;
            println!("\n'{}' found at {:04x} (entry #{})", word, addr, entry_num + 1);
            
            // Show the data bytes
            print!("  Data bytes 4-6: ");
            for i in 4..7 {
                print!("{:02x} ", vm.read_byte(addr as u32 + i));
            }
            println!();
        }
    }
    
    // Now let's trace what the parser actually returns
    println!("\n=== Testing parser output ===");
    
    let text_buffer = 0x01f5;
    let parse_buffer = 0x0225;
    
    for &ch in &[b'e', b'n', b'w'] {
        println!("\nParsing '{}':", ch as char);
        
        // Set up text buffer
        vm.write_byte(text_buffer + 1, 1)?;
        vm.write_byte(text_buffer + 2, ch)?;
        
        // Parse
        vm.parse_text(text_buffer, parse_buffer)?;
        
        // Check result
        let word_count = vm.read_byte(parse_buffer + 1);
        if word_count > 0 {
            let dict_addr = vm.read_word(parse_buffer + 2);
            println!("  Dictionary address: {:04x}", dict_addr);
            
            // Show what's at that address
            print!("  Data at address: ");
            for i in 0..7 {
                print!("{:02x} ", vm.read_byte(dict_addr as u32 + i));
            }
            println!();
            
            // Check byte 4 specifically
            let byte4 = vm.read_byte(dict_addr as u32 + 4);
            let byte5 = vm.read_byte(dict_addr as u32 + 5);
            let byte6 = vm.read_byte(dict_addr as u32 + 6);
            
            println!("  Byte 4 (type?): {:02x}", byte4);
            println!("  Bytes 5-6: {:02x} {:02x}", byte5, byte6);
            
            if byte4 == 0x32 {
                println!("  *** This entry has the special type 0x32! ***");
            }
        }
    }
    
    Ok(())
}