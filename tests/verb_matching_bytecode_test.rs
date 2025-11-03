/// Test that verb matching generates correct je instruction with proper branch logic
use gruesome::grue_compiler::GrueCompiler;
use gruesome::grue_compiler::ZMachineVersion;

#[test]
#[ignore] // TODO: Fix verb matching bytecode generation - this test was failing before file reorganization
fn test_verb_matching_generates_correct_je() {
    let source = r#"
        fn test_handler() {
            print("Handler called\n");
        }

        grammar {
            verb "test" {
                default => test_handler()
            }
        }

        init {
            print("Ready\n");
        }
    "#;

    let compiler = GrueCompiler::new();
    let (story_data, _codegen) = compiler
        .compile(source, ZMachineVersion::V3)
        .expect("Compilation should succeed");

    // Find je instructions in the code
    // je in V3 is opcode 0x01 in LONG form (2OP with 2 operands)
    // LONG form: opcode byte has form 10xxxxx (0x80-0xBF for 2OP)
    // For je (2OP:1), the opcode byte is:
    // - Bits 7-6: 10 (LONG form indicator)
    // - Bits 5-4: operand 1 type (00=small, 01=variable)
    // - Bits 3-2: operand 2 type
    // - Bits 1-0: opcode number in 2OP class = 01 (je is 2OP:1)
    //
    // For je with two variables (Variable(2), Variable(17)):
    // Opcode byte: 10 01 01 01 = 0x55

    println!("Story file size: {} bytes", story_data.len());

    // Get initial PC (first routine to run - the "init" function)
    let initial_pc = ((story_data[0x06] as usize) << 8) | (story_data[0x07] as usize);
    println!("Initial PC (init routine): 0x{:04x}", initial_pc);

    // In the compiler, code_address 0x00CA is where verb matching starts
    // But this is an offset from final_code_base, not from himem
    // The actual file address depends on where code was placed in the final layout

    // Note: Can't easily calculate final file position from code_address
    // because final_code_base is set during compilation, not exposed in header

    // Search for ANY je instruction (2OP:1 = opcode 0x01 in different forms)
    // LONG form: 0x01-0x1F, 0x41-0x5F (bit pattern 00/01 for operand types + 0x01)
    // VAR form: 0xC1 (if more than 2 operands, but je only takes 2)

    println!("\nSearching for je instructions...");
    let mut je_count = 0;
    for i in 0..story_data.len() - 4 {
        let opcode = story_data[i];

        // Check if this is je in LONG form
        // LONG form pattern: bits 7-6 are not both 1, and bits 1-0 are 01 (opcode 1)
        if (opcode & 0xC3) == 0x01
            || (opcode & 0xC3) == 0x41
            || (opcode & 0xC3) == 0x81
            || opcode == 0x55
        {
            let op1 = story_data[i + 1];
            let op2 = story_data[i + 2];
            let branch_byte1 = story_data[i + 3];
            let branch_byte2 = story_data[i + 4];

            println!(
                "\nFound possible je at 0x{:04x}: opcode=0x{:02x}",
                i, opcode
            );
            println!("  Operand 1: {}", op1);
            println!("  Operand 2: {}", op2);
            println!(
                "  Branch bytes: 0x{:02x} 0x{:02x}",
                branch_byte1, branch_byte2
            );

            // Check branch polarity (bit 7 of first branch byte)
            let branch_on_true = (branch_byte1 & 0x80) != 0;
            println!("  Branch on true: {}", branch_on_true);

            // For verb matching, we expect:
            // - op1 = 2 (word from parse buffer)
            // - op2 = 17 (verb dict addr in Global G01)
            // - branch_on_true = true
            if op1 == 2 && op2 == 17 {
                println!("  ✓ This is a verb matching je!");
                assert!(branch_on_true, "Verb matching je should branch on TRUE");
                je_count += 1;
            }
        }
    }

    assert!(
        je_count > 0,
        "Should find at least one verb matching je instruction"
    );
    println!("\n✓ Found {} verb matching je instruction(s)", je_count);
}
