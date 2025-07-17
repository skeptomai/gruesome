use gruesome::vm::Game;
use gruesome::disassembler::Disassembler;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";
    
    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;
    
    let game = Game::from_memory(memory)?;
    let disasm = Disassembler::new(&game);
    
    println!("=== Analyzing routine at 0x2fed ===");
    println!("This routine is called with:");
    println!("- Arg 1 (V08): word length from parse buffer");
    println!("- Arg 2 (V00): text position from parse buffer\n");
    
    // Routine at 0x2fed is packed address, so actual address is 0x2fed * 2 = 0x5fda
    let routine_addr = 0x5fda;
    
    println!("Disassembling routine at 0x{:04x}:", routine_addr);
    if let Ok(output) = disasm.disassemble_range(routine_addr, routine_addr + 0x40) {
        println!("{}", output);
    }
    
    // This routine likely:
    // 1. Takes the text position and length
    // 2. Prints characters from the text buffer
    // 3. The bug might be an off-by-one in the loop
    
    println!("\n\nKey insight:");
    println!("If this routine prints length-1 characters instead of length characters,");
    println!("that would explain why 'leaves' (6 chars) prints as 'leave' (5 chars)!");
    
    Ok(())
}