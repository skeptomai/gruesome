/// codegen_branch.rs
use crate::grue_compiler::codegen::ZMachineCodeGen;
use crate::grue_compiler::codegen_memory::{placeholder_word, MemorySpace};
use crate::grue_compiler::codegen_objects::Operand;
use crate::grue_compiler::codegen_references::{LegacyReferenceType, UnresolvedReference};
use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::IrBinaryOp;
use crate::grue_compiler::ir::IrId;
use crate::grue_compiler::opcodes::{Op1, Opcode};

impl ZMachineCodeGen {
    /// Emit proper Z-Machine conditional branch instruction
    pub fn emit_conditional_branch_instruction(
        &mut self,
        condition: IrId,
        true_label: IrId,
        false_label: IrId,
    ) -> Result<(), CompilerError> {
        log::debug!(
 " EMIT_CONDITIONAL_BRANCH_INSTRUCTION: condition_id={}, true={}, false={} at code_addr=0x{:04x}",
 condition,
 true_label,
 false_label,
 self.code_address
 );
        let before_addr = self.code_address;

        // CORRECT APPROACH: Check if condition is a BinaryOp comparison
        // If so, generate the Z-Machine branch instruction directly
        log::debug!(
            "CHECKING_BINARY_OP_MAPPING: condition={}, mapping exists={}",
            condition,
            self.ir_id_to_binary_op.contains_key(&condition)
        );
        if let Some((op, left, right)) = self.ir_id_to_binary_op.get(&condition).cloned() {
            // CRITICAL FIX: Only comparison operations should take the direct branch path
            // Logical operations (And, Or) should use the normal resolve path to pull from stack
            match op {
                IrBinaryOp::Equal
                | IrBinaryOp::NotEqual
                | IrBinaryOp::Less
                | IrBinaryOp::LessEqual
                | IrBinaryOp::Greater
                | IrBinaryOp::GreaterEqual => {
                    log::debug!(
         "DIRECT_COMPARISON_BRANCH: Detected comparison BinaryOp {:?} - generating direct Z-Machine branch instruction",
         op
         );
                }
                _ => {
                    log::debug!(
         "NON_COMPARISON_BINARYOP: Detected logical BinaryOp {:?} - using normal stack resolution path",
         op
         );
                    // Skip direct branch path, let it fall through to normal resolution
                }
            }

            // Only handle comparison operations in direct branch path
            if matches!(
                op,
                IrBinaryOp::Equal
                    | IrBinaryOp::NotEqual
                    | IrBinaryOp::Less
                    | IrBinaryOp::LessEqual
                    | IrBinaryOp::Greater
                    | IrBinaryOp::GreaterEqual
            ) {
                // Resolve operands for the comparison
                let left_operand = self.resolve_ir_id_to_operand(left)?;
                let right_operand = self.resolve_ir_id_to_operand(right)?;
                log::debug!(
                    "🔍 COMPARISON: left_id={} -> {:?}, right_id={} -> {:?}",
                    left,
                    left_operand,
                    right,
                    right_operand
                );
                // Removed debugging panic - label IDs may legitimately be used as operands in some contexts

                // Generate the appropriate Z-Machine branch instruction
                let (opcode, branch_on_true) = match op {
                    IrBinaryOp::Equal => (0x01, true),         // je - branch if equal
                    IrBinaryOp::NotEqual => (0x01, false),     // je - branch if NOT equal
                    IrBinaryOp::Less => (0x02, true),          // jl - branch if less
                    IrBinaryOp::LessEqual => (0x03, false),    // jg - branch if NOT greater
                    IrBinaryOp::Greater => (0x03, true),       // jg - branch if greater
                    IrBinaryOp::GreaterEqual => (0x02, false), // jl - branch if NOT less
                    _ => {
                        return Err(CompilerError::CodeGenError(format!(
                            "Unsupported comparison operation in direct branch: {:?}",
                            op
                        )));
                    }
                };

                // CRITICAL FIX: We want to skip the THEN block when the condition is FALSE
                // So we branch to false_label (skip) when the condition is FALSE
                // This means we need to INVERT branch_on_true
                let branch_target = false_label; // Always branch to the skip-THEN label
                let emit_branch_on_true = !branch_on_true; // Invert the sense

                log::debug!(
                "GENERATING_DIRECT_BRANCH: {:?} with opcode 0x{:02x}, branching to {} on {} (inverted)",
                op,
                opcode,
                branch_target,
                if emit_branch_on_true { "true" } else { "false" }
            );

                // Generate the comparison branch instruction
                self.emit_comparison_branch(
                    opcode,
                    &[left_operand, right_operand],
                    branch_target,
                    emit_branch_on_true,
                )?;

                let after_addr = self.code_address;
                log::debug!(
 " EMIT_CONDITIONAL_BRANCH_INSTRUCTION: Generated {} bytes (0x{:04x} -> 0x{:04x}) via comparison branch",
 after_addr - before_addr,
 before_addr,
 after_addr
 );
                return Ok(());
            } // End of comparison operation handling
        }

        // Fallback for non-comparison conditions (use jz branch approach)
        log::debug!(
            "Condition {} is not a comparison - using jz branch approach",
            condition
        );
        let result = self.emit_jz_branch(condition, true_label, false_label);
        let after_addr = self.code_address;
        log::debug!(
            " EMIT_CONDITIONAL_BRANCH_INSTRUCTION: Generated {} bytes (0x{:04x} -> 0x{:04x})",
            after_addr - before_addr,
            before_addr,
            after_addr
        );
        result
    }

