use gruesome::disassembler::Disassembler;
use gruesome::vm::Game;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("=== Window Operations Implementation Demo ===\n");

    // Load Seastalker to examine window operations
    let game_path = "resources/test/seastalker/seastalker-r18-s850919.z3";

    let mut f = std::fs::File::open(game_path)?;
    let mut memory = Vec::new();
    f.read_to_end(&mut memory)?;

    let game = Game::from_memory(memory)?;
    let disasm = Disassembler::new(&game);

    println!("Seastalker (v3) uses the following window operations:");
    println!("- split_window: Creates upper window for sonar display");
    println!("- set_window: Switches between upper/lower windows");
    println!("- set_cursor: Positions text in the sonar display");
    println!("- erase_window: Clears the sonar display\n");

    // Search for window operation usage in Seastalker
    println!("Searching for window operations in Seastalker code...\n");

    let mut found_split = false;
    let mut found_erase = false;
    let mut found_set_window = false;
    let mut found_set_cursor = false;

    // Scan through the code section
    for addr in 0x1000..0x10000 {
        if let Ok((inst, _)) = disasm.disassemble_instruction(addr as u32) {
            match inst.opcode {
                0x0A if inst.form == gruesome::instruction::InstructionForm::Variable => {
                    if !found_split {
                        println!("Found split_window at 0x{:04x}:", addr);
                        if let Ok(output) = disasm.disassemble_range(addr as u32, (addr + 8) as u32)
                        {
                            for line in output.lines() {
                                println!("  {}", line);
                            }
                        }
                        found_split = true;
                    }
                }
                0x0D if inst.form == gruesome::instruction::InstructionForm::Variable => {
                    if !found_erase {
                        println!("\nFound erase_window at 0x{:04x}:", addr);
                        if let Ok(output) = disasm.disassemble_range(addr as u32, (addr + 8) as u32)
                        {
                            for line in output.lines() {
                                println!("  {}", line);
                            }
                        }
                        found_erase = true;
                    }
                }
                0x0B if inst.form == gruesome::instruction::InstructionForm::Variable => {
                    if !found_set_window {
                        println!("\nFound set_window at 0x{:04x}:", addr);
                        if let Ok(output) = disasm.disassemble_range(addr as u32, (addr + 8) as u32)
                        {
                            for line in output.lines() {
                                println!("  {}", line);
                            }
                        }
                        found_set_window = true;
                    }
                }
                0x0F if inst.form == gruesome::instruction::InstructionForm::Variable => {
                    if !found_set_cursor {
                        println!("\nFound set_cursor at 0x{:04x}:", addr);
                        if let Ok(output) = disasm.disassemble_range(addr as u32, (addr + 8) as u32)
                        {
                            for line in output.lines() {
                                println!("  {}", line);
                            }
                        }
                        found_set_cursor = true;
                    }
                }
                _ => {}
            }

            if found_split && found_erase && found_set_window && found_set_cursor {
                break;
            }
        }
    }

    println!("\n=== Implementation Status ===");
    println!("✓ split_window: IMPLEMENTED (src/interpreter.rs:1816-1828)");
    println!("✓ erase_window: IMPLEMENTED (src/interpreter.rs:1844-1858)");
    println!("✓ set_window: IMPLEMENTED (src/interpreter.rs:1830-1842)");
    println!("✓ set_cursor: IMPLEMENTED (src/interpreter.rs:1873-1888)");
    println!("\nBoth basic and ratatui display backends support these operations.");
    println!("\nThese implementations were added specifically for Seastalker's");
    println!("ASCII sonar display feature (commit abcf13f).");

    Ok(())
}
