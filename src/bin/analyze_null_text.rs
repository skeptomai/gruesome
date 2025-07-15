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
    
    println!("=== Analyzing text at address 0x0000 ===\n");
    
    // In Z-Machine, text is often encoded. Let's look at raw bytes first
    println!("Raw bytes at 0x0000:");
    for i in 0..40 {
        print!("{:02x} ", vm.game.memory[i]);
        if (i + 1) % 16 == 0 {
            println!();
        }
    }
    println!();
    
    // The header occupies the first 64 bytes, so actual game data starts after
    // But a call to 0x0000 might be trying to execute header data as code
    
    // Let's decode what happens if we try to execute from 0x0000
    println!("\nTrying to decode instructions at 0x0000:");
    for addr in (0..20).step_by(2) {
        let byte1 = vm.game.memory[addr];
        let byte2 = vm.game.memory[addr + 1];
        println!("  {:04x}: {:02x} {:02x}", addr, byte1, byte2);
    }
    
    // Get abbreviation table address from header
    let abbrev_table_addr = vm.game.header.abbrev_table as usize;
    
    // The garbage text "w can you attack a spirit with material objects?"
    // might be coming from trying to decode header bytes as Z-Machine text
    
    // Let's try to decode some text from address 0
    println!("\nTrying to decode as Z-Machine text from various addresses:");
    for start_addr in vec![0, 1, 2, 3, 4, 5] {
        print!("  From {:04x}: ", start_addr);
        match text::decode_string(&vm.game.memory, start_addr, abbrev_table_addr) {
            Ok((text, _)) => {
                // Limit to first 80 chars
                let truncated = if text.len() > 80 {
                    format!("{}...", &text[..80])
                } else {
                    text
                };
                println!("{:?}", truncated);
            },
            Err(e) => println!("Error: {}", e),
        }
    }
    
    // The text "can you attack" might be part of a larger string
    // Let's search for it in the game's string data
    println!("\n=== Searching for related text ===");
    
    // In Z-Machine, strings are often found in specific areas
    // Let's check the abbreviations table and high memory
    for addr in (0x1000..0x20000).step_by(2) {
        if addr + 20 < vm.game.memory.len() {
            match text::decode_string(&vm.game.memory, addr, abbrev_table_addr) {
                Ok((text, _)) => {
                    if text.contains("attack") && text.contains("spirit") {
                        println!("\nFound at {:05x}: {:?}", addr, text);
                        
                        // Show some context
                        println!("  Previous 20 bytes:");
                        print!("    ");
                        for i in 0..20 {
                            if addr >= i + 20 {
                                print!("{:02x} ", vm.game.memory[addr - 20 + i]);
                            }
                        }
                        println!();
                    }
                },
                Err(_) => {},
            }
        }
    }
    
    // Also look for instruction patterns that might print this text
    println!("\n=== Looking for print instructions ===");
    
    // The NULL call issue: when PERFORM calls address 0x0000, it's executing
    // header data as code. Let's see what instructions that would produce
    
    println!("\nDecoding header bytes as instructions:");
    let mut pc = 0;
    while pc < 20 {
        let opcode = vm.game.memory[pc];
        
        // Check instruction form
        let form = if opcode == 0xbe {
            println!("  {:04x}: {:02x} - Extended instruction", pc, opcode);
            pc += 1;
            continue;
        } else if opcode >= 0xc0 {
            "Variable"
        } else if opcode >= 0x80 {
            "Short"
        } else {
            "Long"
        };
        
        println!("  {:04x}: {:02x} - {} form", pc, opcode, form);
        
        // For print instructions (opcodes 0x01-0x03 in short form)
        if form == "Short" && (opcode & 0x1f) >= 1 && (opcode & 0x1f) <= 3 {
            println!("    -> This is a print instruction!");
            // Skip to see what it would print
            pc += 1;
            
            // Try to decode the following bytes as text
            match text::decode_string(&vm.game.memory, pc, abbrev_table_addr) {
                Ok((text, len)) => {
                    println!("    -> Would print: {:?}", text);
                    pc += len;
                    continue;
                },
                Err(_) => {},
            }
        }
        
        pc += 1;
    }
    
    Ok(())
}