    /// Emit a jz (jump if zero) branch instruction for boolean conditions
    fn emit_jz_branch(
        &mut self,
        condition: IrId,
        _true_label: IrId,
        false_label: IrId,
    ) -> Result<(), CompilerError> {
        // CRITICAL FIX: Check if condition is a comparison BinaryOp that should have been handled directly
        if let Some((op, _left, _right)) = self.ir_id_to_binary_op.get(&condition).cloned() {
            // Only return an error for comparison operations - logical operations are legitimate here
            match op {
                IrBinaryOp::Equal
                | IrBinaryOp::NotEqual
                | IrBinaryOp::Less
                | IrBinaryOp::LessEqual
                | IrBinaryOp::Greater
                | IrBinaryOp::GreaterEqual => {
                    log::debug!(
         " MISSING_COMPARISON_FIX: Detected ungenerated comparison BinaryOp {:?} for condition {}, this should have been handled directly!",
         op, condition
         );
                    return Err(CompilerError::CodeGenError(format!(
         "emit_jz_branch: Comparison {:?} should not generate standalone instructions with stack storage - comparisons should be handled by proper conditional branching logic directly",
         op
         )));
                }
                _ => {
                    log::debug!(
         " LOGICAL_BINARYOP_OK: Detected logical BinaryOp {:?} for condition {} - this is legitimate for jz branch",
         op, condition
         );
                    // Logical operations (And, Or) are fine here - they produce values that should be resolved from stack
                }
            }
        }

        // Resolve condition operand
        let condition_operand = match self.resolve_ir_id_to_operand(condition) {
            Ok(operand) => operand,
            Err(_) => {
                // CRITICAL FIX: Using Variable(0) when stack is empty causes underflow
                // Instead, use a safe default constant value
                log::debug!(
 "COMPILER BUG: Could not resolve condition IR ID {} - using constant 0 fallback instead of dangerous stack access",
 condition
 );
                Operand::SmallConstant(0) // Safe fallback: constant 0 (false condition)
            }
        };

        // FIXED: Emit jz instruction WITH placeholder branch offset
        // The emit_instruction function handles placeholder emission properly
        // jz is opcode 0 in the 1OP group, so it should be encoded as 0x80 (1OP form + opcode 0)
        // Bit 15 of placeholder = 1 means "branch on true" (when value IS zero, branch to false_label)
        // Using -1 (0xFFFF) sets bit 15, making branch_on_true = true
        let layout = self.emit_instruction_typed(
            Opcode::Op1(Op1::Jz), // jz (1OP:0) - jump if zero - correct Z-Machine encoding 0x80 + 0x00
            &[condition_operand],
            None,     // No store
            Some(-1), // Negative value sets bit 15 = branch on true (jump when zero)
        )?;

        // Use the branch_location from layout (calculated correctly by emit_instruction)
        if let Some(branch_location) = layout.branch_location {
            log::debug!(
                " JZ_BRANCH_REF_CREATE: branch_location=0x{:04x} target_id={}",
                branch_location,
                false_label
            );
            self.reference_context
                .unresolved_refs
                .push(UnresolvedReference {
                    reference_type: LegacyReferenceType::Branch,
                    location: branch_location, // Use exact location from emit_instruction
                    target_id: false_label,    // jz jumps on false condition
                    is_packed_address: false,
                    offset_size: 1, // Branch offset size depends on the actual offset value
                    location_space: MemorySpace::Code,
                });
        } else {
            return Err(CompilerError::CodeGenError(
                "jz instruction must have branch_location".to_string(),
            ));
        }

        Ok(())
    }

