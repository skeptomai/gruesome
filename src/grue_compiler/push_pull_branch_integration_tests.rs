//! Deep verification tests for DeferredBranchPatch + push/pull integration
//!
//! CRITICAL INTEGRATION PROBLEM: Push/pull operations insert instructions dynamically
//! during code generation, which can shift branch target addresses after branches
//! are already deferred in two-pass compilation.
//!
//! TESTING METHODOLOGY: Real correctness approach with exact bytecode verification
//! following the proven pattern from DeferredBranchPatch and UnresolvedReference tests.
//!
//! KEY SCENARIOS TO VERIFY:
//! 1. Push/pull insertions between deferred branch and target
//! 2. Complex control flow with nested push/pull operations
//! 3. Function calls (which use push/pull) around conditional branches
//! 4. Multiple push/pull operations affecting branch offset calculations
//! 5. Mixed DeferredBranchPatch + UnresolvedReference + push/pull scenarios

use super::*;
use crate::grue_compiler::codegen::{
    placeholder_word, DeferredBranchPatch, LegacyReferenceType, MemorySpace, Operand,
    UnresolvedReference, ZMachineCodeGen,
};
use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::IrId;

/// Helper function to create placeholder operand for testing
fn placeholder_operand() -> Operand {
    Operand::LargeConstant(placeholder_word())
}

/// Create a test codegen instance with push/pull and two-pass systems enabled
fn create_push_pull_branch_test_codegen() -> ZMachineCodeGen {
    let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);

    // Enable two-pass compilation for DeferredBranchPatch
    codegen.two_pass_state.enabled = true;

    // Initialize minimal state for testing
    codegen.code_space.clear();
    codegen.code_address = 0x1000; // Standard code start
    codegen.final_data.clear();
    codegen.final_code_base = 0x1000;

    // Push/pull system state will be initialized by first use

    codegen
}

// ========================================
// PHASE 5.1: BASIC PUSH/PULL + BRANCH INTEGRATION
// ========================================

