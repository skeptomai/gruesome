use gruesome::instruction::{Instruction, InstructionForm};
use gruesome::vm::Game;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <game.z4> <address>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    let address = u32::from_str_radix(&args[2], 16)?;

    let memory = std::fs::read(filename)?;
    let game = Game::from_memory(memory)?;

    println!("Checking if {address:04x} could be reached by fallthrough...");

    // Look backward from the address to find instructions that could fall through
    let mut check_addr = address.saturating_sub(20);

    while check_addr < address {
        match Instruction::decode(&game.memory, check_addr as usize, game.header.version) {
            Ok(inst) => {
                let next_addr = check_addr + inst.size as u32;

                // Check if this instruction ends exactly at our target address
                if next_addr == address {
                    // Check if it's a non-branching instruction that would fall through
                    let is_return = matches!(
                        inst.opcode,
                        0x00 | 0x01 | 0x02 | 0x03 | 0x08 | 0x09 | 0x0a | 0x0b
                    ) && matches!(inst.form, InstructionForm::Short);
                    let is_jump =
                        inst.opcode == 0x0c && matches!(inst.form, InstructionForm::Short);
                    let is_quit =
                        inst.opcode == 0x0a && matches!(inst.form, InstructionForm::Short);

                    if !is_return && !is_jump && !is_quit {
                        println!("  FALLTHROUGH possible from {check_addr:04x}: {inst:?}");
                        println!(
                            "  Instruction at {check_addr:04x} would naturally continue to {address:04x}"
                        );
                        return Ok(());
                    } else {
                        println!(
                            "  Instruction at {:04x} is a {:?} - no fallthrough",
                            check_addr,
                            if is_return {
                                "return"
                            } else if is_jump {
                                "jump"
                            } else {
                                "quit"
                            }
                        );
                    }
                }

                check_addr += 1;
            }
            Err(_) => {
                check_addr += 1;
            }
        }
    }

    println!("  No fallthrough found - address appears to be a genuine routine entry point");

    Ok(())
}
