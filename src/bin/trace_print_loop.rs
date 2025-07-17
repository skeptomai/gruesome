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
    
    println!("=== Understanding the print loop bug ===\n");
    
    // The routine at 0x5fda is called with:
    // - Local 1: word length (6 for "leaves")
    // - Local 2: text position (5 for "leaves" starting at position 5)
    
    println!("The print loop at 0x5fda:");
    if let Ok(output) = disasm.disassemble_range(0x5fda, 0x5ff0) {
        println!("{}", output);
    }
    
    println!("\n\nLoop analysis:");
    println!("Entry: Local 1 = 6 (length), Local 2 = 5 (position)");
    println!();
    
    println!("The loop structure:");
    println!("1. dec_chk L01, #0000 [TRUE RTRUE]");
    println!("   - Decrements L01 and branches to RTRUE if < 0");
    println!("2. loadb V7d, V02 -> V00");  
    println!("   - Loads character from text buffer");
    println!("3. print_char V00");
    println!("   - Prints the character");
    println!("4. inc L02");
    println!("   - Increments position");
    println!("5. jump back to step 1");
    println!();
    
    println!("Execution trace:");
    println!("Iter 1: L01=6→5 (5≥0, continue), pos=5, print 'l'");
    println!("Iter 2: L01=5→4 (4≥0, continue), pos=6, print 'e'");
    println!("Iter 3: L01=4→3 (3≥0, continue), pos=7, print 'a'");
    println!("Iter 4: L01=3→2 (2≥0, continue), pos=8, print 'v'");
    println!("Iter 5: L01=2→1 (1≥0, continue), pos=9, print 'e'");
    println!("Iter 6: L01=1→0 (0≥0, continue), pos=10, print 's'");
    println!("Iter 7: L01=0→-1 (-1<0, RTRUE!)");
    println!();
    
    println!("So it SHOULD print all 6 characters!");
    println!();
    
    // Wait, let me check the exact branch condition
    println!("Checking the branch condition more carefully...");
    println!("The instruction: dec_chk L01, #0000 [TRUE RTRUE]");
    println!("The 'c1' at the end means branch on TRUE");
    println!();
    
    // Let me look at the start of the routine to see if locals are set up correctly
    println!("Let's check how the routine is called and set up:");
    
    // Look at where it's called from
    println!("\nThe call at 0x634d:");
    if let Ok(output) = disasm.disassemble_range(0x6345, 0x6355) {
        println!("{}", output);
    }
    
    // The issue might be that V08 (length) is wrong
    // Or that the initial setup is different
    
    Ok(())
}