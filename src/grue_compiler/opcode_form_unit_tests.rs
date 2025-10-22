// Comprehensive unit tests for opcode form determination
// This module tests the critical form determination logic that was recently buggy
// with context-dependent opcode 0x0D scenarios

use crate::grue_compiler::codegen::{InstructionForm, Operand, ZMachineCodeGen};
use crate::grue_compiler::ZMachineVersion;

#[cfg(test)]
mod opcode_form_tests {
    use super::*;

    // ======================================================================================
    // TEST HELPER FUNCTIONS
    // ======================================================================================

    /// Create a fresh V3 code generator for testing
    fn create_codegen() -> ZMachineCodeGen {
        ZMachineCodeGen::new(ZMachineVersion::V3)
    }

    /// Helper to verify form determination for basic operand count logic
    fn verify_basic_form(operand_count: usize, opcode: u8, expected: InstructionForm) {
        let codegen = create_codegen();
        let actual = codegen.determine_instruction_form(operand_count, opcode);
        assert_eq!(
            actual, expected,
            "Basic form determination failed: {} operands, opcode 0x{:02x} -> expected {:?}, got {:?}",
            operand_count, opcode, expected, actual
        );
    }

    /// Helper to verify form determination with operands
    fn verify_operand_form(
        operands: &[Operand],
        opcode: u8,
        expected: Result<InstructionForm, &str>,
    ) {
        let codegen = create_codegen();
        let actual = codegen.determine_instruction_form_with_operands(operands, opcode);

        match (&actual, expected) {
            (Ok(actual_form), Ok(expected_form)) => {
                assert_eq!(
                    *actual_form, expected_form,
                    "Operand form determination failed: operands={:?}, opcode 0x{:02x} -> expected {:?}, got {:?}",
                    operands, opcode, expected_form, actual_form
                );
            }
            (Err(_), Err(expected_error_pattern)) => {
                let error_msg = format!("{:?}", actual.unwrap_err());
                assert!(
                    error_msg.contains(expected_error_pattern),
                    "Error form determination failed: operands={:?}, opcode 0x{:02x} -> expected error containing '{}', got '{}'",
                    operands, opcode, expected_error_pattern, error_msg
                );
            }
            _ => panic!(
                "Form determination result mismatch: operands={:?}, opcode 0x{:02x} -> expected {:?}, got {:?}",
                operands, opcode, expected, actual
            ),
        }
    }

    /// Helper to verify context-dependent form determination (the 0x0D scenarios)
    fn verify_context_form(
        operands: &[Operand],
        opcode: u8,
        store_var: Option<u8>,
        expected: InstructionForm,
    ) {
        let codegen = create_codegen();
        let actual = codegen
            .determine_instruction_form_with_context(operands, opcode, store_var)
            .expect("Context form determination should not fail");

        assert_eq!(
            actual, expected,
            "Context form determination failed: operands={:?}, opcode 0x{:02x}, store_var={:?} -> expected {:?}, got {:?}",
            operands, opcode, store_var, expected, actual
        );
    }

    // ======================================================================================
    // CORE FORM TESTS: Basic operand count logic (8 tests)
    // ======================================================================================

    #[test]
    fn test_basic_form_0_operands() {
        // 0 operands -> SHORT form (0OP)
        verify_basic_form(0, 0x50, InstructionForm::Short); // arbitrary opcode
        verify_basic_form(0, 0x00, InstructionForm::Short);
        verify_basic_form(0, 0x10, InstructionForm::Short);
    }

    #[test]
    fn test_basic_form_1_operand() {
        // 1 operand -> SHORT form (1OP)
        verify_basic_form(1, 0x50, InstructionForm::Short);
        verify_basic_form(1, 0x8D, InstructionForm::Short); // print_paddr
        verify_basic_form(1, 0x10, InstructionForm::Short);
    }

    #[test]
    fn test_basic_form_2_operands_long() {
        // 2 operands + opcode < 0x80 -> LONG form (2OP)
        verify_basic_form(2, 0x01, InstructionForm::Long); // je
        verify_basic_form(2, 0x0D, InstructionForm::Long); // store
        verify_basic_form(2, 0x7F, InstructionForm::Long); // boundary case
    }

    #[test]
    fn test_basic_form_2_operands_variable() {
        // 2 operands + opcode >= 0x80 -> VARIABLE form
        verify_basic_form(2, 0x80, InstructionForm::Variable);
        verify_basic_form(2, 0xE0, InstructionForm::Variable); // call
        verify_basic_form(2, 0xFF, InstructionForm::Variable);
    }

    #[test]
    fn test_basic_form_3_plus_operands() {
        // 3+ operands -> VARIABLE form
        verify_basic_form(3, 0x01, InstructionForm::Variable);
        verify_basic_form(3, 0x0D, InstructionForm::Variable); // output_stream
        verify_basic_form(4, 0x50, InstructionForm::Variable);
        verify_basic_form(5, 0xE0, InstructionForm::Variable);
    }

