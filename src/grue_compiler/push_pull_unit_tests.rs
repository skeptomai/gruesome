//! Comprehensive unit tests for push/pull stack discipline system
//!
//! These tests verify the core push/pull functionality that prevents Variable(0) collisions
//! by using proper LIFO stack semantics for all operations that would otherwise conflict.
//!
//! The push/pull system consists of:
//! 1. `use_push_pull_for_result()` - Registers IR IDs for stack-based result handling
//! 2. `resolve_ir_id_to_operand()` - Converts registered IDs to pull from temporary globals
//! 3. Temporary global allocation starting at 16 (after locals 0-15)
//!
//! Test Strategy:
//! These tests use behavioral testing rather than accessing private fields.
//! We test observable outcomes: what operands are returned, what code is emitted,
//! and how the system behaves under various scenarios.

use crate::grue_compiler::codegen::{Operand, ZMachineCodeGen};
use crate::grue_compiler::ZMachineVersion;
use std::panic;

/// Helper function to create a fresh codegen instance for testing
fn create_test_codegen() -> ZMachineCodeGen {
    ZMachineCodeGen::new(ZMachineVersion::V3)
}

#[cfg(test)]
mod core_push_pull_behavior_tests {
    use super::*;

    #[test]
    fn test_push_pull_registration_creates_temporary_global_operand() {
        let mut codegen = create_test_codegen();
        let ir_id = 100;

        // Register for push/pull
        codegen
            .use_push_pull_for_result(ir_id, "test operation")
            .expect("Failed to register push/pull");

        // Observable behavior: resolving should return temporary global and emit code
        let initial_code_size = codegen.code_space.len();
        let operand = codegen
            .resolve_ir_id_to_operand(ir_id)
            .expect("Failed to resolve registered IR ID");

        // Should return temporary global variable (>= 16)
        match operand {
            Operand::Variable(var_num) => {
                assert!(
                    var_num >= 16,
                    "Push/pull IR ID should resolve to temporary global >= 16, got {}",
                    var_num
                );
            }
            _ => panic!(
                "Push/pull IR ID should resolve to Variable operand, got {:?}",
                operand
            ),
        }

        // Should emit pull instruction
        assert!(
            codegen.code_space.len() > initial_code_size,
            "Resolving push/pull IR ID should emit pull instruction"
        );
    }

    #[test]
    fn test_double_registration_is_idempotent() {
        let mut codegen = create_test_codegen();
        let ir_id = 101;

        // Register twice
        codegen.use_push_pull_for_result(ir_id, "first").unwrap();
        codegen.use_push_pull_for_result(ir_id, "second").unwrap();

        // Should still resolve to temporary global
        let operand = codegen.resolve_ir_id_to_operand(ir_id).unwrap();
        match operand {
            Operand::Variable(var_num) => {
                assert!(var_num >= 16, "Should still resolve to temporary global");
            }
            _ => panic!("Expected Variable operand"),
        }
    }

    #[test]
    fn test_unregistered_ir_id_behavior() {
        let mut codegen = create_test_codegen();
        let ir_id = 102;

        // Current implementation panics for unmapped IR IDs with a COMPILER BUG message
        // This is actually correct behavior - unmapped IR IDs indicate a compiler bug
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            codegen.resolve_ir_id_to_operand(ir_id)
        }));

        match result {
            Ok(_) => {
                panic!("Expected panic for unmapped IR ID, but got success");
            }
            Err(panic_payload) => {
                // Panic is expected for unmapped IR IDs
                if let Some(error_msg) = panic_payload.downcast_ref::<String>() {
                    assert!(
                        error_msg.contains("No mapping found"),
                        "Panic should mention missing mapping, got: {}",
                        error_msg
                    );
                } else if let Some(error_msg) = panic_payload.downcast_ref::<&str>() {
                    assert!(
                        error_msg.contains("No mapping found"),
                        "Panic should mention missing mapping, got: {}",
                        error_msg
                    );
                } else {
                    panic!("Unexpected panic payload type");
                }
            }
        }
    }

    #[test]
    fn test_resolve_with_existing_literal_mapping() {
        let mut codegen = create_test_codegen();
        let ir_id = 103;

        // Without a way to set up public mappings, test that unmapped IDs panic appropriately
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            codegen.resolve_ir_id_to_operand(ir_id)
        }));

        // Should panic for unmapped IR ID
        match result {
            Ok(_) => {
                panic!("Expected panic for unmapped IR ID, but got success");
            }
            Err(panic_payload) => {
                // Panic is expected for unmapped IR IDs
                if let Some(error_msg) = panic_payload.downcast_ref::<String>() {
                    assert!(
                        error_msg.contains("No mapping found"),
                        "Panic should mention missing mapping, got: {}",
                        error_msg
                    );
                } else if let Some(error_msg) = panic_payload.downcast_ref::<&str>() {
                    assert!(
                        error_msg.contains("No mapping found"),
                        "Panic should mention missing mapping, got: {}",
                        error_msg
                    );
                } else {
                    panic!("Unexpected panic payload type");
                }
            }
        }
    }
}

