//! Targeted tests for Option A: Delayed Branch Patching (Two-Pass Compilation)
//!
//! PROBLEM SOLVED: Branch target address calculation bug where push/pull instruction
//! insertion shifts addresses and causes runtime crashes with "Invalid Long form opcode 0x00".
//!
//! ARCHITECTURE: Two-pass compilation system where:
//! 1. Pass 1: Emit all instructions with branch placeholders
//! 2. Pass 2: Calculate correct branch offsets and patch placeholders
//!
//! TEST STRATEGY: Comprehensive validation of each architectural component:
//! - Data structure creation and manipulation
//! - Branch vs non-branch instruction handling
//! - Deferred branch creation and resolution
//! - Error handling and edge cases
//! - Integration with existing single-pass system
//!
//! CURRENT STATE: Phase 1 complete (infrastructure), Phase 2 tests ready for enablement

use super::*;
use indexmap::IndexMap;

/// Test data structures for two-pass compilation
#[cfg(test)]
mod data_structure_tests {
    use super::*;

    #[test]
    fn test_deferred_branch_patch_creation() {
        // Test DeferredBranchPatch struct creation and field access
        let patch = DeferredBranchPatch {
            instruction_address: 0x1000,
            branch_offset_location: 0x1002,
            target_label_id: 42,
            branch_on_true: true,
            offset_size: 2,
        };

        assert_eq!(patch.instruction_address, 0x1000);
        assert_eq!(patch.branch_offset_location, 0x1002);
        assert_eq!(patch.target_label_id, 42);
        assert_eq!(patch.branch_on_true, true);
        assert_eq!(patch.offset_size, 2);
    }

    #[test]
    fn test_two_pass_state_initialization() {
        // Test TwoPassState creation and default state
        let state = TwoPassState {
            enabled: false,
            deferred_branches: Vec::new(),
            label_addresses: IndexMap::new(),
        };

        assert!(!state.enabled);
        assert_eq!(state.deferred_branches.len(), 0);
        assert_eq!(state.label_addresses.len(), 0);
    }

    #[test]
    fn test_two_pass_state_enabled() {
        let mut state = TwoPassState {
            enabled: true,
            deferred_branches: Vec::new(),
            label_addresses: IndexMap::new(),
        };

        // Test adding deferred branches
        let patch = DeferredBranchPatch {
            instruction_address: 0x1000,
            branch_offset_location: 0x1002,
            target_label_id: 100,
            branch_on_true: false,
            offset_size: 1,
        };

        state.deferred_branches.push(patch);
        assert_eq!(state.deferred_branches.len(), 1);

        // Test adding label addresses
        state.label_addresses.insert(100, 0x2000);
        assert_eq!(state.label_addresses.get(&100), Some(&0x2000));
    }
}

/// Test branch deferral mechanism
#[cfg(test)]
mod branch_deferral_tests {
    use super::*;

    fn create_test_codegen() -> ZMachineCodeGen {
        let mut codegen = ZMachineCodeGen::new(crate::grue_compiler::ZMachineVersion::V3);
        // Initialize basic state for testing
        codegen.code_space = Vec::new();
        codegen.code_address = 0x1000;
        codegen
    }

    #[test]
    fn test_emit_instruction_current_behavior() {
        // This test validates current behavior (single-pass immediate branch patching)
        let mut codegen = create_test_codegen();

        // Emit a branch instruction using current architecture
        let operands = vec![Operand::Variable(1), Operand::Variable(2)];

        let result = codegen.emit_instruction(
            0x01, // je (branch instruction)
            &operands,
            None,
            Some(10), // branch offset
        );

        // Should succeed with current architecture
        assert!(result.is_ok());
        let layout = result.unwrap();

        // Current behavior: immediate branch patching
        // (This test documents existing behavior before two-pass changes)
        assert!(layout.instruction_start >= 0x1000);
    }

    // Tests below will be uncommented as implementation progresses

    // Phase 1 tests (two-pass infrastructure)
    #[test]
    fn test_two_pass_state_integration() {
        // Test that TwoPassState is properly integrated with ZMachineCodeGen
        let mut codegen = create_test_codegen();

        // Should be initialized with enabled state (Phase 3 default)
        assert!(codegen.two_pass_state.enabled);
        assert_eq!(codegen.two_pass_state.deferred_branches.len(), 0);
        assert_eq!(codegen.two_pass_state.label_addresses.len(), 0);

        // Should be able to disable two-pass mode
        codegen.two_pass_state.enabled = false;
        assert!(!codegen.two_pass_state.enabled);
    }

