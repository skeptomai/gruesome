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

    println!("=== Analyzing 'You can't see any X here!' error ===\n");

    // The error starts at 0x4fde with "You can't see any"
    println!("1. Error message start:");
    if let Ok(output) = disasm.disassemble_range(0x4fde, 0x4ff0) {
        println!("{}", output);
    }

    // After printing "You can't see any", it calls routine at 0x5092
    println!("\n2. Routine at 0x5092 (called after 'You can't see any'):");
    if let Ok(output) = disasm.disassemble_range(0x5092, 0x50a5) {
        println!("{}", output);
    }

    // The space print happens later
    println!("\n3. Where the space gets printed (0x630c):");
    if let Ok(output) = disasm.disassemble_range(0x6305, 0x6315) {
        println!("{}", output);
    }

    // Check what the print at 0x630c actually contains
    println!("\n4. Decoding the string at 0x630c:");
    // It's a print instruction, so the string follows immediately
    let string_addr = 0x630c + 1; // Skip the opcode byte
    let abbrev_addr = game.header.abbrev_table as usize;

    match text::decode_string(&game.memory, string_addr, abbrev_addr) {
        Ok((text, bytes)) => {
            println!("   String: \"{}\" ({} bytes)", text.escape_debug(), bytes);
            println!("   String bytes: {:?}", text.as_bytes());
        }
        Err(e) => {
            println!("   Error decoding: {}", e);
        }
    }

    // Find where "leave" gets printed
    println!("\n5. Looking for where 'leave' gets printed:");

    // Search for print instructions in the area
    for addr in 0x6310..0x6380 {
        if let Ok((inst, text)) = disasm.disassemble_instruction(addr as u32) {
            if inst.opcode == 0x02 {
                // print
                println!("\n   Print at 0x{:04x}:", addr);
                // Decode the string
                let str_addr = addr + 1; // Skip opcode byte
                if let Ok((s, _)) = text::decode_string(&game.memory, str_addr, abbrev_addr) {
                    println!("     Text: \"{}\"", s.escape_debug());
                    if s.contains("leave") {
                        println!("     >>> Found 'leave'!");
                    }
                }
            }
        }
    }

    Ok(())
}
