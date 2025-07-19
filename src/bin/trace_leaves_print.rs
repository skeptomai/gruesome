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

    println!("=== Analyzing 'leaves' printing issue ===\n");

    // We know a space is printed at 0x630c
    // Let's see what happens after that
    println!("After space print at 0x630c:");
    if let Ok(output) = disasm.disassemble_range(0x630c, 0x6350) {
        println!("{}", output);
    }

    // Looking for where "leave" gets printed
    // It could be:
    // 1. A print instruction with the string
    // 2. A print_paddr pointing to packed string
    // 3. Byte-by-byte character printing

    println!("\n\nSearching for 'leave' string print...");

    // The string at 0x630c+1 is just a space
    // So "leave" must be printed elsewhere

    // Let's check if it's using print_paddr
    for addr in 0x6310..0x6380 {
        if let Ok((inst, text)) = disasm.disassemble_instruction(addr as u32) {
            if inst.opcode == 0x0D {
                // print_paddr
                println!("\nFound print_paddr at 0x{:04x}: {}", addr, text);
                if let Some(op) = inst.operands.get(0) {
                    let packed_addr = *op;
                    let unpacked = packed_addr as usize * 2;
                    println!("  Packed address: 0x{:04x}", packed_addr);
                    println!("  Unpacked address: 0x{:04x}", unpacked);

                    // Try to decode string at that address
                    let abbrev_addr = game.header.abbrev_table as usize;
                    if let Ok(decoded) = text::decode_string_at_packed_addr(
                        &game.memory,
                        packed_addr,
                        game.header.version,
                        abbrev_addr,
                    ) {
                        println!("  String: \"{}\"", decoded);
                        if decoded.contains("leave") {
                            println!("  >>> Found 'leave'!");
                        }
                    }
                }
            }
        }
    }

    // Also check for byte-by-byte printing with print_char
    println!("\n\nChecking for print_char sequences...");
    for addr in 0x6310..0x6350 {
        if let Ok((inst, text)) = disasm.disassemble_instruction(addr as u32) {
            if inst.opcode == 0x05 {
                // print_char
                println!("print_char at 0x{:04x}: {}", addr, text);
            }
        }
    }

    Ok(())
}
