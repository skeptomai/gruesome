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
    
    println!("=== Tracing the 'w' Bug Sequence ===\n");
    
    // From the debug log, we see:
    // 1. Dictionary lookup finds 'w' with type 0x32
    // 2. Game checks property 17 of objects (action handler)
    // 3. Object 4 (player) has property 17 = 00 00
    // 4. Game calls address 0x0000 (NULL)
    // 5. Game then calls address 0x4552 (unpacked: 0x8aa4)
    // 6. That routine checks property 29 of object 180 (West of House)
    // 7. Then print_paddr at 0x08adc with operand 0x4e1c
    
    println!("Step 1: Dictionary entry for 'w'");
    let dict_addr = 0x4d42;
    println!("  Address: 0x{:04x}", dict_addr);
    println!("  Type byte (offset 4): 0x{:02x}", vm.game.memory[dict_addr + 4]);
    println!("  Data bytes: 0x{:02x} 0x{:02x}", 
             vm.game.memory[dict_addr + 5], 
             vm.game.memory[dict_addr + 6]);
    
    println!("\nStep 2: Object 4 (player) property 17");
    if let Ok(prop_addr) = vm.get_property_addr(4, 17) {
        if prop_addr != 0 {
            let size_byte = vm.game.memory[prop_addr - 1];
            let size = ((size_byte >> 5) & 0x07) + 1;
            println!("  Property 17 address: 0x{:04x}", prop_addr);
            println!("  Property 17 size: {}", size);
            print!("  Property 17 data:");
            for i in 0..size {
                print!(" {:02x}", vm.game.memory[prop_addr + i as usize]);
            }
            println!();
        } else {
            println!("  Property 17: NOT FOUND");
        }
    }
    
    println!("\nStep 3: Object 180 (West of House) property 29");
    if let Ok(prop_addr) = vm.get_property_addr(180, 29) {
        if prop_addr != 0 {
            let size_byte = vm.game.memory[prop_addr - 1];
            let size = ((size_byte >> 5) & 0x07) + 1;
            println!("  Property 29 address: 0x{:04x}", prop_addr);
            println!("  Property 29 size: {}", size);
            print!("  Property 29 data:");
            for i in 0..size {
                print!(" {:02x}", vm.game.memory[prop_addr + i as usize]);
            }
            println!();
        } else {
            println!("  Property 29: NOT FOUND");
        }
    }
    
    println!("\nStep 4: Text at packed address 0x4e1c");
    let packed_addr = 0x4e1c;
    let unpacked = (packed_addr as usize) * 2;
    println!("  Packed address: 0x{:04x}", packed_addr);
    println!("  Unpacked address: 0x{:04x}", unpacked);
    
    match text::decode_string(&vm.game.memory[unpacked..], vm.game.header.abbrev_table, vm.game.header.version as usize) {
        Ok((text, _)) => {
            println!("  Decoded text: '{}'", text);
            if text.contains("can you attack") {
                println!("\n*** THIS IS THE GARBAGE TEXT! ***");
            }
        }
        Err(e) => println!("  Error decoding: {}", e),
    }
    
    println!("\n=== Analysis ===");
    println!("The bug occurs because:");
    println!("1. Dictionary type 0x32 triggers special handling");
    println!("2. Game looks for property 17 (action handler) on objects");
    println!("3. Player object has property 17 = 00 00, causing NULL call");
    println!("4. After NULL call, game calls routine at 0x8aa4");
    println!("5. That routine checks property 29 of West of House");
    println!("6. Then prints text from address 0x4e1c (the garbage)");
    
    println!("\nThe NULL call likely corrupts the execution state,");
    println!("causing the wrong text to be printed instead of");
    println!("moving the player west.");
    
    Ok(())
}