use gruesome::disassembler::Disassembler;
use gruesome::text;
use gruesome::vm::Game;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_path = "resources/test/zork1/DATA/ZORK1.DAT";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;
    let disasm = Disassembler::new(&game);

    println!("=== Finding exact 'leave' print location ===\n");

    // We know:
    // 1. Space is printed at 0x630c
    // 2. Then "leave" is printed (missing 's')
    // 3. Then " here!" is printed

    // Let's look for " here!" to find where that comes from
    let abbrev_addr = game.header.abbrev_table as usize;

    for addr in 0..game.memory.len() - 10 {
        if let Ok((text, _)) = text::decode_string(&game.memory, addr, abbrev_addr) {
            if text == " here!" || text.contains(" here!") && text.len() < 20 {
                println!("Found ' here!' at 0x{:04x}: \"{}\"", addr, text);

                // Check if this is after a print instruction
                if addr > 0 && game.memory[addr - 1] == 0xb2 {
                    println!("  >>> This is a print instruction!");
                }
            }
        }
    }

    // Now let's trace the exact execution path after the space
    println!("\n\nExecution after space print:");
    if let Ok(output) = disasm.disassemble_range(0x630f, 0x6330) {
        println!("{}", output);
    }

    // The key seems to be what happens between the space and " here!"
    // There must be something that prints "leave"

    println!("\n\nLooking more carefully at the flow...");

    // Let's check if the game is using the text buffer directly
    // to print part of the user's input

    // When the error happens, the game needs to show what word wasn't found
    // It might be reading from the text buffer based on parse buffer info

    println!("\nParse buffer structure:");
    println!("For 'move leaves', the parse buffer would contain:");
    println!("- Word 1: 'move' dictionary address, length 4, position 0");
    println!("- Word 2: 'leaves' dictionary address OR 0 if not found, length 6, position 5");

    // The game might be printing characters from position 5 for length 5 (not 6!)
    // That would give "leave" instead of "leaves"

    Ok(())
}
