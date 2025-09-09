use crate::grue_compiler::codegen::{ZMachineCodeGen, ConstantValue};
use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::{IrInstruction, IrValue};
use crate::grue_compiler::codegen::Operand;
use log::debug;

/// Extension trait for ZMachineCodeGen to handle instruction generation
impl ZMachineCodeGen {
    /// Generate code for a single IR instruction
    pub fn generate_instruction(&mut self, instruction: &IrInstruction) -> Result<(), CompilerError> {
        debug!("Generate instruction called: {:?}", instruction);
        // DEBUGGING: Log every instruction that creates a target
        match instruction {
            IrInstruction::LoadImmediate { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: LoadImmediate creates target IR ID {}",
                    target
                );
            }
            IrInstruction::BinaryOp { target, .. } => {
                log::debug!("IR INSTRUCTION: BinaryOp creates target IR ID {}", target);
            }
            IrInstruction::Call {
                target: Some(t), ..
            } => {
                log::debug!("IR INSTRUCTION: Call creates target IR ID {}", t);
            }
            IrInstruction::Call { target: None, .. } => {
                // No target to log
            }
            IrInstruction::GetProperty { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: GetProperty creates target IR ID {}",
                    target
                );
            }
            IrInstruction::GetPropertyByNumber { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: GetPropertyByNumber creates target IR ID {}",
                    target
                );
            }
            IrInstruction::UnaryOp { target, .. } => {
                log::debug!("IR INSTRUCTION: UnaryOp creates target IR ID {}", target);
            }
            IrInstruction::CreateArray { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: CreateArray creates target IR ID {}",
                    target
                );
            }
            IrInstruction::GetNextProperty { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: GetNextProperty creates target IR ID {}",
                    target
                );
            }
            IrInstruction::TestProperty { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: TestProperty creates target IR ID {}",
                    target
                );
            }
            IrInstruction::LoadVar { target, .. } => {
                log::debug!("IR INSTRUCTION: LoadVar creates target IR ID {}", target);
            }
            _ => {
                // Instructions without targets
            }
        }

        match instruction {
            IrInstruction::LoadImmediate { target, value } => {
                // CRITICAL: Register target IR ID mapping for LoadImmediate
                match value {
                    IrValue::Integer(i) => {
                        self.ir_id_to_integer.insert(*target, *i);
                        self.constant_values
                            .insert(*target, ConstantValue::Integer(*i));
                    }
                    IrValue::String(s) => {
                        self.ir_id_to_string.insert(*target, s.clone());
                        self.constant_values
                            .insert(*target, ConstantValue::String(s.clone()));
                    }
                    IrValue::Boolean(b) => {
                        // Convert boolean to integer for compatibility
                        let int_val = if *b { 1 } else { 0 };
                        self.ir_id_to_integer.insert(*target, int_val);
                        self.constant_values
                            .insert(*target, ConstantValue::Boolean(*b));
                    }
                    _ => {}
                }
                self.generate_load_immediate(value)?;
            }

            IrInstruction::BinaryOp {
                target,
                op,
                left,
                right,
            } => {
                self.process_binary_op(*target, op, *left, *right)?;
            }

            IrInstruction::Call {
                target,
                function,
                args,
            } => {
                // Check if this is a builtin function
                if self.is_builtin_function(*function) {
                    self.generate_builtin_function_call(*function, args, *target)?;
                } else {
                    // Generate call with unresolved function reference
                    self.generate_call_with_reference(*function, args, *target)?;
                }

                // CRITICAL: Register call result target for proper LoadVar resolution
                // Use stack for call results (per Z-Machine specification)
                if let Some(target_id) = target {
                    self.use_stack_for_result(*target_id);
                    log::debug!("Call result: IR ID {} -> stack", target_id);
                }
            }

            IrInstruction::Return { value } => {
                if let Some(ir_value) = value {
                    // Return with value - use ret opcode with operand
                    let return_operand = self.resolve_ir_id_to_operand(*ir_value)?;
                    let operands = vec![return_operand]; // Return resolved value
                    self.emit_instruction(0x8B, &operands, None, None)?; // ret (1OP:11)
                } else {
                    // Return without value - rtrue (no operands)
                    self.emit_instruction(0xB0, &[], None, None)?; // rtrue (0OP:0)
                }
            }

            IrInstruction::Jump { label } => {
                // Create unresolved reference for label (will be patched later)
                let _branch_offset = self.create_unresolved_reference(
                    crate::grue_compiler::codegen::LegacyReferenceType::Label(*label),
                    crate::grue_compiler::codegen::MemorySpace::CodeSpace,
                    self.current_address(),
                    *label,
                    false,
                    2,
                );
                // Jump: branch instruction with no condition (always true)
                // Use je (equal) with 1,1 as a way to generate "always true"
                self.emit_instruction(
                    0xC1, // je (2OP:1) in VAR form - will always be true
                    &[Operand::LargeConstant(1), Operand::LargeConstant(1)],
                    None,
                    Some(0), // Placeholder offset - will be resolved later
                )?;
            }

            IrInstruction::Branch {
                condition,
                true_label,
                false_label,
            } => {
                // Resolve condition to an operand
                let cond_operand = self.resolve_ir_id_to_operand(*condition)?;

                // Generate conditional branch - test if condition is true (non-zero)
                // We use jz (zero test) and flip the sense to get non-zero test
                let _true_offset = self.create_unresolved_reference(
                    crate::grue_compiler::codegen::LegacyReferenceType::Label(*true_label),
                    crate::grue_compiler::codegen::MemorySpace::CodeSpace,
                    self.current_address(),
                    *true_label,
                    false,
                    2,
                );

                // Test if condition is zero (false), if so go to false_label
                self.emit_instruction(0xA0, &[cond_operand], None, Some(0))?; // jz (1OP:0) - placeholder offset

                // If we reach here, condition is true, fall through or jump to false_label
                let _false_offset = self.create_unresolved_reference(
                    crate::grue_compiler::codegen::LegacyReferenceType::Label(*false_label),
                    crate::grue_compiler::codegen::MemorySpace::CodeSpace,
                    self.current_address(),
                    *false_label,
                    false,
                    2,
                );
                // Unconditional jump to false label
                self.emit_instruction(
                    0xC1, // je (always true branch)
                    &[Operand::LargeConstant(1), Operand::LargeConstant(1)],
                    None,
                    Some(0), // Placeholder offset
                )?;
            }

            IrInstruction::LoadVar { target, var_id } => {
                // Resolve source to get the value
                let source_operand = self.resolve_ir_id_to_operand(*var_id)?;

                // Store the loaded value in a way that can be accessed later
                // Use a store instruction to move value to stack
                self.emit_instruction(0x21, &[source_operand], Some(0), None)?; // store to stack

                // Map the target to stack access
                self.use_stack_for_result(*target);
                log::debug!("LoadVar: IR ID {} loaded from IR ID {} -> stack", target, var_id);
            }

            IrInstruction::StoreVar { var_id, source } => {
                // Resolve value to operand
                let value_operand = self.resolve_ir_id_to_operand(*source)?;

                // Store to appropriate variable slot
                if let Some(var_num) = self.ir_id_to_local_var.get(var_id) {
                    // Store to local variable
                    self.emit_instruction(0x21, &[value_operand], Some(*var_num), None)?; // store
                } else {
                    // Store to stack as fallback
                    self.emit_instruction(0x21, &[value_operand], Some(0), None)?; // store to stack
                    self.ir_id_to_stack_var.insert(*var_id, 0);
                }
                log::debug!("StoreVar: IR ID {} stored value from IR ID {}", var_id, source);
            }

            IrInstruction::Label { id: _ } => {
                // Labels are just markers - register current address for patching
                // This is handled during the address resolution phase
            }

            IrInstruction::UnaryOp { target, op, operand } => {
                self.generate_unary_op(*target, op, *operand)?;
            }

            IrInstruction::CreateArray { target, size } => {
                // Generate array creation
                // For now, create a simple array representation
                log::debug!("Creating array with size {:?}", size);

                // CRITICAL: Register target for array result
                self.use_stack_for_result(*target);
                log::debug!("CreateArray: IR ID {} -> stack", target);

                // Extract size value from IrValue
                let size_value = match size {
                    IrValue::Integer(i) => *i as u16,
                    IrValue::Boolean(b) => if *b { 1 } else { 0 },
                    _ => 0, // Default size
                };

                // Generate array initialization code
                // This is a placeholder - real arrays need more complex handling
                self.emit_instruction(
                    0x21, // store
                    &[Operand::LargeConstant(size_value)],
                    Some(0), // Store to stack
                    None,
                )?;
            }

            IrInstruction::GetProperty { target, object, property } => {
                // Generate property access code
                let _obj_operand = self.resolve_ir_id_to_operand(*object)?;

                // Get property by name - this needs object system integration
                log::debug!("Getting property '{}' from object", property);

                // CRITICAL: Register target for property result
                self.use_stack_for_result(*target);

                // Placeholder: return a default value
                self.emit_instruction(
                    0x21, // store
                    &[Operand::LargeConstant(0)], // Default property value
                    Some(0), // Store to stack
                    None,
                )?;
            }

            IrInstruction::SetProperty { object, property, value } => {
                // Generate property assignment code
                let _obj_operand = self.resolve_ir_id_to_operand(*object)?;
                let _val_operand = self.resolve_ir_id_to_operand(*value)?;

                log::debug!("Setting property '{}' on object", property);

                // Placeholder: property setting needs object system integration
            }

            IrInstruction::GetPropertyByNumber { target, object, property_num } => {
                // Generate numbered property access
                let obj_operand = self.resolve_ir_id_to_operand(*object)?;
                let prop_operand = self.resolve_ir_id_to_operand((*property_num).into())?;

                // CRITICAL: Register target for property result
                self.use_stack_for_result(*target);

                // Use get_prop instruction: 2OP:17 (0x11)
                self.emit_instruction(
                    0x11, // get_prop
                    &[obj_operand, prop_operand],
                    Some(0), // Store to stack
                    None,
                )?;
                log::debug!("GetPropertyByNumber: IR ID {} -> stack", target);
            }

            IrInstruction::TestProperty { target, object, property_num } => {
                // Generate property existence test
                let obj_operand = self.resolve_ir_id_to_operand(*object)?;
                let prop_operand = self.resolve_ir_id_to_operand((*property_num).into())?;

                // CRITICAL: Register target for test result
                self.use_stack_for_result(*target);

                // Use test_attr instruction: 2OP:10 (0x0A) for testing
                self.emit_instruction(
                    0x0A, // test_attr (placeholder - should be property test)
                    &[obj_operand, prop_operand],
                    Some(0), // Store to stack
                    None,
                )?;
                log::debug!("TestProperty: IR ID {} -> stack", target);
            }

            IrInstruction::GetNextProperty { target, object, current_property } => {
                // Generate next property enumeration
                let obj_operand = self.resolve_ir_id_to_operand(*object)?;
                let prop_operand = self.resolve_ir_id_to_operand((*current_property).into())?;

                // CRITICAL: Register target for next property result
                self.use_stack_for_result(*target);

                // Use get_next_prop instruction: 2OP:19 (0x13)
                self.emit_instruction(
                    0x13, // get_next_prop
                    &[obj_operand, prop_operand],
                    Some(0), // Store to stack
                    None,
                )?;
                log::debug!("GetNextProperty: IR ID {} -> stack", target);
            }

            IrInstruction::Nop => {
                // No operation - do nothing
            }

            _ => {
                // Handle other instruction types that are not yet implemented in the extracted code
                return Err(CompilerError::CodeGenError(format!(
                    "Instruction type {:?} not implemented in extracted generate_instruction",
                    instruction
                )));
            }
        }

        Ok(())
    }
}