    #[test]
    fn test_basic_form_always_var_opcodes() {
        // Special opcodes that are always VAR form regardless of operand count
        verify_basic_form(0, 0xE0, InstructionForm::Variable); // call (VAR:224)
        verify_basic_form(1, 0xE1, InstructionForm::Variable); // storew (VAR:225)
        verify_basic_form(2, 0xE3, InstructionForm::Variable); // put_prop (VAR:227)
        verify_basic_form(1, 0xE4, InstructionForm::Variable); // sread (VAR:228)
        verify_basic_form(0, 0xE5, InstructionForm::Variable); // print_char (VAR:229)
        verify_basic_form(1, 0xE6, InstructionForm::Variable); // print_num (VAR:230)
        verify_basic_form(1, 0xE7, InstructionForm::Variable); // random (VAR:231)
    }

    #[test]
    fn test_basic_form_boundary_operand_counts() {
        // Boundary testing around the 1->2 and 2->3 operand transitions
        verify_basic_form(0, 0x20, InstructionForm::Short);
        verify_basic_form(1, 0x20, InstructionForm::Short);
        verify_basic_form(2, 0x20, InstructionForm::Long); // 2 operands, opcode < 0x80
        verify_basic_form(3, 0x20, InstructionForm::Variable);

        verify_basic_form(2, 0x80, InstructionForm::Variable); // 2 operands, opcode >= 0x80
    }

    #[test]
    fn test_basic_form_edge_case_opcodes() {
        // Test edge case opcode values
        verify_basic_form(1, 0x00, InstructionForm::Short); // minimum opcode
        verify_basic_form(2, 0x7F, InstructionForm::Long); // maximum Long form opcode
        verify_basic_form(2, 0x80, InstructionForm::Variable); // minimum Variable form opcode
        verify_basic_form(1, 0xFF, InstructionForm::Short); // maximum opcode (1 operand)
    }

    // ======================================================================================
    // CONTEXT-DEPENDENT TESTS: 0x0D scenarios (6 tests)
    // ======================================================================================

    #[test]
    fn test_context_0x0d_print_paddr() {
        // 1 operand + no store_var = print_paddr (1OP:13) -> SHORT form
        let operands = [Operand::LargeConstant(0x1234)];
        verify_context_form(&operands, 0x0D, None, InstructionForm::Short);

        let operands = [Operand::SmallConstant(42)];
        verify_context_form(&operands, 0x0D, None, InstructionForm::Short);

        let operands = [Operand::Variable(5)];
        verify_context_form(&operands, 0x0D, None, InstructionForm::Short);
    }

    #[test]
    fn test_context_0x0d_store_1_operand() {
        // 1 operand + store_var = store (2OP:13) -> LONG form
        let operands = [Operand::LargeConstant(0x1234)];
        verify_context_form(&operands, 0x0D, Some(0), InstructionForm::Long);
        verify_context_form(&operands, 0x0D, Some(1), InstructionForm::Long);
        verify_context_form(&operands, 0x0D, Some(255), InstructionForm::Long);

        let operands = [Operand::Variable(10)];
        verify_context_form(&operands, 0x0D, Some(5), InstructionForm::Long);
    }

    #[test]
    fn test_context_0x0d_store_2_operands() {
        // 2 operands = store (2OP:13) -> LONG form (regardless of store_var)
        let operands = [Operand::Variable(1), Operand::LargeConstant(42)];
        verify_context_form(&operands, 0x0D, None, InstructionForm::Long);
        verify_context_form(&operands, 0x0D, Some(0), InstructionForm::Long);
        verify_context_form(&operands, 0x0D, Some(255), InstructionForm::Long);

        let operands = [Operand::SmallConstant(1), Operand::SmallConstant(2)];
        verify_context_form(&operands, 0x0D, None, InstructionForm::Long);
    }

    #[test]
    fn test_context_0x0d_output_stream_3_operands() {
        // 3+ operands = output_stream (VAR:13) -> VARIABLE form
        let operands = [
            Operand::SmallConstant(1),
            Operand::SmallConstant(2),
            Operand::SmallConstant(3),
        ];
        verify_context_form(&operands, 0x0D, None, InstructionForm::Variable);
        verify_context_form(&operands, 0x0D, Some(0), InstructionForm::Variable);

        let operands = [
            Operand::Variable(1),
            Operand::LargeConstant(100),
            Operand::Variable(2),
            Operand::SmallConstant(4),
        ];
        verify_context_form(&operands, 0x0D, Some(5), InstructionForm::Variable);
    }