    /// Emit a Z-Machine comparison branch instruction (je, jl, jg, etc.)
    pub fn emit_comparison_branch(
        &mut self,
        opcode: u8,
        operands: &[Operand],
        target_label: IrId,
        branch_on_true: bool,
    ) -> Result<(), CompilerError> {
        log::debug!(
 " EMIT_COMPARISON_BRANCH: opcode=0x{:02x}, operands={:?}, target={}, branch_on_true={} at code_addr=0x{:04x}",
 opcode,
 operands,
 target_label,
 branch_on_true,
 self.code_address
 );

        if self.code_address > 0x330 && self.code_address < 0x340 {
            log::debug!(
                "EMIT_COMPARISON_BRANCH called at critical address 0x{:04x} with opcode=0x{:02x}",
                self.code_address,
                opcode
            );
        }
        let before_addr = self.code_address;

        // FIXED: Emit comparison instruction WITH placeholder branch offset
        // The placeholder value encodes whether we branch on true (bit 15=1) or false (bit 15=0)
        let placeholder = if branch_on_true {
            0xBFFF_u16 as i16 // bit 15=1 for branch-on-TRUE
        } else {
            0x7FFF_u16 as i16 // bit 15=0 for branch-on-FALSE
        };

        log::debug!("EMIT_COMPARISON_BRANCH: Calling emit_instruction with placeholder=0x{:04x} (branch_on_true={}) at code_address=0x{:04x}",
            placeholder as u16, branch_on_true, self.code_address);

        let layout = self.emit_instruction(
            opcode,
            operands,
            None,              // No store
            Some(placeholder), // Placeholder encodes branch polarity
        )?;
        log::debug!("DEBUG: After emit_instruction, checking branch_location");

        // Use the branch_location from layout (calculated correctly by emit_instruction)
        if let Some(branch_location) = layout.branch_location {
            log::debug!(
                "Creating Branch UnresolvedReference at location 0x{:04x} for target {}",
                branch_location,
                target_label
            );
            self.reference_context
                .unresolved_refs
                .push(UnresolvedReference {
                    reference_type: LegacyReferenceType::Branch,
                    location: branch_location, // Use exact location from emit_instruction
                    target_id: target_label,
                    is_packed_address: false,
                    offset_size: 2,
                    location_space: MemorySpace::Code,
                });
        } else {
            log::error!("ERROR: emit_comparison_branch: layout.branch_location is None! This means emit_instruction didn't create a branch placeholder");
            return Err(CompilerError::CodeGenError(
                "Comparison branch instruction must have branch_location".to_string(),
            ));
        }

        let after_addr = self.code_address;
        log::debug!(
 " EMIT_COMPARISON_BRANCH: Generated {} bytes (0x{:04x} -> 0x{:04x}), fixup at offset=0x{:04x}",
 after_addr - before_addr,
 before_addr,
 after_addr,
 layout.branch_location.unwrap_or(0)
 );

        Ok(())
    }

    /// Generate branch instruction (legacy method, kept for compatibility)
    fn generate_branch(&mut self, true_label: IrId) -> Result<(), CompilerError> {
        // For now, emit a simple unconditional branch using jump
        // TODO: Support proper conditional branching with condition operand

        // Emit jump instruction with placeholder offset
        let layout = self.emit_instruction_typed(
            Opcode::Op1(Op1::Jump), // jump opcode (1OP:12) - fixed from 0x8C
            &[Operand::LargeConstant(placeholder_word())], // Placeholder offset (will be resolved later)
            None,                                          // No store
            None,                                          // No branch
        )?;

        // Add unresolved reference for the jump target using layout-tracked operand location
        let operand_address = layout
            .operand_location
            .expect("jump instruction must have operand");
        let reference = UnresolvedReference {
            reference_type: LegacyReferenceType::Jump,
            location: operand_address,
            target_id: true_label,
            is_packed_address: false,
            offset_size: 2,
            location_space: MemorySpace::Code,
        };
        self.reference_context.unresolved_refs.push(reference);

        Ok(())
    }
}
