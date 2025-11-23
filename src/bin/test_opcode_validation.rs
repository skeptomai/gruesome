use gruesome::interpreter::core::instruction::{Instruction, InstructionForm};
use log::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Test data with invalid Long form opcode 0x00
    let test_data = vec![
        0x00, 0x00, 0x00, // Long form opcode 0x00 (invalid)
        0x01, 0x00, 0x00, // Long form opcode 0x01 (je)
        0xb0, // Short form opcode 0x00 (rtrue)
    ];

    // Test decoding
    info!("Testing instruction decoding with invalid opcodes...");

    // Test Long form 0x00 (should be invalid)
    match Instruction::decode(&test_data, 0, 3) {
        Ok(inst) => {
            info!("Long 0x00: Decoded as {:?}", inst);
            if matches!(inst.form, InstructionForm::Long) && inst.opcode == 0x00 {
                info!("⚠️  WARNING: Accepting invalid Long form opcode 0x00!");
            }
        }
        Err(e) => {
            info!("Long 0x00: Correctly rejected - {}", e);
        }
    }

    // Test Long form 0x01 (valid)
    match Instruction::decode(&test_data, 3, 3) {
        Ok(inst) => {
            info!("Long 0x01: Correctly decoded as {:?}", inst);
        }
        Err(e) => {
            info!("Long 0x01: Incorrectly rejected - {}", e);
        }
    }

    // Test Short form 0x00 (valid rtrue)
    match Instruction::decode(&test_data, 6, 3) {
        Ok(inst) => {
            info!("Short 0x00: Correctly decoded as {:?}", inst);
        }
        Err(e) => {
            info!("Short 0x00: Incorrectly rejected - {}", e);
        }
    }

    Ok(())
}
