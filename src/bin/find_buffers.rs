use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("Game header info:");
    println!("  Version: {}", vm.game.header.version);
    println!("  Dynamic memory ends at: {:04x}", vm.game.header.base_static_mem);
    println!("  Global variables at: {:04x}", vm.game.header.global_variables);
    
    // Look for SREAD instructions in the first part of the game
    println!("\nSearching for SREAD instructions (opcode 0xE4 0x16)...");
    
    for addr in 0x4f00..0x6000 {
        // Check for VAR:4OP form (0xE4) followed by SREAD (0x16)
        if vm.game.memory[addr] == 0xE4 && vm.game.memory[addr + 1] == 0x16 {
            println!("\nFound SREAD at {:04x}", addr);
            
            // Decode operands to find buffer addresses
            let mut offset = addr + 2;
            
            // First two operands are large constants (text buffer, parse buffer)
            let op1_type = (vm.game.memory[offset] >> 6) & 0x03;
            let op2_type = (vm.game.memory[offset] >> 4) & 0x03;
            offset += 1;
            
            println!("  Operand types: op1={}, op2={}", op1_type, op2_type);
            
            // Decode first operand (text buffer)
            let text_buffer = if op1_type == 0 {
                // Large constant
                let val = ((vm.game.memory[offset] as u16) << 8) | (vm.game.memory[offset + 1] as u16);
                offset += 2;
                val
            } else {
                println!("  Unexpected operand type for text buffer");
                0
            };
            
            // Decode second operand (parse buffer)
            let parse_buffer = if op2_type == 0 {
                // Large constant
                let val = ((vm.game.memory[offset] as u16) << 8) | (vm.game.memory[offset + 1] as u16);
                val
            } else {
                println!("  Unexpected operand type for parse buffer");
                0
            };
            
            println!("  Text buffer: {:04x}", text_buffer);
            println!("  Parse buffer: {:04x}", parse_buffer);
            
            // Check if these are in dynamic memory
            if text_buffer < vm.game.header.base_static_mem as u16 {
                println!("  ✓ Text buffer is in dynamic memory");
            } else {
                println!("  ✗ Text buffer is in static memory!");
            }
        }
    }
    
    // Also check some low memory locations that might be buffers
    println!("\nChecking common buffer locations in dynamic memory:");
    for addr in (0x100..0x400).step_by(0x50) {
        println!("  {:04x}: {:02x} {:02x} {:02x} {:02x}", 
            addr, 
            vm.game.memory[addr],
            vm.game.memory[addr + 1],
            vm.game.memory[addr + 2],
            vm.game.memory[addr + 3]);
    }
    
    Ok(())
}