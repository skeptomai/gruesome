use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("Searching for SREAD instructions...");
    println!("SREAD is VAR opcode 246 (0xF6), which is encoded as 0x16 after subtracting 0xE0");
    
    // Search for different VAR forms followed by SREAD
    for addr in 0x4000..0x10000 {
        let byte1 = vm.game.memory[addr];
        let byte2 = if addr + 1 < vm.game.memory.len() { vm.game.memory[addr + 1] } else { 0 };
        
        // Check for VAR forms (0xC0-0xFF) followed by SREAD (0x16)
        if byte1 >= 0xC0 && byte2 == 0x16 {
            println!("\nFound potential SREAD at {:04x}: {:02x} {:02x}", addr, byte1, byte2);
            
            // Show surrounding bytes
            print!("  Context: ");
            for i in 0..10 {
                if addr + i < vm.game.memory.len() {
                    print!("{:02x} ", vm.game.memory[addr + i]);
                }
            }
            println!();
            
            // If it's VAR:4OP (0xE0-0xE3), decode the operand types
            if byte1 >= 0xE0 && byte1 <= 0xE3 {
                let types_byte = if addr + 2 < vm.game.memory.len() { vm.game.memory[addr + 2] } else { 0 };
                let op1_type = (types_byte >> 6) & 0x03;
                let op2_type = (types_byte >> 4) & 0x03;
                
                println!("  VAR:4OP form, operand types: op1={}, op2={}", op1_type, op2_type);
                
                // Try to decode operands if they're large constants
                if op1_type == 0 && op2_type == 0 && addr + 6 < vm.game.memory.len() {
                    let text_buf = ((vm.game.memory[addr + 3] as u16) << 8) | vm.game.memory[addr + 4] as u16;
                    let parse_buf = ((vm.game.memory[addr + 5] as u16) << 8) | vm.game.memory[addr + 6] as u16;
                    println!("  Potential buffers: text={:04x}, parse={:04x}", text_buf, parse_buf);
                }
            }
        }
    }
    
    // Also look at the specific address we know from the trace
    println!("\n\nChecking known SREAD location at 0x51ac:");
    for i in 0..10 {
        let addr = 0x51ac + i;
        if addr < vm.game.memory.len() {
            print!("{:02x} ", vm.game.memory[addr]);
        }
    }
    println!();
    
    Ok(())
}