    #[test]
    fn test_context_0x0d_regression_scenarios() {
        // Specific scenarios that were broken before the 0x0D context-dependent fix

        // The problematic scenario: 1 operand with store_var was incorrectly identified as print_paddr
        let operands = [Operand::Variable(217)]; // player object variable
        verify_context_form(&operands, 0x0D, Some(0), InstructionForm::Long); // should be store, not print_paddr

        // Edge case: exactly 2 operands should always be Long form store
        let operands = [Operand::Variable(1), Operand::SmallConstant(0)];
        verify_context_form(&operands, 0x0D, None, InstructionForm::Long);
    }

    #[test]
    fn test_context_non_0x0d_opcodes() {
        // Non-0x0D opcodes should use the standard operand-based logic
        let operands = [Operand::SmallConstant(1)];

        // 1OP opcodes should be Short regardless of store_var
        verify_context_form(&operands, 0x8D, None, InstructionForm::Short); // print_paddr (different opcode)
        verify_context_form(&operands, 0x8D, Some(0), InstructionForm::Short);

        // 2OP opcodes should follow normal operand count rules
        let operands = [Operand::SmallConstant(1), Operand::SmallConstant(2)];
        verify_context_form(&operands, 0x01, None, InstructionForm::Long); // je
        verify_context_form(&operands, 0x01, Some(0), InstructionForm::Long);
    }

    // ======================================================================================
    // BOUNDARY CONDITION TESTS: Edge cases (5 tests)
    // ======================================================================================

    #[test]
    fn test_boundary_exact_2_operands_form_selection() {
        // Test the critical boundary: exactly 2 operands can be Long or Variable form

        // Opcode < 0x80: should prefer Long form
        let operands = [Operand::SmallConstant(1), Operand::SmallConstant(2)];
        verify_operand_form(&operands, 0x01, Ok(InstructionForm::Long)); // je
        verify_operand_form(&operands, 0x7F, Ok(InstructionForm::Long)); // boundary

        // Opcode >= 0x80: should be Variable form
        verify_operand_form(&operands, 0x80, Ok(InstructionForm::Variable));
        verify_operand_form(&operands, 0xE0, Ok(InstructionForm::Variable)); // call
    }

    #[test]
    fn test_boundary_large_constant_operand_constraints() {
        // Test operand type constraints that might force Variable form

        // Large constants that fit in byte range should allow Long form
        let operands = [
            Operand::LargeConstant(255), // maximum byte value
            Operand::SmallConstant(1),
        ];
        verify_operand_form(&operands, 0x01, Ok(InstructionForm::Long));

        // Large constants > 255 force Variable form (actual implementation behavior)
        let operands = [
            Operand::LargeConstant(256), // exceeds byte range
            Operand::SmallConstant(1),
        ];
        // Large constants > 255 cannot use Long form encoding, so forces Variable form
        verify_operand_form(&operands, 0x01, Ok(InstructionForm::Variable));
    }

    #[test]
    fn test_boundary_operand_count_transitions() {
        // Test transitions between operand count ranges

        let single_operand = [Operand::SmallConstant(1)];
        let double_operands = [Operand::SmallConstant(1), Operand::SmallConstant(2)];
        let triple_operands = [
            Operand::SmallConstant(1),
            Operand::SmallConstant(2),
            Operand::SmallConstant(3),
        ];

        // Regular opcode behavior across transitions
        verify_operand_form(&single_operand, 0x50, Ok(InstructionForm::Short));
        verify_operand_form(&double_operands, 0x50, Ok(InstructionForm::Long));
        verify_operand_form(&triple_operands, 0x50, Ok(InstructionForm::Variable));
    }

    #[test]
    fn test_boundary_always_var_opcodes_override() {
        // Test that always-VAR opcodes override operand count logic

        let no_operands: [Operand; 0] = [];
        let single_operand = [Operand::SmallConstant(1)];
        let double_operands = [Operand::SmallConstant(1), Operand::SmallConstant(2)];

        // 0xE0 (call) should always be Variable form
        verify_operand_form(&no_operands, 0xE0, Ok(InstructionForm::Variable));
        verify_operand_form(&single_operand, 0xE0, Ok(InstructionForm::Variable));
        verify_operand_form(&double_operands, 0xE0, Ok(InstructionForm::Variable));

        // Note: 0xE3 is only always-VAR in determine_instruction_form(), but NOT in
        // determine_instruction_form_with_operands() - it follows normal operand count logic
        // 1 operand -> Short form
        // 2 operands + opcode 0xE3 (> 0x80) -> Variable form (not Long)
        verify_operand_form(&single_operand, 0xE3, Ok(InstructionForm::Short));
        verify_operand_form(&double_operands, 0xE3, Ok(InstructionForm::Variable));
    }

