use gruesome::instruction::{Instruction, InstructionForm};
use gruesome::vm::Game;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let memory = fs::read("resources/test/amfv/amfv-r79-s851122.z4")?;
    let game = Game::from_memory(memory.clone())?;

    let addr = 0xe114;
    let locals = memory[addr];
    println!("Routine at {addr:05x}:");
    println!("  Locals: {locals}");

    let mut pc = addr + 1;
    if game.header.version <= 4 {
        pc += (locals as usize) * 2;
    }

    println!("  First instruction at: {pc:05x}");

    // Decode all instructions
    let mut count = 0;
    let mut found_return = false;
    let mut last_valid_pc = pc;

    while pc < memory.len() && count < 20 {
        match Instruction::decode(&memory, pc, game.header.version) {
            Ok(inst) => {
                println!(
                    "  {:05x}: opcode={:02x} form={:?} operand_count={:?} (size={})",
                    pc, inst.opcode, inst.form, inst.operand_count, inst.size
                );

                // Check if it's a return (matching is_return_instruction logic)
                let is_return = match (inst.form, inst.operand_count) {
                    (InstructionForm::Short, gruesome::instruction::OperandCount::OP0) => {
                        matches!(inst.opcode, 0x00 | 0x01 | 0x03 | 0x08 | 0x0A)
                    }
                    (InstructionForm::Short, gruesome::instruction::OperandCount::OP1) => {
                        matches!(inst.opcode, 0x0B | 0x0C)
                    }
                    _ => false,
                };

                if is_return {
                    println!("    ^ RETURN instruction");
                    found_return = true;
                }

                last_valid_pc = pc;
                pc += inst.size;
                count += 1;
            }
            Err(e) => {
                println!("  {pc:05x}: FAILED - {e}");
                break;
            }
        }
    }

    println!("\nSummary:");
    println!("  Instructions decoded: {count}");
    println!("  Found return: {found_return}");
    println!("  Last valid PC: {last_valid_pc:05x}");
    println!("  Failed at PC: {pc:05x}");

    // Check memory at failure point
    if pc < memory.len() {
        println!("\nMemory at failure point:");
        for i in 0..16 {
            if pc + i < memory.len() {
                print!("{:02x} ", memory[pc + i]);
            }
        }
        println!();
    }

    Ok(())
}
