use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("Global variables start at: {:04x}", vm.game.header.global_variables);
    println!("Dynamic memory ends at: {:04x}", vm.game.header.base_static_mem);
    
    // Check first few globals to see if any point to low memory (potential buffers)
    println!("\nChecking global variables for buffer addresses:");
    
    for i in 0..50 {
        match vm.read_global(i) {
            Ok(value) => {
                // Look for values that could be addresses in dynamic memory
                if value > 0x100 && value < vm.game.header.base_static_mem as u16 {
                    println!("  G{:02}: {:04x} - potential buffer address", i, value);
                    
                    // Check what's at that address
                    if value < vm.game.memory.len() as u16 {
                        print!("    Contents: ");
                        for j in 0..8 {
                            let addr = value as usize + j;
                            if addr < vm.game.memory.len() {
                                print!("{:02x} ", vm.game.memory[addr]);
                            }
                        }
                        println!();
                    }
                }
            }
            Err(e) => {
                println!("  Error reading G{:02}: {}", i, e);
            }
        }
    }
    
    // Also check the known buffer addresses from the original code
    println!("\nChecking known buffer locations:");
    
    let known_addrs = [0x01f5, 0x0225];  // Common text/parse buffer addresses
    for addr in &known_addrs {
        println!("\nAddress {:04x}:", addr);
        print!("  Contents: ");
        for i in 0..16 {
            if addr + i < vm.game.memory.len() {
                print!("{:02x} ", vm.game.memory[addr + i]);
            }
        }
        println!();
    }
    
    Ok(())
}