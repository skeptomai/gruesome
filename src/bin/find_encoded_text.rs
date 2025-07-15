use infocom::vm::{Game, VM};
use infocom::text;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("=== Looking for text that might be displayed ===\n");
    
    // The garbage "w can you attack a spirit with material objects?"
    // might be Z-encoded text being decoded from wrong address
    
    // Let's check what happens if we interpret the direction data as text addresses
    println!("Checking if direction data points to Z-encoded text:\n");
    
    let directions = [
        ("n", 0x45e7, vec![0x13, 0x1f, 0x00]),
        ("e", 0x3fea, vec![0x13, 0x1e, 0x00]),
        ("w", 0x4d42, vec![0x32, 0xa1, 0x1d]),
    ];
    
    for (name, dict_addr, data_bytes) in &directions {
        println!("Direction '{}' (dict addr {:04x}):", name, dict_addr);
        println!("  Data bytes: {:02x?}", data_bytes);
        
        // What if byte 4 (0x32 for 'w') affects how the rest is interpreted?
        let byte4 = data_bytes[0];
        
        // Try different interpretations of bytes 5-6
        let interp1 = ((data_bytes[1] as u32) << 8) | data_bytes[2] as u32;
        let interp2 = ((data_bytes[2] as u32) << 8) | data_bytes[1] as u32;
        let interp3 = data_bytes[1] as u32;
        let interp4 = data_bytes[2] as u32;
        
        println!("  Possible address interpretations:");
        println!("    As big-endian word: {:04x}", interp1);
        println!("    As little-endian word: {:04x}", interp2);
        println!("    Just byte 5: {:02x}", interp3);
        println!("    Just byte 6: {:02x}", interp4);
        
        // For 'w', let's specifically check 0xa11d
        if *name == "w" {
            println!("\n  Special check for 'w' - trying to decode text at various addresses:");
            
            // Try decoding from 0xa11d
            let addr = 0xa11d;
            println!("    At {:04x}:", addr);
            if addr < vm.game.memory.len() {
                // Try to decode as Z-string
                match text::decode_string(&vm.game.memory[addr..], vm.game.header.abbrev_table, vm.game.header.version as usize) {
                    Ok((text, _)) => {
                        println!("      Decoded: '{}'", text);
                        if text.contains("can you attack") {
                            println!("      *** FOUND THE GARBAGE TEXT! ***");
                        }
                    }
                    Err(e) => println!("      Decode error: {}", e),
                }
            }
            
            // Also try 0x1da1 (reversed)
            let addr2 = 0x1da1;
            println!("    At {:04x} (reversed):", addr2);
            if addr2 < vm.game.memory.len() {
                match text::decode_string(&vm.game.memory[addr2..], vm.game.header.abbrev_table, vm.game.header.version as usize) {
                    Ok((text, _)) => println!("      Decoded: '{}'", text),
                    Err(e) => println!("      Decode error: {}", e),
                }
            }
        }
        
        println!();
    }
    
    // Let's also check what V-WALK might do with these values
    println!("\n=== Checking action numbers vs. text addresses ===");
    
    // In Z-machine, actions are usually small numbers
    // The values 0x1f (31) and 0x1e (30) for n/e look like action numbers
    // But 0xa11d for 'w' is way too large
    
    // What if there's a bug in how we handle the dictionary data?
    println!("\nLet's check if 'w' dictionary entry is corrupted:");
    
    // Re-read the 'w' entry byte by byte
    let w_addr = 0x4d42;
    println!("Raw memory at 'w' dictionary entry ({:04x}):", w_addr);
    for i in 0..10 {
        let byte = vm.read_byte(w_addr as u32 + i);
        print!("{:02x} ", byte);
    }
    println!();
    
    // Check the entry before and after
    println!("\nEntry before 'w' (at {:04x}):", w_addr - 7);
    for i in 0..7 {
        let byte = vm.read_byte(w_addr as u32 - 7 + i);
        print!("{:02x} ", byte);
    }
    println!();
    
    println!("\nEntry after 'w' (at {:04x}):", w_addr + 7);
    for i in 0..7 {
        let byte = vm.read_byte(w_addr as u32 + 7 + i);
        print!("{:02x} ", byte);
    }
    println!();
    
    Ok(())
}