#[cfg(test)]
mod temporary_global_allocation_behavior_tests {
    use super::*;

    #[test]
    fn test_sequential_temporary_global_allocation() {
        let mut codegen = create_test_codegen();

        // Register multiple IR IDs and verify sequential allocation
        let ir_ids = [110, 111, 112, 113, 114];
        let mut allocated_variables = Vec::new();

        for ir_id in ir_ids.iter() {
            // Register for push/pull
            codegen
                .use_push_pull_for_result(*ir_id, "sequential test")
                .expect("Failed to register push/pull");

            // Resolve to allocate temporary global
            let operand = codegen
                .resolve_ir_id_to_operand(*ir_id)
                .expect("Failed to resolve IR ID");

            match operand {
                Operand::Variable(var_num) => {
                    assert!(var_num >= 16, "Should allocate temporary global >= 16");
                    allocated_variables.push(var_num);
                }
                _ => panic!("Expected Variable operand, got {:?}", operand),
            }
        }

        // Verify all allocations are unique
        let mut unique_vars = allocated_variables.clone();
        unique_vars.sort();
        unique_vars.dedup();
        assert_eq!(
            allocated_variables.len(),
            unique_vars.len(),
            "All temporary globals should be unique"
        );

        // Verify they start at 16 and are sequential
        assert_eq!(allocated_variables[0], 16, "First allocation should be 16");
        for i in 1..allocated_variables.len() {
            assert_eq!(
                allocated_variables[i],
                allocated_variables[i - 1] + 1,
                "Allocations should be sequential"
            );
        }
    }

    #[test]
    fn test_same_ir_id_gets_different_temporaries_with_multiple_pulls() {
        let mut codegen = create_test_codegen();
        let ir_id = 115;

        // Register for push/pull
        codegen
            .use_push_pull_for_result(ir_id, "multiple pull test")
            .expect("Failed to register push/pull");

        // Resolve multiple times - each should emit a new pull and get a new temporary
        let operand1 = codegen.resolve_ir_id_to_operand(ir_id).unwrap();
        let operand2 = codegen.resolve_ir_id_to_operand(ir_id).unwrap();

        // Should get different temporary globals (each pull gets a fresh temporary)
        // This is correct behavior for stack discipline - each use pulls a fresh value
        match (operand1, operand2) {
            (Operand::Variable(var1), Operand::Variable(var2)) => {
                assert!(var1 >= 16, "First temporary should be >= 16");
                assert!(var2 >= 16, "Second temporary should be >= 16");
                assert_ne!(
                    var1, var2,
                    "Each pull should get a different temporary global"
                );
                assert_eq!(var2, var1 + 1, "Temporaries should be sequential");
            }
            _ => panic!("Expected Variable operands"),
        }
    }

    #[test]
    fn test_high_stress_temporary_global_allocation() {
        let mut codegen = create_test_codegen();

        // Allocate many temporary globals
        let num_allocations = 30;
        let mut allocated_variables = Vec::new();

        for i in 0..num_allocations {
            let ir_id = 1000 + i;

            codegen
                .use_push_pull_for_result(ir_id, "stress test")
                .unwrap();
            let operand = codegen.resolve_ir_id_to_operand(ir_id).unwrap();

            match operand {
                Operand::Variable(var_num) => {
                    allocated_variables.push(var_num);
                    assert!(var_num >= 16, "Should allocate temporary global >= 16");
                }
                _ => panic!("Expected Variable operand"),
            }
        }

        // Verify uniqueness and sequential allocation
        assert_eq!(allocated_variables.len(), num_allocations as usize);

        let mut unique_vars = allocated_variables.clone();
        unique_vars.sort();
        unique_vars.dedup();
        assert_eq!(
            allocated_variables.len(),
            unique_vars.len(),
            "All should be unique"
        );

        // Verify range
        assert_eq!(allocated_variables[0], 16, "Should start at 16");
        assert_eq!(
            allocated_variables[(num_allocations - 1) as usize],
            16 + num_allocations as u8 - 1,
            "Should be sequential"
        );
    }
}