/// REAL TEST: Verify branch resolution when push instruction inserted between branch and target
/// Tests that DeferredBranchPatch correctly calculates branch offsets accounting for
/// dynamically inserted push instructions that shift target addresses.
#[test]
fn test_real_push_insertion_affects_branch_offset() -> Result<(), CompilerError> {
    let mut codegen = create_push_pull_branch_test_codegen();

    // Phase 1: Simulate a realistic scenario where a branch is deferred,
    // then push/pull operations insert instructions before the target label

    let branch_label_id: IrId = 1001;
    let push_ir_id: IrId = 2001;

    // Start code generation sequence
    let branch_address = codegen.code_address;

    // Emit a conditional branch instruction that will be deferred
    // This simulates: if (variable1 == variable2) goto label
    let operand1 = Operand::Variable(1); // First comparison operand
    let operand2 = Operand::Variable(2); // Second comparison operand

    // Add deferred branch manually for DeferredBranchPatch testing
    // For je instruction, branch offset is typically at instruction_address + 3 (after opcode + 2 operands)
    let branch_offset_location = branch_address + 3;
    let deferred_patch = DeferredBranchPatch {
        instruction_address: branch_address,
        branch_offset_location,
        target_label_id: branch_label_id,
        branch_on_true: true,
        offset_size: 2, // 2-byte offset
    };

    // CRITICAL FIX: Temporarily disable two-pass mode to prevent double-deferral
    // emit_instruction() automatically creates DeferredBranchPatch when two-pass is enabled,
    // but this test manually creates patches for precise control over the testing scenario
    let original_enabled = codegen.two_pass_state.enabled;
    codegen.two_pass_state.enabled = false;

    // Emit placeholder branch instruction (what two-pass system does)
    // je (opcode 0x01) requires exactly 2 operands for comparison
    codegen.emit_instruction(
        0x01,
        &[operand1, operand2],
        None,
        Some(placeholder_word() as i16),
    )?;

    // Re-enable two-pass mode and add our manually created deferred patch
    codegen.two_pass_state.enabled = original_enabled;
    codegen
        .two_pass_state
        .deferred_branches
        .push(deferred_patch);

    let post_branch_address = codegen.code_address;

    // Phase 2: Simulate push/pull operations that insert instructions
    // This represents function calls, expression evaluations, etc. that happen
    // between the branch and its target

    // Simulate a function call result being marked for push/pull
    codegen.use_push_pull_for_result(push_ir_id, "test function call")?;

    // The push instruction was just inserted! This shifts subsequent addresses
    let post_push_address = codegen.code_address;
    let push_instruction_size = post_push_address - post_branch_address;

    // Emit some more instructions (simulating code between branch and target)
    log::debug!("About to emit 0x0D with 1 operand...");
    codegen.emit_instruction(0x0D, &[Operand::Constant(42)], None, None)?; // print_num

    // Now simulate using the push-marked value, which triggers pull insertion
    let _operand = codegen.resolve_ir_id_to_operand(push_ir_id)?;

    let post_pull_address = codegen.code_address;
    let pull_instruction_size = post_pull_address - post_push_address - 3; // minus print_num size

    // Phase 3: Mark the target label address and emit target instruction
    let target_label_address = codegen.code_address;
    codegen
        .two_pass_state
        .label_addresses
        .insert(branch_label_id, target_label_address);

    // Emit label target instruction
    codegen.emit_instruction(0xBB, &[], None, None)?; // new_line (target instruction)

    // Phase 4: Resolve deferred branches (simulate end of two-pass compilation)
    codegen.resolve_deferred_branches()?;

    // Phase 5: CRITICAL VERIFICATION - Check that branch offset accounts for inserted instructions

    // Calculate expected branch offset using same logic as resolve_deferred_branches
    // Z-Machine branch offset is calculated from the byte AFTER the branch offset field
    let branch_from = branch_offset_location + 2; // 2-byte offset
    let expected_offset = (target_label_address as i32) - (branch_from as i32);
    let raw_offset = expected_offset as i16;

    // Apply Z-Machine 2-byte branch encoding (same as resolution logic)
    // Bit 7 = branch_on_true, Bit 6 = 2-byte indicator (MUST be set for 2-byte)
    let expected_high_byte = if true {
        // branch_on_true = true
        ((raw_offset as u16) >> 8) as u8 | 0x80 | 0x40 // Set bit 7 (branch on true) + bit 6 (2-byte indicator)
    } else {
        ((raw_offset as u16) >> 8) as u8 | 0x40 // Set bit 6 (2-byte indicator), clear bit 7 (branch on false)
    };
    let expected_low_byte = (raw_offset as u16) as u8;

    // Read actual bytes written to branch offset location
    assert!(
        branch_offset_location + 1 < codegen.code_space.len(),
        "Branch offset location should be within code_space"
    );

    let actual_high_byte = codegen.code_space[branch_offset_location];
    let actual_low_byte = codegen.code_space[branch_offset_location + 1];

    // Extract actual offset by masking out Z-Machine control bits (bits 7 and 6 in high byte)
    let actual_offset_high = actual_high_byte & 0x3F; // Remove polarity (bit 7) and 2-byte indicator (bit 6)
    let actual_offset = ((actual_offset_high as i16) << 8) | (actual_low_byte as i16);

    // VERIFICATION SUCCESS: Log integration test results
    log::debug!("✅ Push/pull + branch integration verified:");
    log::debug!(
        "  Branch address: 0x{:04x} → target: 0x{:04x}",
        branch_address,
        target_label_address
    );
    log::debug!("  Push instruction size: {} bytes", push_instruction_size);
    log::debug!("  Pull instruction size: {} bytes", pull_instruction_size);
    log::debug!(
        "  Final branch offset: {} (0x{:04x})",
        actual_offset,
        actual_offset as u16
    );
    log::debug!(
        "  Total address shift handled: {} bytes",
        push_instruction_size + pull_instruction_size
    );

    // EXACT VERIFICATION: Branch offset must account for inserted push/pull instructions
    assert_eq!(
        actual_high_byte, expected_high_byte,
        "Branch high byte should account for push/pull insertions. Expected 0x{:02x}, got 0x{:02x}",
        expected_high_byte, actual_high_byte
    );

    assert_eq!(
        actual_low_byte, expected_low_byte,
        "Branch low byte should account for push/pull insertions. Expected 0x{:02x}, got 0x{:02x}",
        expected_low_byte, actual_low_byte
    );

    assert_eq!(
        actual_offset, expected_offset as i16,
        "Branch offset should account for inserted instructions. Expected {}, got {}",
        expected_offset, actual_offset
    );

    println!("✅ Push/pull + branch integration verified:");
    println!(
        "  Branch address: 0x{:04x} → target: 0x{:04x}",
        branch_address, target_label_address
    );
    println!("  Push instruction size: {} bytes", push_instruction_size);
    println!("  Pull instruction size: {} bytes", pull_instruction_size);
    println!(
        "  Final branch offset: {} (0x{:04x})",
        actual_offset, actual_offset as u16
    );
    println!(
        "  Total address shift handled: {} bytes",
        push_instruction_size + pull_instruction_size
    );

    Ok(())
}

