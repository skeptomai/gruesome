use gruesome::instruction::Instruction;

fn main() {
    println!("Z-Machine Opcode Mapping Demo for Version 3");
    println!("=============================================\n");

    // Example 1: VAR:224 (0xE0) - call instruction
    println!("Example 1: Decoding 0xE0 (VAR:224 - call)");
    let memory1 = vec![
        0xE0, // Variable form, VAR operand count, opcode 0 = VAR:224 = call
        0x2F, // Operand types: large constant, small constant, omitted, omitted
        0x12, 0x34, // First operand: 0x1234 (routine address)
        0x56, // Second operand: 0x56 (argument)
        0x00, // Store result in local variable 0
        0x00, 0x00, // Padding
    ];

    match Instruction::decode(&memory1, 0, 3) {
        Ok(inst) => {
            println!(
                "  Raw bytes: {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
                memory1[0], memory1[1], memory1[2], memory1[3], memory1[4], memory1[5]
            );
            println!("  Decoded as: {}", inst.format_with_version(3));
            println!(
                "  Form: {:?}, Operand Count: {:?}",
                inst.form, inst.operand_count
            );
            println!("  This is the 'call' instruction, not 'rtrue'!\n");
        }
        Err(e) => println!("  Error: {e}\n"),
    }

    // Example 2: 0OP:176 (0xB0) - rtrue instruction
    println!("Example 2: Decoding 0xB0 (0OP:176 - rtrue)");
    let memory2 = vec![
        0xB0, // Short form, 0OP, opcode 0 = 0OP:176 = rtrue
        0x00, 0x00, // Padding
    ];

    match Instruction::decode(&memory2, 0, 3) {
        Ok(inst) => {
            println!("  Raw byte: {:02X}", memory2[0]);
            println!("  Decoded as: {}", inst.format_with_version(3));
            println!(
                "  Form: {:?}, Operand Count: {:?}",
                inst.form, inst.operand_count
            );
            println!("  This is the actual 'rtrue' instruction!\n");
        }
        Err(e) => println!("  Error: {e}\n"),
    }

    // Example 3: 2OP in variable form (0xC1) - je
    println!("Example 3: Decoding 0xC1 (VAR form of 2OP:1 - je)");
    let memory3 = vec![
        0xC1, // Variable form, 2OP, opcode 1 = je
        0x55, // Operand types: small const, small const, small const, small const
        0x10, // First operand
        0x20, // Second operand
        0x30, // Third operand
        0x40, // Fourth operand
        0x80, // Branch: on true, offset 0
        0x00, 0x00, // Padding
    ];

    match Instruction::decode(&memory3, 0, 3) {
        Ok(inst) => {
            println!(
                "  Raw bytes: {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
                memory3[0], memory3[1], memory3[2], memory3[3], memory3[4], memory3[5], memory3[6]
            );
            println!("  Decoded as: {}", inst.format_with_version(3));
            println!(
                "  Form: {:?}, Operand Count: {:?}",
                inst.form, inst.operand_count
            );
            println!("  Note: je can take 2-4 operands in variable form\n");
        }
        Err(e) => println!("  Error: {e}\n"),
    }

    // Example 4: 1OP:143 (0x8F) - not (V1-4) / call_1n (V5)
    println!("Example 4: Version-dependent opcode 0x8F");
    let memory4 = vec![
        0x8F, // Short form, 1OP, opcode 15
        0x42, // Operand
        0x00, // Store variable (for V1-4)
        0x00, 0x00, // Padding
    ];

    println!("  In Version 3:");
    match Instruction::decode(&memory4, 0, 3) {
        Ok(inst) => {
            println!("    Decoded as: {}", inst.format_with_version(3));
        }
        Err(e) => println!("    Error: {e}"),
    }

    println!("  In Version 5:");
    match Instruction::decode(&memory4, 0, 5) {
        Ok(inst) => {
            println!("    Decoded as: {}", inst.format_with_version(5));
        }
        Err(e) => println!("    Error: {e}"),
    }
}
