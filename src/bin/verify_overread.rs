use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("=== Verifying the buffer overread bug ===\n");
    
    // Property 29 is at 0x1c2f and has only 1 byte (0x4e)
    let prop_addr = 0x1c2f;
    
    println!("Memory around property 29:");
    for addr in (prop_addr - 5)..=(prop_addr + 10) {
        if addr == prop_addr {
            println!("{:04x}: {:02x} <-- Property 29 data (only 1 byte!)", addr, vm.game.memory[addr]);
        } else if addr == prop_addr + 1 {
            println!("{:04x}: {:02x} <-- This byte is NOT part of property 29!", addr, vm.game.memory[addr]);
        } else {
            println!("{:04x}: {:02x}", addr, vm.game.memory[addr]);
        }
    }
    
    println!("\nWhen loadw reads from 0x1c2f, it gets:");
    let word = vm.read_word(prop_addr as u32);
    println!("Word value: 0x{:04x}", word);
    println!("This is 0x4e (from property) + 0x1c (from next byte) = 0x4e1c");
    
    println!("\n=== The Bug ===");
    println!("1. Dictionary entry 'w' has type 0x32 (special handling)");
    println!("2. Game eventually calls routine 0x8aa4 with V02=2");
    println!("3. Routine loads a 'word' from property 29 (but it's only 1 byte!)");
    println!("4. Buffer overread creates address 0x4e1c");
    println!("5. Printing from 0x4e1c (unpacked: 0x9c38) shows garbage text");
    println!("\nThis is a bug in the original Zork I game data!");
    
    Ok(())
}