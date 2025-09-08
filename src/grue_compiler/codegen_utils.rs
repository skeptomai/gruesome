// Utility functions extracted from codegen.rs for better maintainability
// These functions provide debugging, analysis, and validation capabilities

use crate::grue_compiler::ir::{IrProgram, IrInstruction};
use crate::grue_compiler::CompilerError;
use std::collections::HashMap;
use std::mem::discriminant;

/// IR Analysis and Debugging Utilities
pub struct CodeGenUtils;

impl CodeGenUtils {
    /// Log comprehensive IR inventory for debugging and analysis
    pub fn log_ir_inventory(ir: &IrProgram) {
        log::info!(" IR INVENTORY: Comprehensive input analysis");
        log::info!("  ├─ Functions: {} definitions", ir.functions.len());
        log::info!(
            "  ├─ Init block: {}",
            if ir.init_block.is_some() {
                "present"
            } else {
                "missing"
            }
        );
        log::info!("  ├─ Grammar rules: {} rules", ir.grammar.len());
        log::info!("  ├─ Objects: {} definitions", ir.objects.len());
        log::info!("  ├─ Rooms: {} definitions", ir.rooms.len());
        log::info!("  ├─ String table: {} strings", ir.string_table.len());

        let total_ir_instructions = Self::count_total_ir_instructions(ir);
        log::info!(
            "  └─ Total IR instructions: {} instructions",
            total_ir_instructions
        );

        // Log detailed function breakdown
        for (i, function) in ir.functions.iter().enumerate() {
            log::debug!(
                "📋 Function #{}: '{}' with {} instructions",
                i,
                function.name,
                function.body.instructions.len()
            );
        }

        // Log init block breakdown
        if let Some(init_block) = &ir.init_block {
            log::debug!(
                "📋 Init Block: {} instructions",
                init_block.instructions.len()
            );
        }

        // Log instruction type breakdown
        Self::log_ir_instruction_breakdown(ir);
    }

    /// Count total IR instructions across all functions and init blocks
    pub fn count_total_ir_instructions(ir: &IrProgram) -> usize {
        let mut total = 0;

        // Count function instructions
        for function in &ir.functions {
            total += function.body.instructions.len();
        }

        // Count init block instructions
        if let Some(init_block) = &ir.init_block {
            total += init_block.instructions.len();
        }

        total
    }

    /// Log breakdown of IR instruction types for analysis
    pub fn log_ir_instruction_breakdown(ir: &IrProgram) {
        let mut instruction_counts: HashMap<String, usize> = HashMap::new();

        // Count function instructions by type
        for function in &ir.functions {
            for instruction in &function.body.instructions {
                let type_name = format!("{:?}", discriminant(instruction))
                    .replace("std::mem::Discriminant<grue_compiler::ir::", "")
                    .replace(">(", "")
                    .replace(")", "");
                *instruction_counts.entry(type_name).or_insert(0) += 1;
            }
        }

        // Count init block instructions by type
        if let Some(init_block) = &ir.init_block {
            for instruction in &init_block.instructions {
                let type_name = format!("{:?}", discriminant(instruction))
                    .replace("std::mem::Discriminant<grue_compiler::ir::", "")
                    .replace(">(", "")
                    .replace(")", "");
                *instruction_counts.entry(type_name).or_insert(0) += 1;
            }
        }

        log::debug!("📊 IR INSTRUCTION BREAKDOWN:");
        let mut sorted_counts: Vec<_> = instruction_counts.iter().collect();
        sorted_counts.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count descending

        for (instruction_type, count) in sorted_counts {
            log::debug!("  ├─ {}: {}", instruction_type, count);
        }
    }

    /// Validate IR input for completeness and consistency
    pub fn validate_ir_input(ir: &IrProgram) -> Result<(), CompilerError> {
        log::debug!(" IR VALIDATION: Checking input completeness");

        // Critical validations - crash early if IR is malformed
        if ir.functions.is_empty() && ir.init_block.is_none() {
            return Err(CompilerError::CodeGenError(
                "COMPILER BUG: No executable IR instructions found".to_string(),
            ));
        }

        // Log validation success
        let total_executable_content = Self::count_total_ir_instructions(ir);
        log::debug!(
            " IR VALIDATION: Input appears valid ({} total instructions)",
            total_executable_content
        );

        Ok(())
    }

