use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("=== Searching for garbage text components ===\n");
    
    // Search for "can you attack" which appeared in the garbage
    for addr in 0..vm.game.memory.len().saturating_sub(14) {
        if vm.game.memory[addr] == b'c' &&
           vm.game.memory[addr+1] == b'a' &&
           vm.game.memory[addr+2] == b'n' &&
           vm.game.memory[addr+3] == b' ' &&
           vm.game.memory[addr+4] == b'y' &&
           vm.game.memory[addr+5] == b'o' &&
           vm.game.memory[addr+6] == b'u' &&
           vm.game.memory[addr+7] == b' ' &&
           vm.game.memory[addr+8] == b'a' &&
           vm.game.memory[addr+9] == b't' &&
           vm.game.memory[addr+10] == b't' &&
           vm.game.memory[addr+11] == b'a' &&
           vm.game.memory[addr+12] == b'c' &&
           vm.game.memory[addr+13] == b'k' {
            println!("Found 'can you attack' at {:05x}:", addr);
            // Show context
            print!("  Context: ");
            for i in 0..80 {
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
            
            // Also show hex context around this address
            println!("  Hex context before:");
            if addr >= 20 {
                print!("    {:05x}: ", addr - 20);
                for i in 0..20 {
                    if addr - 20 + i < vm.game.memory.len() {
                        print!("{:02x} ", vm.game.memory[addr - 20 + i]);
                    }
                }
                println!();
            }
        }
    }
    
    // Search for "material objects" which also appeared
    for addr in 0..vm.game.memory.len().saturating_sub(16) {
        if vm.game.memory[addr] == b'm' &&
           vm.game.memory[addr+1] == b'a' &&
           vm.game.memory[addr+2] == b't' &&
           vm.game.memory[addr+3] == b'e' &&
           vm.game.memory[addr+4] == b'r' &&
           vm.game.memory[addr+5] == b'i' &&
           vm.game.memory[addr+6] == b'a' &&
           vm.game.memory[addr+7] == b'l' &&
           vm.game.memory[addr+8] == b' ' &&
           vm.game.memory[addr+9] == b'o' &&
           vm.game.memory[addr+10] == b'b' &&
           vm.game.memory[addr+11] == b'j' &&
           vm.game.memory[addr+12] == b'e' &&
           vm.game.memory[addr+13] == b'c' &&
           vm.game.memory[addr+14] == b't' &&
           vm.game.memory[addr+15] == b's' {
            println!("\nFound 'material objects' at {:05x}:", addr);
            // Show context
            print!("  Context before: ");
            if addr >= 40 {
                for i in 40..0 {
                    let byte = vm.game.memory[addr - i];
                    if byte >= 32 && byte <= 126 {
                        print!("{}", byte as char);
                    } else {
                        print!(".");
                    }
                }
            }
            print!(" | ");
            for i in 0..40 {
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
    
    // Search for "spirit" 
    for addr in 0..vm.game.memory.len().saturating_sub(6) {
        if vm.game.memory[addr] == b's' &&
           vm.game.memory[addr+1] == b'p' &&
           vm.game.memory[addr+2] == b'i' &&
           vm.game.memory[addr+3] == b'r' &&
           vm.game.memory[addr+4] == b'i' &&
           vm.game.memory[addr+5] == b't' {
            println!("\nFound 'spirit' at {:05x}:", addr);
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
            
            // Check if "attack" appears nearby
            let mut found_attack = false;
            for j in 0..100 {
                if addr + j + 6 < vm.game.memory.len() {
                    if vm.game.memory[addr + j] == b'a' &&
                       vm.game.memory[addr + j + 1] == b't' &&
                       vm.game.memory[addr + j + 2] == b't' &&
                       vm.game.memory[addr + j + 3] == b'a' &&
                       vm.game.memory[addr + j + 4] == b'c' &&
                       vm.game.memory[addr + j + 5] == b'k' {
                        found_attack = true;
                        println!("  'attack' found {} bytes after 'spirit'", j);
                        break;
                    }
                }
            }
            
            if found_attack {
                println!("  *** This might be the source of the garbage text! ***");
            }
        }
    }
    
    Ok(())
}