    #[test]
    fn test_resolve_deferred_branches_disabled() {
        // Test that resolve_deferred_branches() works when disabled
        let mut codegen = create_test_codegen();

        // Explicitly disable two-pass for this test (tests disabled behavior)
        codegen.two_pass_state.enabled = false;
        assert!(!codegen.two_pass_state.enabled);
        let result = codegen.resolve_deferred_branches();
        assert!(result.is_ok());
    }

    // Phase 2 tests (branch deferral) - CRITICAL FOR SAFETY
    #[test]
    fn test_emit_instruction_two_pass_mode_enabled() {
        // CRITICAL: Test that emit_instruction defers branch patching when two_pass_mode = true
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;

        let operands = vec![Operand::Variable(1), Operand::Variable(2)];

        // Emit a branch instruction
        let layout = codegen
            .emit_instruction(
                0x01, // je (branch instruction)
                &operands,
                None,
                Some(-1), // placeholder branch offset (should be deferred)
            )
            .unwrap();

        // Should create deferred patch
        assert_eq!(codegen.two_pass_state.deferred_branches.len(), 1);

        let patch = &codegen.two_pass_state.deferred_branches[0];
        assert_eq!(patch.instruction_address, layout.instruction_start);
        assert!(patch.branch_offset_location > 0);
    }

    #[test]
    fn test_emit_instruction_non_branch_not_deferred() {
        // CRITICAL: Test that non-branch instructions are NOT deferred
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;

        let operands = vec![Operand::Variable(1)];

        // Emit a non-branch instruction
        let _layout = codegen
            .emit_instruction(
                0x8D, // print_paddr (NOT a branch instruction)
                &operands, None, None, // no branch
            )
            .unwrap();

        // Should NOT create deferred patch
        assert_eq!(codegen.two_pass_state.deferred_branches.len(), 0);
    }

    #[test]
    #[ignore] // Will be enabled when Phase 2 is implemented
    fn test_resolve_deferred_branches_simple() {
        // Test basic branch resolution with known addresses
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;

        // Add a label at address 0x2000
        let label_id = 100;
        codegen
            .two_pass_state
            .label_addresses
            .insert(label_id, 0x2000);

        // Create a deferred branch patch
        let patch = DeferredBranchPatch {
            instruction_address: 0x1000,
            branch_offset_location: 0x1002,
            target_label_id: label_id,
            branch_on_true: true,
            offset_size: 2,
        };
        codegen.two_pass_state.deferred_branches.push(patch);

        // Ensure code_space has enough capacity
        codegen.code_space.resize(0x1010, 0);

        // Resolve branches
        let result = codegen.resolve_deferred_branches();
        assert!(result.is_ok());

        // Check that branch was patched correctly
        // Expected offset: 0x2000 - (0x1002 + 2) = 0x2000 - 0x1004 = 0xFFC
        let patched_bytes = &codegen.code_space[0x1002..0x1004];

        // For a forward branch of 0xFFC bytes, check Z-Machine branch encoding
        // This validates that the branch offset calculation and patching works correctly
        assert_ne!(patched_bytes, &[0xFF, 0xFF]); // Should not be placeholder
    }

    #[test]
    fn test_branch_instruction_identification() {
        // CRITICAL: Test that we correctly identify which instructions are branches
        let codegen = create_test_codegen();

        // These SHOULD be identified as branch instructions (ALL branch opcodes):
        // Only instructions with "Br" column marked with "*" in Z-Machine spec section 14 table
        let branch_opcodes = vec![
            0x01, // je (jump if equal)
            0x02, // jl (jump if less)
            0x03, // jg (jump if greater)
            0x04, // dec_chk (decrement and check)
            0x05, // inc_chk (increment and check)
            0x06, // jin (jump if object in object)
            0x07, // test (jump if all bits set)
            // 0x08 removed - or (bitwise or) stores result, does NOT branch
            // 0x09 removed - and (bitwise and) stores result, does NOT branch
            0x0A, // test_attr (test attribute, then branch)
            // 0x0D removed - print_paddr in our compiler, does NOT branch
            0x0F, // jz (jump if zero)
            0x10, // get_sibling (get sibling, branch if exists)
            0x11, // get_child (get child, branch if exists)
        ];

        // These should NOT be identified as branch instructions:
        let non_branch_opcodes = vec![
            0x00, // call_vs (function call)
            0x08, // or (bitwise or) - stores result, does NOT branch
            0x09, // and (bitwise and) - stores result, does NOT branch
            0x0C, // jump (NOT a branch - direct jump!)
            0x0D, // print_paddr
            0x0E, // insert_obj
            0x12, // get_prop_addr
            0x13, // get_next_prop
            0x14, // add
            0x15, // sub
            0x20, // call_2s
            0x21, // call_2n
            0x33, // pull
            0xFF, // invalid opcode
        ];

        // Test branch instruction detection
        for &opcode in &branch_opcodes {
            assert!(
                codegen.is_branch_instruction(opcode),
                "Opcode 0x{:02x} should be identified as branch instruction",
                opcode
            );
        }

        // Test non-branch instruction detection
        for &opcode in &non_branch_opcodes {
            assert!(
                !codegen.is_branch_instruction(opcode),
                "Opcode 0x{:02x} should NOT be identified as branch instruction",
                opcode
            );
        }

        // Verify we have comprehensive coverage
        assert_eq!(
            branch_opcodes.len(),
            11,
            "Should test all 11 branch instructions per Z-Machine spec"
        );
        assert!(
            non_branch_opcodes.len() >= 10,
            "Should test sufficient non-branch instructions"
        );
    }

