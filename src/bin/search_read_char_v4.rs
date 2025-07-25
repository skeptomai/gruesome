use gruesome::disassembler::Disassembler;
use gruesome::vm::Game;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // List of V4+ games to check
    let games = vec![
        ("resources/test/zork1/DATA/ZORK1.DAT", "Zork I"),
        // Add paths to V4+ games here
    ];

    for (path, name) in games {
        println!("\n=== Checking {name} ({path}) ===");

        if let Ok(mut f) = std::fs::File::open(path) {
            let mut memory = Vec::new();
            f.read_to_end(&mut memory)?;

            let game = Game::from_memory(memory)?;
            let version = game.header.version;

            println!("Version: {version}");

            if version < 4 {
                println!("This is a V{version} game - read_char not available");
                continue;
            }

            println!("This is a V{version} game - searching for read_char usage...");

            // Search for VAR:0x16 (read_char)
            let mut count = 0;
            let high_mem = game.header.base_static_mem;

            for addr in 0..high_mem.saturating_sub(2) {
                // Check for variable form opcode
                let byte1 = game.memory[addr];
                if (0xC0..=0xDF).contains(&byte1) {
                    // Variable form with 2OP base
                    let opcode = byte1 & 0x1F;
                    if opcode == 0x16 {
                        count += 1;
                        println!("Found possible read_char at 0x{addr:04x}");
                    }
                } else if byte1 == 0xE0 {
                    // Variable form with VAR base
                    if addr + 1 < game.memory.len() {
                        let opcode = game.memory[addr + 1];
                        if opcode == 0x16 {
                            count += 1;
                            println!("Found read_char at 0x{addr:04x}");

                            // Try to disassemble it
                            let disasm = Disassembler::new(&game);
                            if let Ok((inst_str, _)) = disasm.disassemble_instruction(addr as u32) {
                                println!("  {inst_str}");
                            }
                        }
                    }
                }
            }

            println!("Total read_char instructions found: {count}");
        } else {
            println!("Could not open file: {path}");
        }
    }

    Ok(())
}
