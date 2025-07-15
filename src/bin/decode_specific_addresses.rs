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
    
    println!("=== Decoding specific addresses from debug log ===\n");
    
    // From trace_w_bug_sequence, we found:
    // Property 29 of West of House contains 0x4e
    // print_paddr is called with 0x4e1c
    
    println!("1. Packed address 0x4e1c (from print_paddr call):");
    let addr1 = 0x4e1c * 2; // Unpack
    match text::decode_string(&vm.game.memory[addr1..], vm.game.header.abbrev_table, vm.game.header.version as usize) {
        Ok((text, _)) => println!("   Text: '{}'", text),
        Err(e) => println!("   Error: {}", e),
    }
    
    // Let's also check what property 29 value 0x4e might point to
    println!("\n2. If property 29 value 0x4e is a packed address:");
    let addr2 = 0x4e * 2;
    match text::decode_string(&vm.game.memory[addr2..], vm.game.header.abbrev_table, vm.game.header.version as usize) {
        Ok((text, len)) => {
            println!("   Text: '{}'", text);
            println!("   Length: {} bytes", len);
        }
        Err(e) => println!("   Error: {}", e),
    }
    
    // Check some nearby addresses
    println!("\n3. Checking nearby addresses to 0x4e1c:");
    for offset in &[-4, -2, 0, 2, 4] {
        let packed = (0x4e1c as i32 + offset) as u32;
        let addr = (packed * 2) as usize;
        if addr < vm.game.memory.len() {
            match text::decode_string(&vm.game.memory[addr..], vm.game.header.abbrev_table, vm.game.header.version as usize) {
                Ok((text, _)) => {
                    println!("   0x{:04x}: '{}'", packed, text);
                    if text.contains("can you attack") || text.contains("How") {
                        println!("   ^^^ FOUND IT!");
                    }
                }
                Err(_) => {}
            }
        }
    }
    
    // The user reported seeing "w can you attack a spirit with material objects?"
    // The "w " prefix suggests it's echoing the command
    println!("\n4. Theory: The game might be printing multiple things:");
    println!("   - First: the command 'w' (echo)");
    println!("   - Then: an error message");
    
    Ok(())
}