    /// Analyze instruction expectations for bytecode generation
    /// Returns (expected_bytecode_instructions, expected_zero_instructions, total_instructions)
    pub fn analyze_instruction_expectations(
        instructions: &[IrInstruction],
    ) -> (usize, usize, usize) {
        let mut expected_bytecode = 0;
        let mut expected_zero = 0;

        for instruction in instructions {
            match instruction {
                // Instructions that should NOT generate bytecode
                IrInstruction::LoadImmediate {
                    value: crate::grue_compiler::ir::IrValue::String(_),
                    ..
                } => {
                    expected_zero += 1; // String definitions
                }
                IrInstruction::LoadImmediate {
                    value: crate::grue_compiler::ir::IrValue::Integer(_),
                    ..
                } => {
                    expected_zero += 1; // Integer constants
                }
                IrInstruction::LoadImmediate {
                    value: crate::grue_compiler::ir::IrValue::Boolean(_),
                    ..
                } => {
                    expected_zero += 1; // Boolean constants
                }
                IrInstruction::LoadImmediate {
                    value: crate::grue_compiler::ir::IrValue::Null,
                    ..
                } => {
                    expected_zero += 1; // Null values
                }
                IrInstruction::Nop => {
                    expected_zero += 1; // No-op instructions
                }
                IrInstruction::Label { .. } => {
                    expected_zero += 1; // Labels don't generate code
                }
                IrInstruction::LoadVar { .. } => {
                    expected_bytecode += 1; // Variable loading generates Z-Machine load instructions
                }
                IrInstruction::StoreVar { .. } => {
                    expected_bytecode += 1; // Variable storing generates Z-Machine store instructions
                }
                IrInstruction::BinaryOp { .. } => {
                    expected_bytecode += 1; // Binary operations generate arithmetic instructions
                }
                IrInstruction::SetProperty { .. } => {
                    expected_bytecode += 1; // Property assignment generates put_prop instructions
                }

                // Instructions that SHOULD generate bytecode
                IrInstruction::Call { .. } => {
                    expected_bytecode += 1; // Function calls generate bytecode
                }
                IrInstruction::Return { .. } => {
                    expected_bytecode += 1; // Return instructions generate bytecode
                }
                IrInstruction::Jump { .. } => {
                    expected_bytecode += 1; // Jump instructions generate bytecode
                }
                IrInstruction::Branch { .. } => {
                    expected_bytecode += 1; // Branch instructions generate bytecode
                }
                IrInstruction::UnaryOp { .. } => {
                    expected_bytecode += 1; // Unary operations generate bytecode
                }
                IrInstruction::GetProperty { .. } => {
                    expected_bytecode += 1; // Property access generates get_prop instructions
                }

                // Conservative: assume other instructions should generate bytecode
                _ => {
                    expected_bytecode += 1;
                }
            }
        }

        (expected_bytecode, expected_zero, instructions.len())
    }
}

/// Final Assembly Validation Utilities
pub struct AssemblyValidator;

impl AssemblyValidator {
    /// Validate final assembled data for consistency and correctness
    pub fn validate_final_assembly(final_data: &[u8]) -> Result<(), CompilerError> {
        log::debug!(" VALIDATION: Checking final assembly");

        // Check for remaining placeholders
        let mut placeholder_count = 0;
        for (i, &byte) in final_data.iter().enumerate() {
            if byte == 0xFF {
                // PLACEHOLDER_BYTE
                placeholder_count += 1;
                log::warn!(" PLACEHOLDER_REMAINING: Byte {} at address 0x{:04x}", byte, i);
            }
        }

        if placeholder_count > 0 {
            return Err(CompilerError::CodeGenError(format!(
                "Final assembly contains {} unresolved placeholders",
                placeholder_count
            )));
        }

        log::debug!(
            " VALIDATION: Final assembly OK ({} bytes, no placeholders)",
            final_data.len()
        );
        Ok(())
    }

    /// Calculate Z-Machine checksum for the assembled data
    pub fn calculate_checksum(final_data: &[u8]) -> u16 {
        let mut sum: u32 = 0;

        // Sum all bytes except the checksum bytes themselves (28-29)
        for (i, &byte) in final_data.iter().enumerate() {
            if i != 28 && i != 29 {
                sum = sum.wrapping_add(byte as u32);
            }
        }

        // Z-Machine checksum is the low 16 bits
        (sum & 0xFFFF) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_total_ir_instructions() {
        // Simple test - count should work correctly on empty IR
        let ir = IrProgram::default();
        assert_eq!(CodeGenUtils::count_total_ir_instructions(&ir), 0);
    }

    #[test]
    fn test_validate_ir_input_empty() {
        let ir = IrProgram::default();
        assert!(CodeGenUtils::validate_ir_input(&ir).is_err());
    }

    #[test]
    fn test_calculate_checksum() {
        let data = vec![0x01, 0x02, 0x03, 0x04]; // Simple test data
        let checksum = AssemblyValidator::calculate_checksum(&data);
        assert_eq!(checksum, 10); // 1 + 2 + 3 + 4 = 10
    }
}