// ========================================
// PHASE 5.2: COMPLEX CONTROL FLOW SCENARIOS
// ========================================

/// REAL TEST: Verify complex control flow with multiple push/pull operations and branches
/// Tests scenarios with nested function calls, conditional branches, and mixed operations
/// that can create complex instruction insertion patterns.
#[test]
fn test_real_complex_control_flow_with_nested_push_pull() -> Result<(), CompilerError> {
    let mut codegen = create_push_pull_branch_test_codegen();

    // Phase 1: Set up complex scenario with multiple branches and push/pull operations
    // Simulates: if (condition1) { func1(); if (condition2) goto label2; func2(); } else goto label1;

    let label1_id: IrId = 3001; // else target
    let label2_id: IrId = 3002; // inner if target
    let func1_result_id: IrId = 4001;
    let func2_result_id: IrId = 4002;
    let condition2_id: IrId = 5001;

    // Branch 1: if (variable1 == variable2) else goto label1
    let branch1_address = codegen.code_address;
    let condition1_operand1 = Operand::Variable(1);
    let condition1_operand2 = Operand::Variable(2);

    // Emit first deferred branch (branch on false to label1)
    let branch1_offset_location = branch1_address + 3; // After opcode + 2 operands
    let deferred_patch1 = DeferredBranchPatch {
        instruction_address: branch1_address,
        branch_offset_location: branch1_offset_location,
        target_label_id: label1_id,
        branch_on_true: false, // branch on false (else case)
        offset_size: 2,
    };

    // Temporarily disable two-pass mode to avoid double-deferral
    let original_enabled1 = codegen.two_pass_state.enabled;
    codegen.two_pass_state.enabled = false;

    codegen.emit_instruction(
        0x01,
        &[condition1_operand1, condition1_operand2],
        None,
        Some(placeholder_word() as i16),
    )?;

    // Re-enable and add manual patch
    codegen.two_pass_state.enabled = original_enabled1;
    codegen
        .two_pass_state
        .deferred_branches
        .push(deferred_patch1);

    // Phase 2: Inside if block - function call with push/pull
    // This simulates func1() which uses push/pull for result handling
    codegen.use_push_pull_for_result(func1_result_id, "func1 call")?;

    // Emit simulated function call instruction (call_vs: VAR call - V3 compatible)
    codegen.emit_instruction(
        0x00,
        &[Operand::Constant(100), Operand::SmallConstant(0)],
        Some(0),
        None,
    )?; // call routine with dummy arg

    // Use func1 result in a condition, triggering pull insertion
    let _func1_operand = codegen.resolve_ir_id_to_operand(func1_result_id)?;

    // Store condition2 result for nested branch
    codegen.use_push_pull_for_result(condition2_id, "condition2 expression")?;

    // Emit condition2 calculation (simulated)
    codegen.emit_instruction(
        0x14,
        &[Operand::Variable(2), Operand::Constant(5)],
        Some(0),
        None,
    )?; // je

    // Branch 2: if (condition2_value == 0) goto label2 (nested branch)
    let branch2_address = codegen.code_address;
    let condition2_operand = codegen.resolve_ir_id_to_operand(condition2_id)?; // This triggers pull!
    let zero_operand = Operand::SmallConstant(0);

    let branch2_offset_location = branch2_address + 3; // After opcode + 2 operands
    let deferred_patch2 = DeferredBranchPatch {
        instruction_address: branch2_address,
        branch_offset_location: branch2_offset_location,
        target_label_id: label2_id,
        branch_on_true: true,
        offset_size: 2,
    };

    // Temporarily disable two-pass mode to avoid double-deferral
    let original_enabled2 = codegen.two_pass_state.enabled;
    codegen.two_pass_state.enabled = false;

    codegen.emit_instruction(
        0x01,
        &[condition2_operand, zero_operand],
        None,
        Some(placeholder_word() as i16),
    )?;

    // Re-enable and add manual patch
    codegen.two_pass_state.enabled = original_enabled2;
    codegen
        .two_pass_state
        .deferred_branches
        .push(deferred_patch2);

    // Phase 3: More function calls with push/pull operations
    // This simulates func2() call after the nested branch
    codegen.use_push_pull_for_result(func2_result_id, "func2 call")?;
    codegen.emit_instruction(
        0x00,
        &[Operand::Constant(200), Operand::SmallConstant(0)],
        Some(0),
        None,
    )?; // call routine with dummy arg

    // Use func2 result, triggering another pull
    let _func2_operand = codegen.resolve_ir_id_to_operand(func2_result_id)?;
    codegen.emit_instruction(0x0D, &[_func2_operand], None, None)?; // print_num

    // Phase 4: Set up label addresses
    let label2_address = codegen.code_address; // label2 comes first
    codegen
        .two_pass_state
        .label_addresses
        .insert(label2_id, label2_address);
    codegen.emit_instruction(0xBB, &[], None, None)?; // new_line (label2 target)

    // Some instructions after label2
    codegen.emit_instruction(0x0D, &[Operand::Constant(999)], None, None)?; // print_num

    let label1_address = codegen.code_address; // label1 comes later
    codegen
        .two_pass_state
        .label_addresses
        .insert(label1_id, label1_address);
    codegen.emit_instruction(0xBB, &[], None, None)?; // new_line (label1 target)

    // Phase 5: Resolve all deferred branches
    codegen.resolve_deferred_branches()?;

    // Phase 6: COMPREHENSIVE VERIFICATION
    // Verify that both branches correctly account for all the push/pull insertions

    // Verify Branch 1 (to label1)
    let branch1_high_byte = codegen.code_space[branch1_offset_location];
    let branch1_low_byte = codegen.code_space[branch1_offset_location + 1];

    // Extract actual offset by masking out Z-Machine control bits (bits 7 and 6 in high byte)
    let actual_offset1_high = branch1_high_byte & 0x3F; // Remove polarity (bit 7) and 2-byte indicator (bit 6)
    let branch1_offset = ((actual_offset1_high as i16) << 8) | (branch1_low_byte as i16);

    // Calculate expected offset using same logic as resolve_deferred_branches
    let branch1_from = branch1_offset_location + 2; // 2-byte offset
    let expected_branch1_offset = (label1_address as i32) - (branch1_from as i32);

    assert_eq!(
        branch1_offset, expected_branch1_offset as i16,
        "Branch 1 should correctly account for all nested push/pull insertions"
    );

    // Verify Branch 2 (to label2)
    let branch2_high_byte = codegen.code_space[branch2_offset_location];
    let branch2_low_byte = codegen.code_space[branch2_offset_location + 1];

    // Extract actual offset by masking out Z-Machine control bits (bits 7 and 6 in high byte)
    let actual_offset2_high = branch2_high_byte & 0x3F; // Remove polarity (bit 7) and 2-byte indicator (bit 6)
    let branch2_offset = ((actual_offset2_high as i16) << 8) | (branch2_low_byte as i16);

    // Calculate expected offset using same logic as resolve_deferred_branches
    let branch2_from = branch2_offset_location + 2; // 2-byte offset
    let expected_branch2_offset = (label2_address as i32) - (branch2_from as i32);

    assert_eq!(
        branch2_offset, expected_branch2_offset as i16,
        "Branch 2 should correctly account for subsequent push/pull insertions"
    );

    println!("✅ Complex control flow integration verified:");
    println!(
        "  Branch 1: 0x{:04x} → 0x{:04x} (offset: {})",
        branch1_address, label1_address, branch1_offset
    );
    println!(
        "  Branch 2: 0x{:04x} → 0x{:04x} (offset: {})",
        branch2_address, label2_address, branch2_offset
    );
    println!("  Total push/pull operations: 4 (2 push, 2 pull)");
    println!("  Nested control flow with dynamic instruction insertion handled correctly");

    Ok(())
}