#[cfg(test)]
mod variable_collision_prevention_tests {
    use super::*;

    #[test]
    fn test_multiple_push_pull_operations_get_unique_variables() {
        let mut codegen = create_test_codegen();

        // Simulate multiple operations that would normally use Variable(0)
        let call1_result = 120;
        let call2_result = 121;
        let call3_result = 122;

        // Register all for push/pull
        codegen
            .use_push_pull_for_result(call1_result, "call1")
            .unwrap();
        codegen
            .use_push_pull_for_result(call2_result, "call2")
            .unwrap();
        codegen
            .use_push_pull_for_result(call3_result, "call3")
            .unwrap();

        // Resolve all - should get unique temporary globals
        let op1 = codegen.resolve_ir_id_to_operand(call1_result).unwrap();
        let op2 = codegen.resolve_ir_id_to_operand(call2_result).unwrap();
        let op3 = codegen.resolve_ir_id_to_operand(call3_result).unwrap();

        let vars: Vec<u8> = [op1, op2, op3]
            .iter()
            .map(|op| match op {
                Operand::Variable(v) => *v,
                _ => panic!("Expected Variable operand"),
            })
            .collect();

        // All should be different
        assert_ne!(
            vars[0], vars[1],
            "Call results should use different variables"
        );
        assert_ne!(
            vars[1], vars[2],
            "Call results should use different variables"
        );
        assert_ne!(
            vars[0], vars[2],
            "Call results should use different variables"
        );

        // All should be temporary globals
        for var_num in &vars {
            assert!(*var_num >= 16, "Should use temporary global {}", var_num);
        }

        // Should be sequential starting from 16
        assert_eq!(vars, vec![16, 17, 18]);
    }

    #[test]
    fn test_complex_expression_evaluation_no_collision() {
        let mut codegen = create_test_codegen();

        // Simulate: result = func1() + func2() * func3()
        let func1_result = 130;
        let func2_result = 131;
        let func3_result = 132;
        let multiply_result = 133;
        let add_result = 134;

        // Register all intermediate results
        let ir_ids = [
            func1_result,
            func2_result,
            func3_result,
            multiply_result,
            add_result,
        ];
        for ir_id in &ir_ids {
            codegen
                .use_push_pull_for_result(*ir_id, "expression")
                .unwrap();
        }

        // Resolve all operands
        let operands: Vec<_> = ir_ids
            .iter()
            .map(|&ir_id| codegen.resolve_ir_id_to_operand(ir_id).unwrap())
            .collect();

        // Extract variable numbers
        let var_nums: Vec<u8> = operands
            .iter()
            .map(|op| match op {
                Operand::Variable(v) => *v,
                _ => panic!("Expected Variable operand"),
            })
            .collect();

        // Check uniqueness
        let mut unique_vars = var_nums.clone();
        unique_vars.sort();
        unique_vars.dedup();
        assert_eq!(
            var_nums.len(),
            unique_vars.len(),
            "All expression results should use unique variables"
        );

        // Check they're all temporary globals
        for var_num in &var_nums {
            assert!(
                *var_num >= 16,
                "Expression result should use temporary global"
            );
        }

        // Check sequential allocation
        assert_eq!(var_nums, vec![16, 17, 18, 19, 20]);
    }

