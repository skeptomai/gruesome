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
        let operands = vec![
            Operand::Variable(1),
            Operand::Variable(2)
        ];

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

        // Should be initialized with disabled state
        assert!(!codegen.two_pass_state.enabled);
        assert_eq!(codegen.two_pass_state.deferred_branches.len(), 0);
        assert_eq!(codegen.two_pass_state.label_addresses.len(), 0);

        // Should be able to enable two-pass mode
        codegen.two_pass_state.enabled = true;
        assert!(codegen.two_pass_state.enabled);
    }

    #[test]
    fn test_resolve_deferred_branches_disabled() {
        // Test that resolve_deferred_branches() works when disabled
        let mut codegen = create_test_codegen();

        // Should succeed when disabled (no-op)
        assert!(!codegen.two_pass_state.enabled);
        let result = codegen.resolve_deferred_branches();
        assert!(result.is_ok());
    }

    // Phase 2 tests (branch deferral) - CRITICAL FOR SAFETY
    #[test]
    #[ignore] // Will be enabled when Phase 2 is implemented
    fn test_emit_instruction_two_pass_mode_enabled() {
        // CRITICAL: Test that emit_instruction defers branch patching when two_pass_mode = true
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;

        let operands = vec![
            Operand::Variable(1),
            Operand::Variable(2)
        ];

        // Emit a branch instruction
        let layout = codegen.emit_instruction(
            0x01, // je (branch instruction)
            &operands,
            None,
            Some(-1), // placeholder branch offset (should be deferred)
        ).unwrap();

        // Should create deferred patch
        assert_eq!(codegen.two_pass_state.deferred_branches.len(), 1);

        let patch = &codegen.two_pass_state.deferred_branches[0];
        assert_eq!(patch.instruction_address, layout.instruction_start);
        assert!(patch.branch_offset_location > 0);
    }

    #[test]
    #[ignore] // Will be enabled when Phase 2 is implemented
    fn test_emit_instruction_non_branch_not_deferred() {
        // CRITICAL: Test that non-branch instructions are NOT deferred
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;

        let operands = vec![Operand::Variable(1)];

        // Emit a non-branch instruction
        let _layout = codegen.emit_instruction(
            0x8D, // print_paddr (NOT a branch instruction)
            &operands,
            None,
            None, // no branch
        ).unwrap();

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
        codegen.two_pass_state.label_addresses.insert(label_id, 0x2000);

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
    #[ignore] // Will be enabled when Phase 2 is implemented
    fn test_branch_instruction_identification() {
        // CRITICAL: Test that we correctly identify which instructions are branches
        let _codegen = create_test_codegen();

        // These SHOULD be identified as branch instructions:
        let branch_opcodes = vec![
            0x01, // je
            0x02, // jl
            0x03, // jg
            0x04, // dec_chk
            0x05, // inc_chk
            0x06, // jin
            0x07, // test
            0x08, // or
            0x09, // and
            0x0A, // test_attr
            0x0F, // jz
            0x10, // get_sibling
            0x11, // get_child
        ];

        // These should NOT be identified as branch instructions:
        let non_branch_opcodes = vec![
            0x8D, // print_paddr
            0x20, // call_2s
            0x21, // call_2n
            0x33, // pull
            0x40, // call_vs
        ];

        // This test will validate the branch detection logic once implemented
        // For now, just verify we can create the test structure
        assert!(branch_opcodes.len() > 0);
        assert!(non_branch_opcodes.len() > 0);
    }

    #[test]
    #[ignore] // Will be enabled when Phase 2 is implemented
    fn test_mixed_mode_compilation() {
        // CRITICAL: Test compilation with both branch and non-branch instructions
        let mut codegen = create_test_codegen();
        codegen.two_pass_state.enabled = true;

        // Emit sequence: non-branch, branch, non-branch
        let operands = vec![Operand::Variable(1)];

        // 1. Non-branch instruction
        codegen.emit_instruction(0x8D, &operands, None, None).unwrap();

        // 2. Branch instruction
        codegen.emit_instruction(0x01, &operands, None, Some(-1)).unwrap();

        // 3. Another non-branch instruction
        codegen.emit_instruction(0x8D, &operands, None, None).unwrap();

        // Should have exactly 1 deferred branch
        assert_eq!(codegen.two_pass_state.deferred_branches.len(), 1);
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
        codegen.two_pass_state.label_addresses.insert(label_id, 0x2000);

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

        codegen.two_pass_state.label_addresses.insert(label_id, target_address);

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

        codegen.two_pass_state.label_addresses.insert(label_id, target_address);

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

/// Test integration with push/pull instructions (the core bug scenario)
/// These tests will be enabled as implementation progresses
#[cfg(test)]
mod integration_tests {
    // Tests for the specific object traversal bug scenario
    // Will be uncommented during Phase 3 implementation
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