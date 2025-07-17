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
    
    println!("=== Analyzing print sequence ===\n");
    
    // The sequence is:
    // 0x630c: print space
    // 0x6340: print_obj V7b
    
    println!("1. Space print at 0x630c:");
    if let Ok(output) = disasm.disassemble_range(0x630c, 0x6310) {
        println!("{}", output);
    }
    
    println!("\n2. Object name print at 0x6340:");
    if let Ok(output) = disasm.disassemble_range(0x633e, 0x6345) {
        println!("{}", output);
    }
    
    // The key is: what's in V7b?
    // Let's trace back to see where V7b is loaded
    println!("\n3. Where V7b is loaded (0x633e):");
    if let Ok(output) = disasm.disassemble_range(0x6335, 0x6345) {
        println!("{}", output);
    }
    
    // loadb #0047, #00aa -> V7b
    // This loads a byte from address 0x0047 + 0x00aa = 0x00f1
    let addr = 0x0047 + 0x00aa;
    println!("\n4. What's at address 0x{:04x} (0x47 + 0xaa)?", addr);
    println!("   Byte value: 0x{:02x} ({})", game.memory[addr], game.memory[addr]);
    
    // This might be loading a truncated object number or word number
    println!("\n5. Checking if this relates to 'leaves' text:");
    
    // Let's see what happens if we use this as a word offset
    let word_num = game.memory[addr];
    println!("   If {} is a word number in the object name...", word_num);
    
    // For object 144 "pile of leaves"
    // Word 0-1: "pile "
    // Word 2-3: "of leave"  
    // Word 4: "s"
    
    Ok(())
}