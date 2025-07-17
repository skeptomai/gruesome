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
    
    println!("=== Finding NOT-HERE routines ===\n");
    
    // We know:
    // 1. "You can't see any" is printed at 0x4fde (this is in NOT-HERE-OBJECT-F)
    // 2. Then routine at 0x5092 is called (this is likely NOT-HERE-PRINT)
    // 3. BUFFER-PRINT is at 0x6301
    
    println!("1. Finding NOT-HERE-OBJECT-F (contains print at 0x4fde):");
    
    // Look for routine header before 0x4fde
    for addr in (0x4f00..0x4fde).rev() {
        let locals = game.memory[addr];
        if locals <= 15 {
            let expected_code_start = addr + 1 + (locals as usize * 2);
            if expected_code_start <= 0x4fde {
                println!("\nFound routine at 0x{:04x} with {} locals", addr, locals);
                if let Ok(output) = disasm.disassemble_range(addr as u32, 0x4ff0) {
                    let lines: Vec<&str> = output.lines().collect();
                    for line in lines.iter().take(15) {
                        println!("{}", line);
                        if line.contains("4fde") {
                            println!("  ^-- 'You can't see any' print found!");
                        }
                    }
                }
                break;
            }
        }
    }
    
    println!("\n\n2. Checking routine at 0x5092 (likely NOT-HERE-PRINT):");
    
    // Look for routine header before 0x5092
    for addr in (0x5000..0x5092).rev() {
        let locals = game.memory[addr];
        if locals <= 15 {
            let expected_code_start = addr + 1 + (locals as usize * 2);
            if expected_code_start <= 0x5092 {
                println!("\nFound routine at 0x{:04x} with {} locals", addr, locals);
                if let Ok(output) = disasm.disassemble_range(addr as u32, 0x50b0) {
                    let lines: Vec<&str> = output.lines().collect();
                    for line in lines.iter().take(20) {
                        println!("{}", line);
                    }
                }
                
                // Check if this routine calls BUFFER-PRINT
                println!("\nLooking for calls to BUFFER-PRINT (0x6301)...");
                // Packed address for 0x6301 would be 0x6301/2 = 0x3180
                let packed_addr = 0x3180;
                println!("(BUFFER-PRINT packed address would be 0x{:04x})", packed_addr);
                
                break;
            }
        }
    }
    
    println!("\n\n3. Summary of call chain:");
    println!("   NOT-HERE-OBJECT-F -> prints 'You can't see any'");
    println!("   NOT-HERE-OBJECT-F -> calls NOT-HERE-PRINT");  
    println!("   NOT-HERE-PRINT -> calls BUFFER-PRINT");
    println!("   BUFFER-PRINT (0x6301) -> prints space, calls WORD-PRINT");
    println!("   WORD-PRINT (0x5fda) -> prints the word");
    
    Ok(())
}