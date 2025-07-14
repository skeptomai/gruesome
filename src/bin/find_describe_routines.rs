use std::fs::File;
use std::io::prelude::*;
use infocom::disassembler::Disassembler;
use infocom::vm::Game;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    let game = Game::from_memory(game_data).map_err(|e| format!("Game load error: {}", e))?;
    let memory = &game.memory;  // Get reference to game memory

    println!("Searching for DESCRIBE-ROOM and DESCRIBE-OBJECTS routines...\n");
    
    // These routines are typically called from the main loop
    // Let's look at the main routine at 0x4f04
    
    let disasm = Disassembler::new(&game);
    
    println!("Main routine at 0x4f04:");
    match disasm.disassemble_range(0x4f04, 0x4f40) {
        Ok(output) => println!("{}", output),
        Err(e) => println!("Error: {}", e),
    }
    
    // Look for typical patterns of DESCRIBE-ROOM
    // It usually:
    // 1. Gets current location (G00)
    // 2. Prints object name
    // 3. Checks if location has been visited
    // 4. Prints description
    
    println!("\n\nSearching for routines that access G00 (location) and print object names...");
    
    // Look for patterns like:
    // - get_var G00
    // - print_obj
    
    for addr in (0x4000..0x10000).step_by(2) {
        if addr + 10 >= memory.len() {
            break;
        }
        
        // Look for common patterns
        let byte = memory[addr];
        
        // Check for Variable form instruction that might be reading G00
        if byte == 0xE5 || byte == 0xA5 { // PRINT_OBJ or GET_VAR patterns
            // Check next bytes
            if addr + 3 < memory.len() {
                // Simple heuristic: if we see print_obj near a G00 reference
                if byte == 0xA5 && memory[addr + 1] == 0x10 { // load G00
                    println!("\nPossible location access at 0x{:04x}", addr);
                    
                    // Show disassembly of this area
                    match disasm.disassemble_range(addr as u32, (addr + 20) as u32) {
                        Ok(output) => {
                            for line in output.lines().take(5) {
                                println!("  {}", line);
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
        }
    }
    
    // Also look for the specific error case - how do we get to 0x86ca?
    println!("\n\nLooking for jump tables or dispatch mechanisms around common addresses:");
    
    // Action dispatch tables are often in the 0x6000-0x8000 range
    for addr in (0x6000..0x8000).step_by(2) {
        if addr + 2 >= memory.len() {
            break;
        }
        
        let word = ((memory[addr] as u16) << 8) | (memory[addr + 1] as u16);
        
        // Check if this looks like a packed address to STAND
        if word == 0x4365 { // 0x86ca / 2
            println!("\nFound packed address 0x4365 (STAND) at 0x{:04x}", addr);
            
            // Show context - might be part of a table
            println!("Context (possible table entries):");
            for i in 0..10 {
                let table_addr = addr.saturating_sub(i * 2);
                if table_addr + 2 < memory.len() {
                    let entry = ((memory[table_addr] as u16) << 8) | (memory[table_addr + 1] as u16);
                    let unpacked = entry * 2;
                    println!("  0x{:04x}: 0x{:04x} -> routine at 0x{:05x}", 
                             table_addr, entry, unpacked);
                }
            }
        }
    }

    Ok(())
}