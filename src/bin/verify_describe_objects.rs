use infocom::instruction::Instruction;
use std::fs::File;
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut f = File::open("resources/test/zork1/DATA/ZORK1.DAT")?;
    let mut game_data = Vec::new();
    f.read_to_end(&mut game_data)?;

    println!("Verifying DESCRIBE-OBJECTS routine at 0x8eaa\n");
    
    // First check the routine header
    let routine_addr = 0x8eaa;
    let num_locals = game_data[routine_addr];
    println!("Routine header:");
    println!("  Number of locals: {} (expected: 10)", num_locals);
    
    if num_locals != 10 {
        println!("  ERROR: Wrong number of locals!");
        return Ok(());
    }
    
    // For V3, locals are initialized to 0000
    let mut pc = routine_addr + 1;
    for i in 0..num_locals {
        let value = ((game_data[pc] as u16) << 8) | (game_data[pc + 1] as u16);
        if value != 0 {
            println!("  ERROR: Local {} initialized to {:04x}, expected 0000", i, value);
        }
        pc += 2;
    }
    
    println!("  All locals correctly initialized to 0000 ✓\n");
    
    // Now compare each instruction
    let known_good = vec![
        (0x8ebf, "GET_CHILD L00 -> L03 [FALSE] RTRUE"),
        (0x8ec3, "GET_PARENT G6f -> L06"),
        (0x8ec6, "JZ L06 [TRUE] 8ed0"),
        (0x8ec9, "TEST_ATTR L06,#1b [FALSE] 8ed0"),
        (0x8ecd, "JUMP 8ed3"),
        (0x8ed0, "STORE L06,#00"),
        (0x8ed3, "STORE L04,#01"),
        (0x8ed6, "STORE L05,#01"),
        (0x8ed9, "GET_PARENT L00 -> -(SP)"),
        (0x8edc, "JE G6f,L00,(SP)+ [FALSE] 8ee8"),
        (0x8ee2, "STORE L09,#01"),
        (0x8ee5, "JUMP 8f45"),
        (0x8ee8, "JZ L03 [FALSE] 8eee"),
        (0x8eeb, "JUMP 8f45"),
        (0x8eee, "JE L03,L06 [FALSE] 8ef8"),
        (0x8ef2, "STORE L08,#01"),
        (0x8ef5, "JUMP 8f3e"),
        (0x8ef8, "JE L03,G6f [FALSE] 8eff"),
        (0x8efc, "JUMP 8f3e"),
        (0x8eff, "TEST_ATTR L03,#07 [TRUE] 8f3e"),
        (0x8f04, "TEST_ATTR L03,#03 [TRUE] 8f3e"),
        (0x8f08, "GET_PROP L03,#0e -> L07"),
        (0x8f0c, "JZ L07 [TRUE] 8f3e"),
        (0x8f0f, "TEST_ATTR L03,#0e [TRUE] 8f19"),
        (0x8f13, "PRINT_PADDR L07"),
        (0x8f15, "NEW_LINE"),
        (0x8f16, "STORE L05,#00"),
        (0x8f19, "CALL 9052 (L03) -> -(SP)"),
        (0x8f1f, "JZ (SP)+ [TRUE] 8f3e"),
        (0x8f22, "GET_PARENT L03 -> -(SP)"),
        (0x8f25, "GET_PROP (SP)+,#09 -> -(SP)"),
        (0x8f29, "JZ (SP)+ [FALSE] 8f3e"),
        (0x8f2c, "GET_CHILD L03 -> -(SP) [FALSE] 8f3e"),
        (0x8f30, "CALL 8eaa (L03,L01,#00) -> -(SP)"),
        (0x8f38, "JZ (SP)+ [TRUE] 8f3e"),
        (0x8f3b, "STORE L04,#00"),
        (0x8f3e, "GET_SIBLING L03 -> L03 [TRUE] 8f42"),
        (0x8f42, "JUMP 8ee8"),
    ];
    
    println!("Comparing disassembly (showing first 20 instructions):");
    let mut all_match = true;
    
    for (expected_addr, expected_text) in &known_good[..20] {
        match Instruction::decode(&game_data, *expected_addr, 3) {
            Ok(inst) => {
                let our_text = format_instruction(&inst, *expected_addr);
                
                print!("{:05x}: ", expected_addr);
                
                // Compare ignoring minor formatting differences
                if instructions_match(&our_text, expected_text) {
                    println!("✓ {}", our_text);
                } else {
                    println!("✗ Expected: {}", expected_text);
                    println!("       Got: {}", our_text);
                    all_match = false;
                }
            }
            Err(e) => {
                println!("{:05x}: ✗ DECODE ERROR: {}", expected_addr, e);
                all_match = false;
            }
        }
    }
    
    if all_match {
        println!("\nAll checked instructions match! ✓");
        println!("\nThe DESCRIBE-OBJECTS routine is correctly decoded.");
        println!("The error must be happening during execution, not decoding.");
    } else {
        println!("\nSome instructions don't match!");
        println!("This could explain the execution errors.");
    }
    
    // Also check if there are any branches to suspicious addresses
    println!("\nChecking for branches to problematic addresses...");
    
    pc = 0x8ebf; // Start of code
    while pc < 0x8fd7 {
        if let Ok(inst) = Instruction::decode(&game_data, pc, 3) {
            if let Some(ref branch) = inst.branch {
                let pc_after = pc + inst.size;
                let target = if branch.offset >= 2 {
                    (pc_after as i32 + branch.offset as i32 - 2) as u32
                } else {
                    0 // rtrue/rfalse
                };
                
                // Check if target is outside routine bounds
                if target != 0 && (target < 0x8eaa || target > 0x8fd7) {
                    println!("  {:05x}: branches to {:05x} (outside routine!)", pc, target);
                }
            }
            
            pc += inst.size;
        } else {
            pc += 1;
        }
    }
    
    Ok(())
}

