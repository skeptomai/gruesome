use infocom::vm::{Game, VM};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    // Load game data
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    std::io::Read::read_to_end(&mut f, &mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let vm = VM::new(game);
    
    println!("=== Decoding GET_PROP at 0x57f7 ===\n");
    
    let addr = 0x57f7;
    println!("Bytes at 0x{:04x}:", addr);
    for i in 0..5 {
        print!(" {:02x}", vm.game.memory[addr + i]);
    }
    println!("\n");
    
    // GET_PROP is opcode 0x51 (2OP:0x11 in variable form)
    let opcode = vm.game.memory[addr];
    println!("Opcode: 0x{:02x}", opcode);
    
    if opcode == 0x51 {
        println!("This is GET_PROP (2OP:0x11)");
        
        // Next byte should be operand types
        let types = vm.game.memory[addr + 1];
        println!("Operand types: 0x{:02x}", types);
        
        // Decode operand types
        let type1 = (types >> 6) & 0x03;
        let type2 = (types >> 4) & 0x03;
        
        println!("  First operand type: {} ({})", type1, 
                 match type1 {
                     0 => "large constant",
                     1 => "small constant", 
                     2 => "variable",
                     3 => "omitted",
                     _ => "?"
                 });
        
        // First operand
        let op1_byte = vm.game.memory[addr + 2];
        println!("  First operand byte: 0x{:02x}", op1_byte);
        
        println!("\n  Wait, type=1 means small constant, not variable!");
        println!("  So first operand is the constant 0x11 (17)");
        
        // The variable must be in the operand types byte
        // Looking at 51 7f 11 00:
        // 51 = opcode
        // 7f = operand types AND first operand!
        // For 2OP in variable form with type=1 (small constant),
        // the value is in the types byte itself
        
        println!("\n  Actually, 0x7f is the variable number!");
        println!("  Variable 0x7f = global 0x{:02x} ({})", 0x7f - 0x10, 0x7f - 0x10);
    }
    
    println!("\nConclusion: GET_PROP G7f,#11");
    println!("G7f = global 0x{:02x} = global {}", 0x7f - 0x10, 0x7f - 0x10);
    println!("This was set to 4 at PC 04f8b!");
    
    Ok(())
}