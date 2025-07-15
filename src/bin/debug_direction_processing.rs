use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("=== Analyzing direction word dictionary entries ===\n");
    
    // The dictionary data bytes we found:
    let directions = [
        ("n", 0x45e7, vec![0x4c, 0xa5, 0x94, 0xa5, 0x13, 0x1f, 0x00]),
        ("e", 0x3fea, vec![0x28, 0xa5, 0x94, 0xa5, 0x13, 0x1e, 0x00]),
        ("w", 0x4d42, vec![0x70, 0xa5, 0x94, 0xa5, 0x32, 0xa1, 0x1d]),
    ];
    
    for (dir, addr, bytes) in &directions {
        println!("Direction '{}' at {:04x}:", dir, addr);
        println!("  Raw bytes: {:02x?}", bytes);
        
        // Dictionary entry format for Z3:
        // - 4 bytes: encoded word (2 words)
        // - 3 bytes: data (varies by game)
        
        // The data bytes seem to be:
        // byte 4: some kind of type or category
        // bytes 5-6: action/verb code or pointer
        
        let byte4 = bytes[4];
        let data_word = (bytes[5] as u16) | ((bytes[6] as u16) << 8);
        
        println!("  Data byte 4: {:02x}", byte4);
        println!("  Data word (bytes 5-6): {:04x}", data_word);
        
        // Check if data_word might be an address
        if data_word > 0 && data_word < 0x2000 {
            println!("  Might be an action/verb number");
        } else if data_word >= 0x4000 && data_word < vm.game.memory.len() as u16 {
            println!("  Might be a routine address");
            // Try to decode instruction at that address
            let byte = vm.read_byte(data_word as u32);
            println!("    First byte at {:04x}: {:02x}", data_word, byte);
        }
        println!();
    }
    
    // Now let's look at what V-WALK might expect
    println!("\n=== Checking V-WALK routine at 0x6f76 ===");
    
    // Disassemble first few instructions of V-WALK
    let vwalk_addr = 0x6f76;
    println!("First 16 bytes of V-WALK:");
    for i in 0..16 {
        let byte = vm.read_byte(vwalk_addr + i);
        print!("{:02x} ", byte);
        if i % 8 == 7 { println!(); }
    }
    println!();
    
    // Check PERFORM routine similarly
    println!("\n=== Checking PERFORM routine at 0x50a8 ===");
    let perform_addr = 0x50a8;
    println!("First 16 bytes of PERFORM:");
    for i in 0..16 {
        let byte = vm.read_byte(perform_addr + i);
        print!("{:02x} ", byte);
        if i % 8 == 7 { println!(); }
    }
    println!();
    
    // Let's also check what the garbage text addresses might be
    println!("\n=== Investigating potential text locations ===");
    
    // The user reported seeing "w can you attack a spirit with material objects?"
    // and "hsDvvmh h" - these look like text being read from wrong addresses
    
    // Search for "can you attack" text
    let search_text = "can you attack";
    println!("Searching for '{}' in game memory...", search_text);
    
    for addr in 0x4000..0x14000 {
        let mut matches = true;
        for (i, ch) in search_text.bytes().enumerate() {
            if vm.game.memory.get(addr + i) != Some(&ch) {
                matches = false;
                break;
            }
        }
        if matches {
            println!("Found at {:04x}!", addr);
            // Show context
            print!("  Context: ");
            for i in 0..40 {
                if let Some(&byte) = vm.game.memory.get(addr + i) {
                    if byte >= 32 && byte <= 126 {
                        print!("{}", byte as char);
                    } else {
                        print!(".");
                    }
                }
            }
            println!();
        }
    }
    
    Ok(())
}