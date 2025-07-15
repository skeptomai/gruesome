use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("=== Checking Z-Machine routine structure ===\n");
    
    // A Z-machine routine starts with a header byte that specifies
    // the number of local variables (0-15)
    
    let addr = 0x577c;
    println!("At address 0x{:04x}:", addr);
    println!("  Header byte: 0x{:02x}", vm.game.memory[addr]);
    
    let local_count = vm.game.memory[addr] & 0x0F;
    println!("  Local variables: {}", local_count);
    
    // The actual code starts after the header and any local variable initializers
    // In Z3, locals are not initialized, so code starts at addr + 1
    let code_start = addr + 1;
    
    println!("\nCode should start at: 0x{:04x}", code_start);
    println!("\nFirst few bytes of code:");
    for i in 0..10 {
        print!(" {:02x}", vm.game.memory[code_start + i]);
    }
    println!();
    
    // The debug log shows the property check happens early in this routine
    // Let's look for a get_prop instruction (0x51 in variable form)
    println!("\nLooking for get_prop instruction (0x51)...");
    for i in 0..50 {
        if vm.game.memory[code_start + i] == 0x51 {
            println!("Found at offset +{} (address 0x{:04x})", i, code_start + i);
        }
    }
    
    Ok(())
}