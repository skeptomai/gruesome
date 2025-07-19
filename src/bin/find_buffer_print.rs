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

    println!("=== Finding BUFFER-PRINT routine ===");
    println!("\nWe know it contains:");
    println!("  - Print space at 0x630c");
    println!("  - Load parse buffer at 0x6345, 0x6349");
    println!("  - Call to WORD-PRINT (0x5fda) at around 0x6350");

    // Look for the routine header before 0x630c
    println!("\nSearching for routine header before 0x630c...");

    // Check backwards from 0x630c for a routine header
    for addr in (0x6200..0x630c).rev() {
        let locals = game.memory[addr];
        if locals <= 15 {
            // This could be a routine header
            // V3 routines have: num_locals, then 2*num_locals bytes of initial values
            let expected_code_start = addr + 1 + (locals as usize * 2);

            // See if the code after the header looks valid
            if expected_code_start < 0x630c {
                println!("\nPossible routine at 0x{:04x}:", addr);
                println!("  Locals: {}", locals);
                println!("  Code starts at: 0x{:04x}", expected_code_start);

                // Show some disassembly
                if let Ok(output) = disasm.disassemble_range(addr as u32, 0x6320) {
                    println!("\nDisassembly:");
                    let lines: Vec<&str> = output.lines().collect();
                    for (i, line) in lines.iter().take(20).enumerate() {
                        println!("{}", line);
                        if line.contains("630c") {
                            println!("  ^-- Space print found!");
                        }
                    }
                }

                if addr >= 0x62e0 && addr <= 0x6300 {
                    println!("\nThis is likely BUFFER-PRINT!");
                }
            }
        }
    }

    // Also check the call chain
    println!("\n\n=== Call chain analysis ===");
    println!("Looking for who calls this routine...");

    // From the ZIL code, we know:
    // NOT-HERE-OBJECT-F prints "You can't see any"
    // NOT-HERE-PRINT calls BUFFER-PRINT
    // BUFFER-PRINT prints space then calls WORD-PRINT

    println!("\nFrom debug logs, the sequence is:");
    println!("1. \"You can't see any\" printed");
    println!("2. Call to routine that contains 0x630c (BUFFER-PRINT)");
    println!("3. Space printed at 0x630c");
    println!("4. Parse buffer accessed at 0x6345, 0x6349");
    println!("5. Call to WORD-PRINT at 0x5fda");

    Ok(())
}