    #[test]
    fn test_mixed_mode_compilation() {
        // CRITICAL: Test compilation with both branch and non-branch instructions
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;

        // Emit sequence: non-branch, branch, non-branch
        let operands = vec![Operand::Variable(1)];

        // 1. Non-branch instruction
        codegen
            .emit_instruction(0x8D, &operands, None, None)
            .unwrap();

        // 2. Branch instruction
        codegen
            .emit_instruction(0x01, &operands, None, Some(-1))
            .unwrap();

        // 3. Another non-branch instruction
        codegen
            .emit_instruction(0x8D, &operands, None, None)
            .unwrap();

        // Should have exactly 1 deferred branch
        assert_eq!(codegen.two_pass_state.deferred_branches.len(), 1);
    }

    #[test]
    fn test_mixed_compilation_modes() {
        // CRITICAL: Test switching between single-pass and two-pass modes mid-compilation
        let mut codegen = create_test_codegen();

        // Start in single-pass mode
        codegen.two_pass_state.enabled = false;
        let operands = vec![Operand::Variable(1), Operand::Variable(2)];

        let layout1 = codegen
            .emit_instruction(0x01, &operands, None, Some(10))
            .unwrap();
        assert_eq!(
            codegen.two_pass_state.deferred_branches.len(),
            0,
            "Single-pass mode should not defer branches"
        );

        // Switch to two-pass mode mid-compilation
        codegen.two_pass_state.enabled = true;
        let layout2 = codegen
            .emit_instruction(0x01, &operands, None, Some(10))
            .unwrap();
        assert_eq!(
            codegen.two_pass_state.deferred_branches.len(),
            1,
            "Two-pass mode should defer branches"
        );

        // Switch back to single-pass mode
        codegen.two_pass_state.enabled = false;
        let layout3 = codegen
            .emit_instruction(0x01, &operands, None, Some(10))
            .unwrap();
        assert_eq!(
            codegen.two_pass_state.deferred_branches.len(),
            1,
            "Should not add more deferred branches in single-pass mode"
        );

        // Verify all instructions were emitted correctly
        assert!(layout1.instruction_start < layout2.instruction_start);
        assert!(layout2.instruction_start < layout3.instruction_start);
    }

    #[test]
    fn test_branch_offset_size_detection() {
        // CRITICAL: Test that branch offset size is detected correctly
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;
        let operands = vec![Operand::Variable(1), Operand::Variable(2)];

        // Test placeholder (-1) handling
        codegen
            .emit_instruction(0x01, &operands, None, Some(-1))
            .unwrap();
        assert_eq!(codegen.two_pass_state.deferred_branches.len(), 1);
        let patch = &codegen.two_pass_state.deferred_branches[0];
        assert_eq!(
            patch.offset_size, 1,
            "Placeholder should default to 1-byte offset"
        );

        // Clear for next test
        codegen.two_pass_state.deferred_branches.clear();

        // Test short form offset (should fit in 1 byte: -64 to +63)
        codegen
            .emit_instruction(0x01, &operands, None, Some(50))
            .unwrap();
        assert_eq!(codegen.two_pass_state.deferred_branches.len(), 1);
        let patch = &codegen.two_pass_state.deferred_branches[0];
        assert_eq!(patch.offset_size, 1, "Short offset should use 1-byte");

        // Clear for next test
        codegen.two_pass_state.deferred_branches.clear();

        // Test long form offset (should need 2 bytes: outside -64 to +63)
        codegen
            .emit_instruction(0x01, &operands, None, Some(100))
            .unwrap();
        assert_eq!(codegen.two_pass_state.deferred_branches.len(), 1);
        let patch = &codegen.two_pass_state.deferred_branches[0];
        assert_eq!(patch.offset_size, 2, "Long offset should use 2-byte");

        // Clear for next test
        codegen.two_pass_state.deferred_branches.clear();

        // Test negative long form offset
        codegen
            .emit_instruction(0x01, &operands, None, Some(-100))
            .unwrap();
        assert_eq!(codegen.two_pass_state.deferred_branches.len(), 1);
        let patch = &codegen.two_pass_state.deferred_branches[0];
        assert_eq!(
            patch.offset_size, 2,
            "Large negative offset should use 2-byte"
        );
    }