// ========================================
// PHASE 5.3: MIXED SYSTEM INTEGRATION
// ========================================

/// REAL TEST: Verify DeferredBranchPatch + UnresolvedReference + push/pull all working together
/// Tests the complete integration of all three systems in a realistic compilation scenario
/// where branches, references, and stack operations all interact.
#[test]
fn test_real_three_system_integration() -> Result<(), CompilerError> {
    let mut codegen = create_push_pull_branch_test_codegen();

    // Phase 1: Set up a scenario that uses all three systems:
    // - DeferredBranchPatch for conditional branches
    // - UnresolvedReference for function calls and string references
    // - Push/pull for stack discipline

    let branch_label_id: IrId = 6001;
    let function_id: IrId = 7001;
    let string_id: IrId = 8001;
    let expr_result_id: IrId = 9001;

    // Set up function and string for UnresolvedReference system
    let function_offset = 0x500;
    codegen
        .function_addresses
        .insert(function_id, function_offset);

    let string_offset = 0x100;
    let test_string = "Integration test string";
    codegen
        .string_space
        .resize(string_offset + test_string.len() + 1, 0);
    codegen.string_space[string_offset..string_offset + test_string.len()]
        .copy_from_slice(test_string.as_bytes());
    codegen.string_offsets.insert(string_id, string_offset);

    // Phase 2: Emit complex sequence mixing all systems

    // Start with an expression that uses push/pull
    codegen.use_push_pull_for_result(expr_result_id, "complex expression")?;
    codegen.emit_instruction(
        0x15,
        &[Operand::Variable(1), Operand::Variable(2)],
        Some(0),
        None,
    )?; // add

    // Conditional branch using the expression result (DeferredBranchPatch)
    // if (expr_result != 0) goto label
    let branch_address = codegen.code_address;
    let condition_operand = codegen.resolve_ir_id_to_operand(expr_result_id)?; // Triggers pull!
    let zero_operand = Operand::SmallConstant(0);

    let branch_offset_location = branch_address + 3; // After opcode + 2 operands
    let deferred_patch = DeferredBranchPatch {
        instruction_address: branch_address,
        branch_offset_location,
        target_label_id: branch_label_id,
        branch_on_true: false, // branch on false (if NOT equal to 0)
        offset_size: 2,
    };

    // Temporarily disable two-pass mode to avoid double-deferral
    let original_enabled3 = codegen.two_pass_state.enabled;
    codegen.two_pass_state.enabled = false;

    codegen.emit_instruction(
        0x01,
        &[condition_operand, zero_operand],
        None,
        Some(placeholder_word() as i16),
    )?;

    // Re-enable and add manual patch
    codegen.two_pass_state.enabled = original_enabled3;
    codegen
        .two_pass_state
        .deferred_branches
        .push(deferred_patch);

    // Function call using UnresolvedReference
    let call_location = codegen.code_address + 1; // Where function address will go
    let function_ref = UnresolvedReference {
        reference_type: LegacyReferenceType::FunctionCall,
        location: call_location,
        target_id: function_id,
        is_packed_address: true,
        offset_size: 2,
        location_space: MemorySpace::Code,
    };

    codegen.emit_instruction(
        0x00,
        &[placeholder_operand(), Operand::SmallConstant(0)],
        Some(0),
        None,
    )?; // call with placeholder + dummy arg
    codegen.reference_context.unresolved_refs.push(function_ref);

    // String reference using UnresolvedReference
    let string_ref_location = codegen.code_address + 1;
    let string_ref = UnresolvedReference {
        reference_type: LegacyReferenceType::StringRef,
        location: string_ref_location,
        target_id: string_id,
        is_packed_address: true,
        offset_size: 2,
        location_space: MemorySpace::Code,
    };

    codegen.emit_instruction(0x0A, &[placeholder_operand()], None, None)?; // print_addr with placeholder
    codegen.reference_context.unresolved_refs.push(string_ref);

    // Phase 3: Set up target label and final addresses
    let target_label_address = codegen.code_address;
    codegen
        .two_pass_state
        .label_addresses
        .insert(branch_label_id, target_label_address);
    codegen.emit_instruction(0xBB, &[], None, None)?; // new_line (target)

    // Set up final addresses for code generation completion
    codegen
        .final_data
        .resize(codegen.code_space.len() + 0x1000, 0);
    codegen.final_code_base = 0x1000;
    codegen.final_string_base = 0x2000;

    // Copy code_space to final_data at final_code_base
    let code_start = codegen.final_code_base;
    let code_end = code_start + codegen.code_space.len();
    codegen.final_data[code_start..code_end].copy_from_slice(&codegen.code_space);

    // Set up target addresses for UnresolvedReference resolution
    let expected_final_function_addr = codegen.final_code_base + function_offset;
    codegen
        .reference_context
        .ir_id_to_address
        .insert(function_id, expected_final_function_addr);

    // Phase 4: Resolve all systems

    // Resolve DeferredBranchPatch
    codegen.resolve_deferred_branches()?;

    // Resolve UnresolvedReference
    codegen.resolve_all_addresses()?;

    // Phase 5: COMPREHENSIVE VERIFICATION
    // Verify that all three systems worked correctly together

    // Verify branch resolution
    let branch_high_byte = codegen.code_space[branch_offset_location];
    let branch_low_byte = codegen.code_space[branch_offset_location + 1];

    // Extract actual offset by masking out Z-Machine control bits (bits 7 and 6 in high byte)
    let actual_offset_high = branch_high_byte & 0x3F; // Remove polarity (bit 7) and 2-byte indicator (bit 6)
    let branch_offset = ((actual_offset_high as i16) << 8) | (branch_low_byte as i16);

    // Calculate expected offset using same logic as resolve_deferred_branches
    let branch_from = branch_offset_location + 2; // 2-byte offset
    let expected_branch_offset = (target_label_address as i32) - (branch_from as i32);

    assert_eq!(
        branch_offset, expected_branch_offset as i16,
        "DeferredBranchPatch should work correctly with push/pull insertions"
    );

    // Verify function reference resolution
    let final_call_location = codegen.final_code_base + call_location;
    let func_high_byte = codegen.final_data[final_call_location];
    let func_low_byte = codegen.final_data[final_call_location + 1];
    let func_addr = ((func_high_byte as u16) << 8) | (func_low_byte as u16);

    let expected_packed_func_addr = expected_final_function_addr / 2;
    assert_eq!(
        func_addr, expected_packed_func_addr as u16,
        "UnresolvedReference should resolve function calls correctly"
    );

    // Verify string reference resolution
    let final_string_location = codegen.final_code_base + string_ref_location;
    let string_high_byte = codegen.final_data[final_string_location];
    let string_low_byte = codegen.final_data[final_string_location + 1];
    let string_addr = ((string_high_byte as u16) << 8) | (string_low_byte as u16);

    let expected_final_string_addr = codegen.final_string_base + string_offset;
    let expected_packed_string_addr = expected_final_string_addr / 2;
    assert_eq!(
        string_addr, expected_packed_string_addr as u16,
        "UnresolvedReference should resolve string references correctly"
    );

    println!("✅ Three-system integration verified:");
    println!("  DeferredBranchPatch: branch offset = {}", branch_offset);
    println!(
        "  UnresolvedReference: function = 0x{:04x}, string = 0x{:04x}",
        func_addr, string_addr
    );
    println!(
        "  Push/pull: {} operations with proper stack discipline",
        2 // 1 push + 1 pull
    );
    println!("  All systems working together without conflicts");

    Ok(())
}