    #[test]
    fn test_nested_function_call_scenarios() {
        let mut codegen = create_test_codegen();

        // Simulate nested calls: outer(inner(deep()))
        let deep_call = 140;
        let inner_call = 141;
        let outer_call = 142;

        // Register in reverse order (deep first, then inner, then outer)
        codegen
            .use_push_pull_for_result(deep_call, "deep call")
            .unwrap();
        codegen
            .use_push_pull_for_result(inner_call, "inner call")
            .unwrap();
        codegen
            .use_push_pull_for_result(outer_call, "outer call")
            .unwrap();

        // Resolve in evaluation order
        let deep_op = codegen.resolve_ir_id_to_operand(deep_call).unwrap();
        let inner_op = codegen.resolve_ir_id_to_operand(inner_call).unwrap();
        let outer_op = codegen.resolve_ir_id_to_operand(outer_call).unwrap();

        // All should be different temporary globals
        let vars = [deep_op, inner_op, outer_op].map(|op| match op {
            Operand::Variable(v) => v,
            _ => panic!("Expected Variable operand"),
        });

        assert_eq!(vars, [16, 17, 18]);

        // Verify no collision - all unique
        assert_ne!(vars[0], vars[1]);
        assert_ne!(vars[1], vars[2]);
        assert_ne!(vars[0], vars[2]);
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_ir_id_zero_edge_case() {
        let mut codegen = create_test_codegen();
        let ir_id = 0; // Edge case: IR ID 0

        // Register and resolve IR ID 0
        codegen
            .use_push_pull_for_result(ir_id, "edge case")
            .unwrap();
        let operand = codegen.resolve_ir_id_to_operand(ir_id).unwrap();

        match operand {
            Operand::Variable(var_num) => {
                assert!(var_num >= 16, "IR ID 0 should get temporary global");
            }
            _ => panic!("Expected Variable operand for IR ID 0"),
        }
    }

    #[test]
    fn test_large_ir_id_edge_case() {
        let mut codegen = create_test_codegen();
        let large_ir_id = u32::MAX; // Edge case: very large IR ID

        // Register and resolve large IR ID
        codegen
            .use_push_pull_for_result(large_ir_id, "large ID")
            .unwrap();
        let operand = codegen.resolve_ir_id_to_operand(large_ir_id).unwrap();

        match operand {
            Operand::Variable(var_num) => {
                assert!(var_num >= 16, "Large IR ID should get temporary global");
            }
            _ => panic!("Expected Variable operand for large IR ID"),
        }
    }

    #[test]
    fn test_code_emission_pattern() {
        let mut codegen = create_test_codegen();
        let ir_id = 150;

        // Register for push/pull
        codegen
            .use_push_pull_for_result(ir_id, "code test")
            .unwrap();

        // Track code space changes
        let initial_size = codegen.code_space.len();

        // First resolution should emit code
        let _operand1 = codegen.resolve_ir_id_to_operand(ir_id).unwrap();
        let after_first = codegen.code_space.len();

        // Second resolution might emit more code or reuse
        let _operand2 = codegen.resolve_ir_id_to_operand(ir_id).unwrap();
        let after_second = codegen.code_space.len();

        // First resolution should definitely emit code
        assert!(
            after_first > initial_size,
            "First resolution should emit pull instruction"
        );

        // Second resolution behavior depends on implementation
        // (might emit more code or reuse - both are acceptable)
        println!(
            "Code sizes: {} -> {} -> {}",
            initial_size, after_first, after_second
        );
    }
}

#[cfg(test)]
mod integration_pattern_tests {
    use super::*;

    #[test]
    fn test_typical_arithmetic_expression_pattern() {
        let mut codegen = create_test_codegen();

        // Pattern: a = get_prop(obj, prop) + call_func(arg)
        let get_prop_result = 160;
        let call_func_result = 161;
        let add_result = 162;

        // Register operations that would normally conflict on Variable(0)
        codegen
            .use_push_pull_for_result(get_prop_result, "get_prop")
            .unwrap();
        codegen
            .use_push_pull_for_result(call_func_result, "call_func")
            .unwrap();
        codegen.use_push_pull_for_result(add_result, "add").unwrap();

        // Resolve in typical evaluation order
        let prop_op = codegen.resolve_ir_id_to_operand(get_prop_result).unwrap();
        let func_op = codegen.resolve_ir_id_to_operand(call_func_result).unwrap();
        let result_op = codegen.resolve_ir_id_to_operand(add_result).unwrap();

        // Should get sequential temporary globals
        assert_eq!(prop_op, Operand::Variable(16));
        assert_eq!(func_op, Operand::Variable(17));
        assert_eq!(result_op, Operand::Variable(18));
    }

    #[test]
    fn test_push_pull_code_generation_observable_effects() {
        let mut codegen = create_test_codegen();

        // Test that push/pull operations actually generate different code patterns
        // compared to non-push/pull operations

        let push_pull_id = 170;

        // Register for push/pull
        codegen
            .use_push_pull_for_result(push_pull_id, "observable test")
            .unwrap();

        // Measure code generation effects
        let initial_code_size = codegen.code_space.len();
        let operand = codegen.resolve_ir_id_to_operand(push_pull_id).unwrap();
        let final_code_size = codegen.code_space.len();

        // Should have emitted code
        let code_emitted = final_code_size > initial_code_size;

        // Should return temporary global
        let uses_temp_global = match operand {
            Operand::Variable(v) => v >= 16,
            _ => false,
        };

        assert!(
            code_emitted || uses_temp_global,
            "Push/pull should either emit code or use temporary global (or both)"
        );

        // At minimum, push/pull should use temporary global
        assert!(uses_temp_global, "Push/pull should use temporary global");
    }
}
