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
    println!("Analyzing address {address:04x}:");

    // Check if it's a valid routine header
    let locals_count = game.memory[address as usize];
    println!("  Locals count: {locals_count}");

    if locals_count > 15 {
        println!("  INVALID: Locals count > 15");
        return Ok(());
    }

    // Skip local variables for v1-4
    let mut pc = address + 1;
    if game.header.version <= 4 {
        pc += (locals_count as u32) * 2;
    }

    println!("  First instruction at: {pc:04x}");

    // Decode first few instructions
    println!("\n  Instructions:");
    let mut instruction_count = 0;
    let initial_pc = pc;

    while instruction_count < 10 && pc < game.memory.len() as u32 {
        match Instruction::decode(&game.memory, pc as usize, game.header.version) {
            Ok(inst) => {
                println!(
                    "    {:04x}: {:?} (form={:?}, opcode={:02x})",
                    pc, inst, inst.form, inst.opcode
                );

                // Check for invalid opcodes
                if matches!(inst.form, InstructionForm::Long) && inst.opcode == 0x00 {
                    println!("      INVALID: Long form opcode 0x00");
                }

                // Check for returns
                if matches!(
                    inst.opcode,
                    0x00 | 0x01 | 0x02 | 0x03 | 0x08 | 0x09 | 0x0a | 0x0b
                ) && matches!(inst.form, InstructionForm::Short)
                {
                    println!("      RETURN instruction found");
                }

                pc += inst.size as u32;
                instruction_count += 1;
            }
            Err(e) => {
                println!("    {pc:04x}: Failed to decode - {e}");
                break;
            }
        }

        // Detect if we're stuck
        if pc == initial_pc && instruction_count > 0 {
            println!("    WARNING: PC not advancing!");
            break;
        }
    }

    // Check what's around this address
    println!("\n  Context (16 bytes before and after):");
    let start = address.saturating_sub(16);
    for offset in (start..address + 32).step_by(16) {
        if offset >= game.memory.len() as u32 {
            break;
        }

        print!("    {offset:04x}: ");
        for i in 0..16 {
            let addr = offset + i;
            if addr < game.memory.len() as u32 {
                let byte = game.memory[addr as usize];
                print!("{byte:02x} ");
            }
        }
        println!();
    }

    Ok(())
}