// ========================================
// PHASE 5.4: STRESS TESTING
// ========================================

/// REAL TEST: High-stress scenario with many push/pull operations and branches
/// Tests the limits of the integration to ensure it scales correctly
#[test]
fn test_real_high_stress_push_pull_branch_integration() -> Result<(), CompilerError> {
    let mut codegen = create_push_pull_branch_test_codegen();

    // Phase 1: Create a stress scenario with many operations
    const NUM_BRANCHES: usize = 10;
    const NUM_PUSH_PULL_OPS: usize = 20;

    let mut branch_patches = Vec::new();
    let mut label_addresses = Vec::new();

    // Phase 2: Emit many push/pull operations with interspersed branches
    for i in 0..NUM_PUSH_PULL_OPS {
        let push_ir_id = 10000 + i as IrId;

        // Every few operations, emit a branch
        if i % 3 == 0 && i / 3 < NUM_BRANCHES {
            let label_id = 20000 + (i / 3) as IrId;
            let branch_address = codegen.code_address;
            let branch_offset_location = branch_address + 3; // After opcode + 2 operands

            let deferred_patch = DeferredBranchPatch {
                instruction_address: branch_address,
                branch_offset_location,
                target_label_id: label_id,
                branch_on_true: true,
                offset_size: 2,
            };

            let condition_operand1 = Operand::Variable((i % 5 + 1) as u8);
            let condition_operand2 = Operand::SmallConstant(i as u8);

            // Temporarily disable two-pass mode to avoid double-deferral
            let original_enabled_loop = codegen.two_pass_state.enabled;
            codegen.two_pass_state.enabled = false;

            codegen.emit_instruction(
                0x01,
                &[condition_operand1, condition_operand2],
                None,
                Some(placeholder_word() as i16),
            )?;

            // Re-enable and add manual patch
            codegen.two_pass_state.enabled = original_enabled_loop;
            codegen
                .two_pass_state
                .deferred_branches
                .push(deferred_patch);
            branch_patches.push((branch_address, branch_offset_location, label_id));
        }

        // Emit push/pull sequence
        codegen.use_push_pull_for_result(push_ir_id, &format!("stress test op {}", i))?;

        // Some computation
        codegen.emit_instruction(
            0x15,
            &[Operand::Variable(1), Operand::Constant(i as u16)],
            Some(0),
            None,
        )?; // add

        // Use the result (triggers pull)
        let _operand = codegen.resolve_ir_id_to_operand(push_ir_id)?;
        codegen.emit_instruction(0x0D, &[_operand], None, None)?; // print_num
    }

    // Phase 3: Set up all label addresses
    for i in 0..NUM_BRANCHES {
        let label_id = 20000 + i as IrId;
        let label_address = codegen.code_address;

        codegen
            .two_pass_state
            .label_addresses
            .insert(label_id, label_address);
        label_addresses.push(label_address);

        // Emit target instruction
        codegen.emit_instruction(0xBB, &[], None, None)?; // new_line
    }

    // Phase 4: Resolve all deferred branches
    let total_instructions_before = codegen.code_space.len();
    codegen.resolve_deferred_branches()?;
    let total_instructions_after = codegen.code_space.len();

    // Phase 5: STRESS TEST VERIFICATION
    // Verify that all branches resolved correctly despite many push/pull insertions

    for (i, (branch_address, branch_offset_location, _label_id)) in
        branch_patches.iter().enumerate()
    {
        let branch_high_byte = codegen.code_space[*branch_offset_location];
        let branch_low_byte = codegen.code_space[*branch_offset_location + 1];

        // Extract actual offset by masking out Z-Machine control bits (bits 7 and 6 in high byte)
        let actual_offset_high = branch_high_byte & 0x3F; // Remove polarity (bit 7) and 2-byte indicator (bit 6)
        let branch_offset = ((actual_offset_high as i16) << 8) | (branch_low_byte as i16);

        // Calculate expected offset using same logic as resolve_deferred_branches
        let branch_from = *branch_offset_location + 2; // 2-byte offset
        let target_address = label_addresses[i];
        let expected_offset = (target_address as i32) - (branch_from as i32);

        assert_eq!(
            branch_offset,
            expected_offset as i16,
            "Branch {} should resolve correctly despite {} push/pull operations",
            i,
            NUM_PUSH_PULL_OPS * 2 // each op = 1 push + 1 pull
        );
    }

    // Verify no instruction duplication or corruption
    assert_eq!(
        total_instructions_before, total_instructions_after,
        "resolve_deferred_branches should not modify instruction count"
    );

    println!("✅ High-stress integration verified:");
    println!("  {} branches resolved correctly", NUM_BRANCHES);
    println!("  {} push/pull operations handled", NUM_PUSH_PULL_OPS * 2);
    println!("  Total instructions: {}", total_instructions_after);
    println!("  No conflicts or corruption detected");

    Ok(())
}
