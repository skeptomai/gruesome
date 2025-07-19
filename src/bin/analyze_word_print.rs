use gruesome::disassembler::Disassembler;
use gruesome::vm::Game;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;
    let disasm = Disassembler::new(&game);

    println!("=== WORD-PRINT routine at 0x5fda ===\n");

    // Disassemble the complete routine
    if let Ok(output) = disasm.disassemble_range(0x5fda, 0x5ff0) {
        println!("{}", output);
    }

    println!("\n=== Analysis ===");
    println!("Entry: L01 = word length, L02 = text position");
    println!("\nThe loop:");
    println!("1. 0x5fdf: dec_chk L01, #0000 [TRUE RTRUE]");
    println!("   - Decrements L01");
    println!("   - If result < 0, branches to RTRUE (returns)");
    println!("2. 0x5fe3: loadb V7d, V02 -> V00");
    println!("   - Loads byte from text buffer + position");
    println!("3. 0x5fe7: print_char V00");
    println!("   - Prints the character");
    println!("4. 0x5fea: inc L02");
    println!("   - Increments position");
    println!("5. 0x5fec: jump #fff2");
    println!("   - Jumps back to step 1");

    Ok(())
}
