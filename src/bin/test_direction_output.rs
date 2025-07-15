use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("=== Examining dictionary data for directions ===\n");
    
    // Look at the actual dictionary entries more carefully
    let directions = [
        ("n", 0x45e7),
        ("north", 0x461f),
        ("e", 0x3fea),
        ("east", 0x3ff1),
        ("w", 0x4d42),
        ("west", 0x4d88),
    ];
    
    for (name, addr) in &directions {
        println!("'{}' at {:04x}:", name, addr);
        
        // Read all 7 bytes
        let mut bytes = Vec::new();
        for i in 0..7 {
            bytes.push(vm.read_byte(addr + i));
        }
        
        println!("  Full entry: {:02x?}", bytes);
        
        // Decode the text part (first 4 bytes)
        let word1 = ((bytes[0] as u16) << 8) | bytes[1] as u16;
        let word2 = ((bytes[2] as u16) << 8) | bytes[3] as u16;
        println!("  Encoded text: {:04x} {:04x}", word1, word2);
        
        // The data bytes
        println!("  Data bytes: {:02x} {:02x} {:02x}", bytes[4], bytes[5], bytes[6]);
        
        // Let's check if bytes[5] and bytes[6] might point to text
        let addr1 = bytes[5] as u16;
        let addr2 = bytes[6] as u16; 
        let combined = ((bytes[5] as u16) << 8) | bytes[6] as u16;
        
        println!("  Byte 5 as address: {:02x}", addr1);
        println!("  Byte 6 as address: {:02x}", addr2);
        println!("  Bytes 5-6 combined: {:04x}", combined);
        
        // For 'w', we see 0xa1 0x1d - let's check what's at address 0xa11d
        if combined > 0x1000 && combined < vm.game.memory.len() as u16 {
            println!("  Checking address {:04x}:", combined);
            print!("    First 20 bytes: ");
            for i in 0..20 {
                let byte = vm.read_byte(combined as u32 + i);
                if byte >= 32 && byte <= 126 {
                    print!("{}", byte as char);
                } else {
                    print!(".");
                }
            }
            println!();
        }
    }
    
    // Let's also search for the garbage text the user reported
    println!("\n=== Searching for garbage text ===");
    
    // Search for "can you attack" which appeared after 'w'
    let search_patterns = [
        "can you attack",
        "spirit with material",
        "hsDvvmh",
    ];
    
    for pattern in &search_patterns {
        println!("\nSearching for '{}'...", pattern);
        
        // Simple byte search
        let pattern_bytes = pattern.as_bytes();
        for addr in 0x1000..0x10000 {
            let mut found = true;
            for (i, &byte) in pattern_bytes.iter().enumerate() {
                if addr + i >= vm.game.memory.len() || vm.game.memory[addr + i] != byte {
                    found = false;
                    break;
                }
            }
            
            if found {
                println!("  Found at {:04x}!", addr);
                // Show more context
                print!("  Context: ");
                for i in 0..60 {
                    if addr + i < vm.game.memory.len() {
                        let byte = vm.game.memory[addr + i];
                        if byte >= 32 && byte <= 126 {
                            print!("{}", byte as char);
                        } else {
                            print!(".");
                        }
                    }
                }
                println!();
                break;
            }
        }
    }
    
    Ok(())
}