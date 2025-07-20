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

    println!("=== Tracing Error Message Word Printing ===\n");

    // We know:
    // 1. "You can't see any" is printed at 0x4fde
    // 2. Then routine 0x5092 is called
    // 3. A space is printed at 0x630c
    // 4. Then something prints "leave" (not "leaves")

    // Let's trace the routine at 0x5092 more carefully
    println!("Routine at 0x5092 (after 'You can't see any'):");
    if let Ok(output) = disasm.disassemble_range(0x5092, 0x5100) {
        println!("{output}");
    }

    // The routine seems to be checking variables and calling print_addr
    // print_addr V59 at 0x50a0 might be key

    println!("\n\nLooking for where the word gets prepared...");

    // When the game can't find an object, it needs to print what the player typed
    // This is usually stored in the text buffer or parse buffer

    // Text buffer is at 0x2641
    // Parse buffer is at 0x2551

    println!("\nText/Parse buffer addresses:");
    println!("  Text buffer: 0x2641");
    println!("  Parse buffer: 0x2551");

    // The game might be:
    // 1. Reading the word from text buffer directly
    // 2. Using parse buffer info to extract from text buffer
    // 3. Using a dictionary word

    // Let's look for loadb instructions that might be reading the word
    println!("\n\nSearching for byte loads from text buffer area...");

    for addr in 0x5000..0x6400 {
        if let Ok((inst, text)) = disasm.disassemble_instruction(addr as u32) {
            // Look for loadb instructions
            if inst.opcode == 0x10 {
                // loadb
                if inst.operands.len() >= 2 {
                    let base = inst.operands[0];
                    // Check if it's accessing text buffer area
                    if (0x2600..=0x2700).contains(&base) {
                        println!("\nloadb from text area at 0x{addr:04x}: {text}");
                        // Show context
                        if let Ok(output) =
                            disasm.disassemble_range((addr - 5) as u32, (addr + 10) as u32)
                        {
                            for line in output.lines() {
                                println!("  {line}");
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