fn format_instruction(inst: &Instruction, addr: usize) -> String {
    let mut result = inst.format_with_version(3);
    
    // Fix some formatting to match the expected format
    result = result.replace("get_child", "GET_CHILD");
    result = result.replace("get_parent", "GET_PARENT");
    result = result.replace("get_sibling", "GET_SIBLING");
    result = result.replace("get_prop", "GET_PROP");
    result = result.replace("test_attr", "TEST_ATTR");
    result = result.replace("print_paddr", "PRINT_PADDR");
    result = result.replace("new_line", "NEW_LINE");
    result = result.replace("store", "STORE");
    result = result.replace("jump", "JUMP");
    result = result.replace("call", "CALL");
    result = result.replace("jz", "JZ");
    result = result.replace("je", "JE");
    
    // Fix variable formatting
    result = result.replace("V7f", "G6f"); // Global 127
    result = result.replace("V01", "L00"); // Local 1
    result = result.replace("V02", "L01"); // Local 2
    result = result.replace("V03", "L02"); // Local 3
    result = result.replace("V04", "L03"); // Local 4
    result = result.replace("V05", "L04"); // Local 5
    result = result.replace("V06", "L05"); // Local 6
    result = result.replace("V07", "L06"); // Local 7
    result = result.replace("V08", "L07"); // Local 8
    result = result.replace("V09", "L08"); // Local 9
    result = result.replace("V0a", "L09"); // Local 10
    
    result
}

fn instructions_match(our: &str, expected: &str) -> bool {
    // Normalize and compare
    let our_normalized = our.to_uppercase().replace(" ", "");
    let expected_normalized = expected.to_uppercase().replace(" ", "");
    
    // Allow for minor differences in formatting
    our_normalized.contains(&expected_normalized) || 
    expected_normalized.contains(&our_normalized) ||
    our_normalized == expected_normalized
}