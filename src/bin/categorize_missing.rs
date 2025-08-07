use gruesome::instruction::{Instruction, InstructionForm};
use gruesome::vm::Game;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let memory = fs::read("resources/test/amfv/amfv-r79-s851122.z4")?;
    let game = Game::from_memory(memory.clone())?;

    // The 36 routines TXD finds that we don't
    let missing_routines = vec![
        0x12a04, 0x12b18, 0x12b38, 0x1b0d8, 0x1b980, 0x1bf3c, 0x1d854, 0x1da50, 0x1dc1c, 0x1e138,
        0x1f250, 0x20ae8, 0x25b9c, 0x25bc0, 0x275a8, 0x27618, 0x27890, 0x279c4, 0x27b24, 0x2846c,
        0x289a4, 0x28d24, 0x2b248, 0x2d768, 0x2d840, 0x2d868, 0x319b4, 0x32f94, 0x33c04, 0x35a44,
        0x35b50, 0x38d1c, 0x38d6c, 0x38db0, 0x38e78, 0x38ed0,
    ];

    let mut fall_through = Vec::new();
    let mut data_referenced = Vec::new();
    let mut other = Vec::new();

    for &addr in &missing_routines {
        match categorize_routine(&memory, addr, game.header.version) {
            Category::FallThrough => fall_through.push(addr),
            Category::DataReferenced => data_referenced.push(addr),
            Category::Other(reason) => {
                other.push((addr, reason));
            }
        }
    }

    println!("=== CATEGORIZATION OF 36 MISSING ROUTINES ===");
    println!(
        "\nFall-through (no proper terminator): {} routines",
        fall_through.len()
    );
    for &addr in &fall_through {
        println!("  {:05x}", addr);
    }

    println!(
        "\nData-referenced (valid but not in code flow): {} routines",
        data_referenced.len()
    );
    for &addr in &data_referenced {
        println!("  {:05x}", addr);
    }

    println!("\nOther issues: {} routines", other.len());
    for (addr, reason) in &other {
        println!("  {:05x}: {}", addr, reason);
    }

    Ok(())
}

enum Category {
    FallThrough,
    DataReferenced,
    Other(String),
}

fn categorize_routine(memory: &[u8], addr: u32, version: u8) -> Category {
    if addr as usize >= memory.len() {
        return Category::Other("Address out of bounds".to_string());
    }

    let locals = memory[addr as usize];
    if locals > 15 {
        return Category::Other(format!("Invalid locals: {}", locals));
    }

    let mut pc = addr as usize + 1;
    if version <= 4 {
        pc += locals as usize * 2;
    }

    // Scan up to 100 instructions looking for terminator
    let mut has_terminator = false;
    let mut instruction_count = 0;

    for _ in 0..100 {
        if pc >= memory.len() {
            break;
        }

        match Instruction::decode(memory, pc, version) {
            Ok(inst) => {
                instruction_count += 1;

                // Check for terminator
                match (inst.form, inst.opcode) {
                    // ret, rtrue, rfalse, ret_popped
                    (InstructionForm::Short, 0x00..=0x03) |
                    (InstructionForm::Short, 0x08) | // ret_popped
                    (InstructionForm::Short, 0x0a) | // quit
                    (InstructionForm::Short, 0x0b) if inst.operand_count == gruesome::instruction::OperandCount::OP1 => { // ret value
                        has_terminator = true;
                        break;
                    }
                    // Unconditional jump
                    (InstructionForm::Short, 0x0c) => {
                        has_terminator = true;
                        break;
                    }
                    _ => {}
                }

                pc += inst.size;
            }
            Err(e) => {
                // Hit invalid instruction
                if e.contains("Invalid Long form opcode 0x00") {
                    return Category::Other("Hits invalid opcode 0x00".to_string());
                }
                break;
            }
        }
    }

    if instruction_count == 0 {
        Category::Other("No valid instructions".to_string())
    } else if !has_terminator {
        Category::FallThrough
    } else {
        // Has proper terminator - must be data-referenced
        Category::DataReferenced
    }
}
