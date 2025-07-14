use infocom::vm::{Game, VM};
use std::env;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Get command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <game_file.dat> <object_number>", args[0]);
        eprintln!("Example: {} resources/test/zork1/DATA/ZORK1.DAT 180", args[0]);
        std::process::exit(1);
    }

    let game_path = &args[1];
    let obj_num: usize = args[2].parse().expect("Object number must be a valid integer");

    // Load the game file
    println!("Loading Z-Machine game: {}", game_path);
    let mut file = File::open(game_path)?;
    let mut game_data = Vec::new();
    file.read_to_end(&mut game_data)?;

    // Create the game and VM
    let game = Game::from_memory(game_data)?;
    let vm = VM::new(game);

    println!("=== Z-Machine Object Table Debug ===");
    println!("Game version: {}", vm.game.header.version);
    println!("Object table starts at: 0x{:04x}", vm.game.header.object_table_addr);
    
    // Calculate object addresses manually
    let obj_table_addr = vm.game.header.object_table_addr as usize;
    let property_defaults = obj_table_addr;
    let obj_tree_base = property_defaults + 31 * 2; // 31 default properties, 2 bytes each
    let obj_offset = obj_tree_base + (obj_num - 1) * 9; // Each object is 9 bytes in v1-3
    
    println!("Property defaults start: 0x{:04x}", property_defaults);
    println!("Object tree starts: 0x{:04x}", obj_tree_base);
    println!("Object {} offset: 0x{:04x}", obj_num, obj_offset);
    
    // Also dump the raw bytes for the object
    println!("\n=== Raw Object {} Bytes (at offset 0x{:04x}) ===", obj_num, obj_offset);
    let obj_bytes = &vm.game.memory[obj_offset..obj_offset + 9];
    
    println!("Attribute bytes: [{:02x} {:02x} {:02x} {:02x}]", 
             obj_bytes[0], obj_bytes[1], obj_bytes[2], obj_bytes[3]);
    println!("Parent: {}, Sibling: {}, Child: {}", obj_bytes[4], obj_bytes[5], obj_bytes[6]);
    println!("Property offset: 0x{:04x}", u16::from_be_bytes([obj_bytes[7], obj_bytes[8]]));
    
    // Analyze attribute bits in detail
    println!("\n=== Attribute Bit Analysis ===");
    for (byte_idx, &byte) in obj_bytes[0..4].iter().enumerate() {
        for bit_idx in 0..8 {
            let attr_num = byte_idx * 8 + bit_idx;
            let is_set = (byte & (0x80 >> bit_idx)) != 0;
            if is_set {
                println!("Attribute {} is SET (byte {}, bit {})", attr_num, byte_idx, bit_idx);
            }
        }
    }
    
    // Check specifically for attribute 3 (ONBIT)
    let attr_3_byte = obj_bytes[0]; // Attribute 3 is in the first byte
    let attr_3_bit = 0x80 >> 3; // Bit position 3 (counting from left)
    let has_onbit = (attr_3_byte & attr_3_bit) != 0;
    
    println!("\n=== ONBIT (Attribute 3) Check ===");
    println!("Attribute byte 0: 0b{:08b} (0x{:02x})", attr_3_byte, attr_3_byte);
    println!("Attribute 3 bit mask: 0b{:08b} (0x{:02x})", attr_3_bit, attr_3_bit);
    println!("Has ONBIT (attribute 3): {}", has_onbit);
    
    // Test the VM's test_attribute function
    println!("\n=== VM test_attribute Results ===");
    for attr in 0..32 {
        match vm.test_attribute(obj_num as u16, attr) {
            Ok(true) => println!("VM reports attribute {} is SET", attr),
            Ok(false) => {}, // Don't spam with unset attributes
            Err(e) => println!("Error testing attribute {}: {}", attr, e),
        }
    }
    
    // Specifically test attribute 3
    match vm.test_attribute(obj_num as u16, 3) {
        Ok(result) => println!("VM test_attribute(obj {}, attr 3) = {}", obj_num, result),
        Err(e) => println!("Error testing attribute 3: {}", e),
    }

    Ok(())
}