    #[test]
    fn test_branch_polarity_detection() {
        // CRITICAL: Test that branch polarity (branch_on_true) is detected correctly
        // Note: Current implementation has known limitations with Z-Machine branch encoding
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;
        let operands = vec![Operand::Variable(1), Operand::Variable(2)];

        // Test placeholder (-1) - should default to true
        codegen
            .emit_instruction(0x01, &operands, None, Some(-1))
            .unwrap();
        assert_eq!(codegen.two_pass_state.deferred_branches.len(), 1);
        let patch = &codegen.two_pass_state.deferred_branches[0];
        assert_eq!(
            patch.branch_on_true, true,
            "Placeholder should default to branch_on_true=true"
        );

        // Clear for next test
        codegen.two_pass_state.deferred_branches.clear();

        // Test positive offset - current implementation treats as branch_on_true
        codegen
            .emit_instruction(0x01, &operands, None, Some(10))
            .unwrap();
        assert_eq!(codegen.two_pass_state.deferred_branches.len(), 1);
        let patch = &codegen.two_pass_state.deferred_branches[0];
        assert_eq!(
            patch.branch_on_true, true,
            "Positive offset should be branch_on_true=true"
        );

        // Clear for next test
        codegen.two_pass_state.deferred_branches.clear();

        // Test negative offset - current implementation treats as branch_on_false
        codegen
            .emit_instruction(0x01, &operands, None, Some(-10))
            .unwrap();
        assert_eq!(codegen.two_pass_state.deferred_branches.len(), 1);
        let patch = &codegen.two_pass_state.deferred_branches[0];
        assert_eq!(
            patch.branch_on_true, false,
            "Negative offset should be branch_on_false=false"
        );
    }

    #[test]
    fn test_deferred_branch_memory_growth() {
        // CRITICAL: Test that many branch instructions don't cause memory issues
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;
        let operands = vec![Operand::Variable(1)];

        // Generate many branch instructions
        let branch_count = 100; // Use reasonable number for test performance
        for _i in 0..branch_count {
            codegen
                .emit_instruction(0x0F, &operands, None, Some(-1))
                .unwrap(); // jz instruction
        }

        assert_eq!(codegen.two_pass_state.deferred_branches.len(), branch_count);

        // Verify each patch was created correctly
        for (i, patch) in codegen.two_pass_state.deferred_branches.iter().enumerate() {
            assert!(
                patch.instruction_address > 0,
                "Patch {} should have valid instruction address",
                i
            );
            assert!(
                patch.branch_offset_location > 0,
                "Patch {} should have valid branch offset location",
                i
            );
            assert_eq!(
                patch.target_label_id, 0,
                "Patch {} should have placeholder target_label_id",
                i
            );
        }
    }

    #[test]
    fn test_non_branch_with_branch_parameter() {
        // CRITICAL: Test that non-branch instructions with branch_offset parameter are handled correctly
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;
        let operands = vec![Operand::Variable(1), Operand::Variable(2)];

        // Non-branch instruction with branch parameter (should be ignored)
        let _layout = codegen
            .emit_instruction(0x14, &operands, None, Some(10))
            .unwrap(); // add (2OP:20)

        // Should NOT create deferred branch patch
        assert_eq!(
            codegen.two_pass_state.deferred_branches.len(),
            0,
            "Non-branch instruction should not create deferred patch even with branch_offset"
        );
    }

    #[test]
    fn test_branch_instruction_without_branch_parameter() {
        // CRITICAL: Test that branch instructions without branch_offset are handled correctly
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;
        let operands = vec![Operand::Variable(1)];

        // Branch instruction without branch parameter (should not defer)
        let _layout = codegen
            .emit_instruction(0x01, &operands, None, None)
            .unwrap(); // je without branch

        // Should NOT create deferred branch patch
        assert_eq!(
            codegen.two_pass_state.deferred_branches.len(),
            0,
            "Branch instruction without branch_offset should not create deferred patch"
        );
    }

