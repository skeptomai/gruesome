use infocom::vm::{Game, VM};
use infocom::zobject::ObjectTable;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("=== Investigating 'w' Processing ===\n");
    
    // The key difference is byte 4:
    // - 'e'/'n' have 0x13
    // - 'w' has 0x32
    
    // Dictionary entries store different types of data:
    // Type 0x13: Simple verb/action number in byte 5
    // Type 0x32: Something else - maybe a routine address or object/property combo
    
    // Let's check if 0xa1 (161) is a valid object number
    println!("Checking if 0xa1 (161) is a valid object:");
    if let Some(obj_table) = vm.game.get_object_table() {
        if let Ok(addr) = obj_table.get_object_addr(0xa1) {
            println!("  Object 161 exists at address: {:04x}", addr);
            
            // Get object details
            if let Ok(parent) = obj_table.get_parent(0xa1) {
                println!("  Parent: {}", parent);
            }
            
            // Check if it has property 0x1d (29)
            if let Ok(prop_addr) = obj_table.get_prop_addr(0xa1, 0x1d) {
                println!("  Has property 29 at address: {:04x}", prop_addr);
                
                // Read property data
                if let Ok(prop_data) = obj_table.get_prop_data(0xa1, 0x1d) {
                    print!("  Property 29 data: ");
                    for byte in prop_data {
                        print!("{:02x} ", byte);
                    }
                    println!();
                }
            } else {
                println!("  Does NOT have property 29");
            }
        } else {
            println!("  Object 161 does NOT exist");
        }
    }
    
    // Let's check what the garbage text might be
    println!("\n=== Looking for the garbage text source ===");
    
    // The user reported "w can you attack a spirit with material objects?"
    // This sounds like it might be from reading the wrong text
    
    // When dictionary type 0x32 is processed incorrectly, it might:
    // 1. Use bytes 5-6 as a text address (0xa11d)
    // 2. Use byte 5 as object and byte 6 as property
    // 3. Something else entirely
    
    // Let's check what text is near common addresses
    println!("\nChecking for text patterns in memory:");
    
    // Search for "attack" which appeared in the garbage
    for addr in 0x4000..0x10000 {
        if addr + 6 < vm.game.memory.len() {
            if vm.game.memory[addr] == b'a' &&
               vm.game.memory[addr+1] == b't' &&
               vm.game.memory[addr+2] == b't' &&
               vm.game.memory[addr+3] == b'a' &&
               vm.game.memory[addr+4] == b'c' &&
               vm.game.memory[addr+5] == b'k' {
                println!("\nFound 'attack' at {:04x}:", addr);
                // Show context
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
            }
        }
    }
    
    // Let's also check if the issue is with how action numbers are handled
    println!("\n=== Comparing action handling ===");
    
    println!("\nFor 'e' (type 0x13, action 0x1e):");
    println!("  This is a simple action number");
    println!("  Game likely has a table mapping actions to routines");
    
    println!("\nFor 'w' (type 0x32, data 0xa1 0x1d):");
    println!("  This is NOT a simple action");
    println!("  Possible interpretations:");
    println!("    1. Object 0xa1 (161), property 0x1d (29)");
    println!("    2. Packed routine address");
    println!("    3. Special handling required");
    
    // The bug might be that our interpreter doesn't handle type 0x32 correctly
    // and tries to use it as a simple action number, causing it to read from
    // the wrong place
    
    println!("\n=== Hypothesis ===");
    println!("The interpreter might be treating ALL dictionary entries as type 0x13");
    println!("and using bytes 5-6 as an action number, which for 'w' gives 0xa11d");
    println!("This large number might be used as an index into an action table,");
    println!("causing it to read garbage from memory.");
    
    Ok(())
}