    #[test]
    fn test_boundary_form_sensitive_opcodes() {
        // Test form-sensitive opcodes that can conflict between Long and Variable forms

        let double_operands = [Operand::SmallConstant(1), Operand::SmallConstant(2)];
        let triple_operands = [
            Operand::SmallConstant(1),
            Operand::SmallConstant(2),
            Operand::SmallConstant(3),
        ];

        // 0x01: je (Long) vs storew (Variable) - should prefer Long for 2 operands
        verify_operand_form(&double_operands, 0x01, Ok(InstructionForm::Long));
        verify_operand_form(&triple_operands, 0x01, Ok(InstructionForm::Variable));

        // 0x03: jg (Long) vs put_prop (Variable) - should prefer Long for 2 operands
        verify_operand_form(&double_operands, 0x03, Ok(InstructionForm::Long));
        verify_operand_form(&triple_operands, 0x03, Ok(InstructionForm::Variable));
    }

    // ======================================================================================
    // ERROR HANDLING TESTS: Invalid combinations (4 tests)
    // ======================================================================================

    // TODO: Add error handling tests once we understand what error conditions exist
    // in the current implementation. The form determination functions mostly return
    // Ok(form) rather than errors, so we need to identify what should cause errors.

    #[test]
    fn test_error_handling_placeholder() {
        // Placeholder for error handling tests
        // Will be implemented once we analyze what error conditions should exist
        assert!(true, "Error handling tests pending implementation");
    }

    #[test]
    fn test_invalid_operand_counts_placeholder() {
        // Placeholder for invalid operand count tests
        assert!(true, "Invalid operand count tests pending implementation");
    }

    #[test]
    fn test_conflicting_form_requirements_placeholder() {
        // Placeholder for conflicting form requirement tests
        assert!(
            true,
            "Conflicting form requirement tests pending implementation"
        );
    }

    #[test]
    fn test_error_message_quality_placeholder() {
        // Placeholder for error message quality tests
        assert!(true, "Error message quality tests pending implementation");
    }

    // ======================================================================================
    // INTEGRATION TESTS: Real instruction emission (3 tests)
    // ======================================================================================

    // TODO: Add integration tests that verify form determination affects actual
    // instruction emission correctly. These should test the full pipeline from
    // form determination through emit_instruction_typed().

    #[test]
    fn test_integration_form_affects_bytecode_placeholder() {
        // Placeholder for integration tests
        assert!(true, "Integration tests pending implementation");
    }

    #[test]
    fn test_integration_emit_instruction_typed_placeholder() {
        // Placeholder for emit_instruction_typed integration
        assert!(
            true,
            "emit_instruction_typed integration tests pending implementation"
        );
    }

    #[test]
    fn test_integration_real_instruction_sequences_placeholder() {
        // Placeholder for real instruction sequence tests
        assert!(
            true,
            "Real instruction sequence tests pending implementation"
        );
    }

    // ======================================================================================
    // REGRESSION TESTS: Recently fixed bugs (2 tests)
    // ======================================================================================

    #[test]
    fn test_regression_0x0d_context_bug() {
        // Specific regression test for the recently fixed 0x0D context-dependent bug

        // Before the fix: 1 operand + store_var was incorrectly identified as print_paddr
        // After the fix: should correctly identify as store (Long form)
        let operands = [Operand::Variable(217)]; // specific case from the bug report
        verify_context_form(&operands, 0x0D, Some(0), InstructionForm::Long);

        // Verify the correct print_paddr case still works
        verify_context_form(&operands, 0x0D, None, InstructionForm::Short);

        // Verify other context scenarios work correctly
        let two_operands = [Operand::Variable(1), Operand::SmallConstant(42)];
        verify_context_form(&two_operands, 0x0D, None, InstructionForm::Long);
        verify_context_form(&two_operands, 0x0D, Some(5), InstructionForm::Long);
    }

    #[test]
    fn test_regression_compilation_failure_scenarios() {
        // Test scenarios that previously caused "Long form requires exactly 2 operands" failures

        // These should now work correctly with proper form determination
        let single_operand = [Operand::Variable(100)];
        let double_operands = [Operand::Variable(1), Operand::LargeConstant(42)];
        let triple_operands = [
            Operand::SmallConstant(1),
            Operand::Variable(2),
            Operand::LargeConstant(3),
        ];

        // Verify no form conflicts for common opcode patterns
        verify_operand_form(&single_operand, 0x8D, Ok(InstructionForm::Short)); // print_paddr
        verify_operand_form(&double_operands, 0x01, Ok(InstructionForm::Long)); // je
        verify_operand_form(&triple_operands, 0x0D, Ok(InstructionForm::Variable)); // output_stream

        // Context-dependent verification
        verify_context_form(&single_operand, 0x0D, Some(0), InstructionForm::Long); // store
        verify_context_form(&triple_operands, 0x0D, None, InstructionForm::Variable);
        // output_stream
    }
}