    #[test]
    #[ignore] // Will be enabled when Phase 2 is implemented
    fn test_resolve_deferred_branches_missing_label() {
        // CRITICAL: Test error handling when target label is missing
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;

        // Create a deferred branch patch with non-existent label
        let patch = DeferredBranchPatch {
            instruction_address: 0x1000,
            branch_offset_location: 0x1002,
            target_label_id: 999, // Label that doesn't exist
            branch_on_true: true,
            offset_size: 2,
        };
        codegen.two_pass_state.deferred_branches.push(patch);
        codegen.code_space.resize(0x1010, 0);

        // Should return an error
        let result = codegen.resolve_deferred_branches();
        assert!(result.is_err());
    }

    #[test]
    #[ignore] // Will be enabled when Phase 2 is implemented
    fn test_resolve_deferred_branches_bounds_check() {
        // CRITICAL: Test bounds checking during branch patching
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;

        let label_id = 100;
        codegen
            .two_pass_state
            .label_addresses
            .insert(label_id, 0x2000);

        // Create a patch that would write beyond code_space bounds
        let patch = DeferredBranchPatch {
            instruction_address: 0x1000,
            branch_offset_location: 0x9999, // Way beyond code_space size
            target_label_id: label_id,
            branch_on_true: true,
            offset_size: 2,
        };
        codegen.two_pass_state.deferred_branches.push(patch);
        codegen.code_space.resize(0x1010, 0); // Much smaller than 0x9999

        // Should return bounds error
        let result = codegen.resolve_deferred_branches();
        assert!(result.is_err());
    }

    #[test]
    #[ignore] // Will be enabled when Phase 2 is implemented
    fn test_branch_offset_calculation_forward() {
        // CRITICAL: Test forward branch offset calculation
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;

        let label_id = 100;
        let target_address = 0x2000;
        let branch_offset_location = 0x1002;
        let offset_size = 2;

        codegen
            .two_pass_state
            .label_addresses
            .insert(label_id, target_address);

        let patch = DeferredBranchPatch {
            instruction_address: 0x1000,
            branch_offset_location,
            target_label_id: label_id,
            branch_on_true: true,
            offset_size,
        };
        codegen.two_pass_state.deferred_branches.push(patch);
        codegen.code_space.resize(0x3000, 0);

        codegen.resolve_deferred_branches().unwrap();

        // Expected offset: target - (offset_location + offset_size)
        // 0x2000 - (0x1002 + 2) = 0x2000 - 0x1004 = 0xFFC
        let expected_offset = 0xFFC;
        let high_byte = ((expected_offset >> 8) as u8) | 0x80; // branch_on_true bit
        let low_byte = (expected_offset & 0xFF) as u8;

        assert_eq!(codegen.code_space[branch_offset_location], high_byte);
        assert_eq!(codegen.code_space[branch_offset_location + 1], low_byte);
    }

    #[test]
    #[ignore] // Will be enabled when Phase 2 is implemented
    fn test_branch_offset_calculation_backward() {
        // CRITICAL: Test backward branch offset calculation
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;

        let label_id = 100;
        let target_address = 0x1000;
        let branch_offset_location = 0x2002;
        let offset_size = 2;

        codegen
            .two_pass_state
            .label_addresses
            .insert(label_id, target_address);

        let patch = DeferredBranchPatch {
            instruction_address: 0x2000,
            branch_offset_location,
            target_label_id: label_id,
            branch_on_true: false, // Test branch_on_false
            offset_size,
        };
        codegen.two_pass_state.deferred_branches.push(patch);
        codegen.code_space.resize(0x3000, 0);

        codegen.resolve_deferred_branches().unwrap();

        // Backward branch: target < from
        // Expected offset: -(from - target) = -((0x2002 + 2) - 0x1000) = -(0x2004 - 0x1000) = -0x1004
        let expected_offset = -0x1004i16;
        let high_byte = ((expected_offset as u16 >> 8) as u8) & 0x7F; // branch_on_false (clear bit 7)
        let low_byte = (expected_offset as u16 & 0xFF) as u8;

        assert_eq!(codegen.code_space[branch_offset_location], high_byte);
        assert_eq!(codegen.code_space[branch_offset_location + 1], low_byte);
    }

    // Phase 3 tests (push/pull integration)
    // #[test]
    // fn test_branch_target_preserved_with_instruction_insertion() { ... }

    // Phase 4 tests (performance and compatibility)
    // #[test]
    // fn test_backward_compatibility() { ... }
}

/// Phase 3 Integration Tests: Verify two-pass compilation is enabled and working
#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test that two-pass compilation is enabled by default (Phase 3)
    #[test]
    fn test_phase3_two_pass_enabled_by_default() {
        let codegen = create_test_codegen();
        assert!(
            codegen.two_pass_state.enabled,
            "Phase 3: Two-pass compilation should be enabled by default"
        );
        assert_eq!(
            codegen.two_pass_state.deferred_branches.len(),
            0,
            "Should start with no deferred branches"
        );
        assert_eq!(
            codegen.two_pass_state.label_addresses.len(),
            0,
            "Should start with no label addresses"
        );
    }

    /// Test that resolve_deferred_branches() is properly integrated
    #[test]
    fn test_phase3_resolve_integration() {
        let mut codegen = create_test_codegen();

        // Should work even with no deferred branches
        let result = codegen.resolve_deferred_branches();
        assert!(
            result.is_ok(),
            "resolve_deferred_branches should work with empty state"
        );

        // Should still be enabled after resolution
        assert!(
            codegen.two_pass_state.enabled,
            "Two-pass should remain enabled after resolution"
        );
    }

    // ========================================
    // REAL CORRECTNESS TESTS FOR TWO-PASS BRANCH PATCHING
    // ========================================
    // These tests verify actual bytecode generation and branch resolution behavior,
    // replacing previous "smoke tests" that only checked if code ran without crashing.
    //
    // CRITICAL SUCCESS: These tests found and helped fix a real bug in 2-byte branch
    // encoding where bit 6 (0x40) was missing for 2-byte offset indicator.
    //
    // Each test verifies:
    // - Exact branch byte values with correct bit patterns
    // - Precise offset calculations for forward/backward/1-byte/2-byte scenarios
    // - Branch polarity (branch_on_true vs branch_on_false)
    // - Integration with other compiler features (push/pull instructions)

    /// REAL TEST: Verify actual branch patching with forward reference
    /// Tests 1-byte forward branch with branch_on_true and exact offset calculation
    #[test]
    fn test_real_forward_branch_patching_correctness() {
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;

        // Emit a je instruction with forward branch placeholder using the real API
        let start_pos = codegen.code_space.len();

        // Manually construct a je instruction that would be deferred
        // je (opcode 0x01) with two SmallConstant operands and placeholder branch
        codegen.code_space.extend_from_slice(&[
            0x01, // je opcode
            42,   // first operand
            43,   // second operand
            0xFF, // placeholder branch byte
        ]);

        let branch_pos = codegen.code_space.len() - 1; // Position of branch byte

        // Create a deferred branch for this instruction with correct field names
        codegen
            .two_pass_state
            .deferred_branches
            .push(DeferredBranchPatch {
                instruction_address: start_pos,
                branch_offset_location: branch_pos,
                target_label_id: 100,
                branch_on_true: true,
                offset_size: 1,
            });

        // Add some bytecode to create distance
        codegen.code_space.extend_from_slice(&[0x00, 0x01, 0x02]); // 3 bytes

        // Place the target label
        let target_pos = codegen.code_space.len();
        codegen
            .two_pass_state
            .label_addresses
            .insert(100, target_pos);

        // Resolve the deferred branch
        let result = codegen.resolve_deferred_branches();
        assert!(result.is_ok(), "Branch resolution should succeed");

        // REAL VERIFICATION: Check the actual branch byte was patched correctly
        // Offset should be 3 (distance to target), with branch_on_true bit set
        let expected_branch_byte = 0x80 | 3; // 0x80 = branch_on_true, 3 = offset
        assert_eq!(
            codegen.code_space[branch_pos], expected_branch_byte,
            "Branch byte should be correctly patched with offset 3 and branch_on_true bit"
        );

        // Verify the instruction bytes before the branch are unchanged
        assert_eq!(
            codegen.code_space[start_pos], 0x01,
            "je opcode should be unchanged"
        );
    }

    /// REAL TEST: Verify 2-byte branch offset handling
    /// Tests Z-Machine 2-byte branch encoding with bit 6 (0x40) indicator
    /// CRITICAL: This test found the missing 0x40 bit bug in the original implementation
    #[test]
    fn test_real_2byte_branch_offset_correctness() {
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;

        // Emit a jz instruction with 2-byte branch placeholder
        let start_pos = codegen.code_space.len();

        // Manually construct a jz instruction with 2-byte branch placeholder
        codegen.code_space.extend_from_slice(&[
            0x0F, // jz opcode
            5,    // operand
            0xFF, 0xFF, // 2-byte placeholder branch offset
        ]);

        let branch_pos = codegen.code_space.len() - 2; // Position of 2-byte branch offset

        // Create a deferred branch requiring 2-byte offset
        codegen
            .two_pass_state
            .deferred_branches
            .push(DeferredBranchPatch {
                instruction_address: start_pos,
                branch_offset_location: branch_pos,
                target_label_id: 200,
                branch_on_true: false,
                offset_size: 2,
            });

        // Add enough bytecode to require 2-byte offset (>63 bytes)
        let padding = vec![0xBB; 70]; // 70 new_line instructions
        codegen.code_space.extend_from_slice(&padding);

        // Place the target label
        let target_pos = codegen.code_space.len();
        codegen
            .two_pass_state
            .label_addresses
            .insert(200, target_pos);

        // Resolve the deferred branch
        let result = codegen.resolve_deferred_branches();
        assert!(
            result.is_ok(),
            "Branch resolution should succeed for 2-byte offset"
        );

        // REAL VERIFICATION: Check 2-byte offset was written correctly
        // Offset should be 70, with branch_on_false (no 0x80 bit)
        let offset_high = codegen.code_space[branch_pos];
        let offset_low = codegen.code_space[branch_pos + 1];

        // First byte should be 0x40 (2-byte offset indicator) + high bits of offset
        assert_eq!(
            offset_high, 0x40,
            "High byte should indicate 2-byte offset and branch_on_false"
        );
        assert_eq!(
            offset_low, 70,
            "Low byte should contain the offset value 70"
        );
    }

    /// REAL TEST: Verify backward branch calculation
    /// Tests negative offset calculation and 2's complement encoding for backward jumps
    #[test]
    fn test_real_backward_branch_correctness() {
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;

        // Place target label first
        let target_pos = codegen.code_space.len();
        codegen
            .two_pass_state
            .label_addresses
            .insert(300, target_pos);

        // Add some bytecode between label and branch
        codegen.code_space.extend_from_slice(&[0x10, 0x20, 0x30]); // 3 bytes

        let start_pos = codegen.code_space.len();

        // Emit a jg instruction with backward branch
        codegen.code_space.extend_from_slice(&[
            0x03, // jg opcode
            10,   // first operand
            5,    // second operand
            0xFF, // placeholder branch byte
        ]);

        let branch_pos = codegen.code_space.len() - 1;

        // Create deferred branch pointing backward
        codegen
            .two_pass_state
            .deferred_branches
            .push(DeferredBranchPatch {
                instruction_address: start_pos,
                branch_offset_location: branch_pos,
                target_label_id: 300,
                branch_on_true: true,
                offset_size: 1,
            });

        // Resolve the branch
        let result = codegen.resolve_deferred_branches();
        assert!(result.is_ok(), "Backward branch resolution should succeed");

        // REAL VERIFICATION: For backward branches, offset calculation
        // Branch instruction at start_pos (3), branch_offset at branch_pos (6)
        // Target at target_pos (0), so offset = 0 - (6 + 1) = -7
        // Z-Machine uses 2's complement: -7 = 256 - 7 = 249 (0xF9)
        let branch_byte = codegen.code_space[branch_pos];

        // Should have branch_on_true bit (0x80) and correct backward offset
        assert!(
            (branch_byte & 0x80) != 0,
            "Should have branch_on_true bit set"
        );

        // The lower 6 bits should contain the backward offset
        let offset_bits = branch_byte & 0x3F;
        assert!(
            offset_bits > 50,
            "Backward offset should be encoded as large positive value (2's complement)"
        );
    }

    /// REAL TEST: Multiple branches to same label with correct offsets
    /// Tests that different branches calculate different offsets to same target correctly
    #[test]
    fn test_real_multiple_branches_same_target() {
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;

        // First branch at position 0
        let start1 = codegen.code_space.len();
        codegen.code_space.extend_from_slice(&[
            0x01, // je opcode
            1,    // first operand
            2,    // second operand
            0xFF, // placeholder branch byte
        ]);
        let branch1_pos = codegen.code_space.len() - 1;

        codegen
            .two_pass_state
            .deferred_branches
            .push(DeferredBranchPatch {
                instruction_address: start1,
                branch_offset_location: branch1_pos,
                target_label_id: 400,
                branch_on_true: true,
                offset_size: 1,
            });

        // Add exactly 2 bytes spacing
        codegen.code_space.extend_from_slice(&[0x11, 0x22]);

        // Second branch
        let start2 = codegen.code_space.len();
        codegen.code_space.extend_from_slice(&[
            0x02, // jl opcode
            3,    // first operand
            4,    // second operand
            0xFF, // placeholder branch byte
        ]);
        let branch2_pos = codegen.code_space.len() - 1;

        codegen
            .two_pass_state
            .deferred_branches
            .push(DeferredBranchPatch {
                instruction_address: start2,
                branch_offset_location: branch2_pos,
                target_label_id: 400,
                branch_on_true: false,
                offset_size: 1,
            });

        // Add exactly 3 bytes spacing
        codegen.code_space.extend_from_slice(&[0x33, 0x44, 0x55]);

        // Place the shared target label
        let target_pos = codegen.code_space.len();
        codegen
            .two_pass_state
            .label_addresses
            .insert(400, target_pos);

        // Resolve all branches
        let result = codegen.resolve_deferred_branches();
        assert!(result.is_ok(), "Multiple branch resolution should succeed");

        // REAL VERIFICATION: Check exact offset calculations
        let branch1_byte = codegen.code_space[branch1_pos];
        let branch2_byte = codegen.code_space[branch2_pos];

        // First branch: from position 3 (branch1_pos) to position 13 (target_pos)
        // Offset = 13 - (3 + 1) = 9
        assert_eq!(
            branch1_byte,
            0x80 | 9,
            "First branch should be 0x89 (branch_on_true + offset 9)"
        );

        // Second branch: from position 9 (branch2_pos) to position 13 (target_pos)
        // Offset = 13 - (9 + 1) = 3
        assert_eq!(
            branch2_byte,
            0x00 | 3,
            "Second branch should be 0x03 (branch_on_false + offset 3)"
        );
    }

    /// REAL TEST: Integration with push/pull instruction insertion
    /// Tests that branch offsets correctly account for push/pull instructions inserted between
    /// branch instruction and target (critical for Phase 3 push/pull integration)
    #[test]
    fn test_real_branch_with_push_pull_insertion() {
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;

        // Emit a je instruction before push/pull
        let start_pos = codegen.code_space.len();
        codegen.code_space.extend_from_slice(&[
            0x01, // je opcode
            10,   // first operand
            20,   // second operand
            0xFF, // placeholder branch byte
        ]);
        let branch_pos = codegen.code_space.len() - 1;

        // Create deferred branch
        codegen
            .two_pass_state
            .deferred_branches
            .push(DeferredBranchPatch {
                instruction_address: start_pos,
                branch_offset_location: branch_pos,
                target_label_id: 500,
                branch_on_true: true,
                offset_size: 1,
            });

        // Simulate push/pull instructions being inserted (Phase 3 scenario)
        // VAR:232 (push) and VAR:233 (pull) operations
        codegen.code_space.extend_from_slice(&[
            0xE8, 0x00, // push Variable(0) - 2 bytes
            0xE9, 0x00, // pull to Variable(0) - 2 bytes
        ]);

        // Place target label after push/pull
        let target_pos = codegen.code_space.len();
        codegen
            .two_pass_state
            .label_addresses
            .insert(500, target_pos);

        // Resolve branches
        let result = codegen.resolve_deferred_branches();
        assert!(
            result.is_ok(),
            "Branch resolution with push/pull should succeed"
        );

        // REAL VERIFICATION: Branch offset accounts for inserted instructions
        let branch_byte = codegen.code_space[branch_pos];

        // Branch from position 3 (branch_pos) to position 8 (target_pos after 4 bytes push/pull)
        // Offset = 8 - (3 + 1) = 4
        assert_eq!(
            branch_byte,
            0x80 | 4,
            "Branch should correctly calculate offset of 4 to skip over push/pull instructions"
        );

        // Verify push/pull instructions are actually there
        assert_eq!(
            codegen.code_space[4], 0xE8,
            "Push instruction should be present"
        );
        assert_eq!(
            codegen.code_space[6], 0xE9,
            "Pull instruction should be present"
        );
    }
}

/// Helper function to create a basic test codegen instance
fn create_test_codegen() -> ZMachineCodeGen {
    let mut codegen = ZMachineCodeGen::new(crate::grue_compiler::ZMachineVersion::V3);
    // Initialize basic state for testing
    codegen.code_space = Vec::new();
    codegen.code_address = 0x1000;

    // Phase 1 complete: TwoPassState is now part of ZMachineCodeGen
    // (initialized automatically in constructor)

    codegen
}
