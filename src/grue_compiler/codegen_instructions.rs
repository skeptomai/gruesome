use crate::grue_compiler::codegen::{ConstantValue, ZMachineCodeGen};
use crate::grue_compiler::codegen::{
    InstructionForm, InstructionLayout, Operand, OperandType, UNIMPLEMENTED_OPCODE,
};
use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::{IrInstruction, IrValue};
use log::debug;

/// Extension trait for ZMachineCodeGen to handle instruction generation
impl ZMachineCodeGen {
    /// Generate code for a single IR instruction
    pub fn generate_instruction(
        &mut self,
        instruction: &IrInstruction,
    ) -> Result<(), CompilerError> {
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
            // Array instructions with targets
            IrInstruction::ArrayRemove { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: ArrayRemove creates target IR ID {}",
                    target
                );
            }
            IrInstruction::ArrayLength { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: ArrayLength creates target IR ID {}",
                    target
                );
            }
            IrInstruction::ArrayEmpty { target, .. } => {
                log::debug!("IR INSTRUCTION: ArrayEmpty creates target IR ID {}", target);
            }
            IrInstruction::ArrayContains { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: ArrayContains creates target IR ID {}",
                    target
                );
            }
            IrInstruction::ArrayIndexOf { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: ArrayIndexOf creates target IR ID {}",
                    target
                );
            }
            IrInstruction::ArrayFilter { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: ArrayFilter creates target IR ID {}",
                    target
                );
            }
            IrInstruction::ArrayMap { target, .. } => {
                log::debug!("IR INSTRUCTION: ArrayMap creates target IR ID {}", target);
            }
            IrInstruction::ArrayFind { target, .. } => {
                log::debug!("IR INSTRUCTION: ArrayFind creates target IR ID {}", target);
            }
            IrInstruction::ArrayJoin { target, .. } => {
                log::debug!("IR INSTRUCTION: ArrayJoin creates target IR ID {}", target);
            }
            IrInstruction::ArrayReverse { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: ArrayReverse creates target IR ID {}",
                    target
                );
            }
            IrInstruction::ArraySort { target, .. } => {
                log::debug!("IR INSTRUCTION: ArraySort creates target IR ID {}", target);
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
                    // Generate user function call with proper reference registration
                    self.generate_user_function_call(*function, args, *target)?;
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
                // Delegate to the proper conditional branch function that handles binary operations
                self.emit_conditional_branch_instruction(*condition, *true_label, *false_label)?;
            }

            IrInstruction::LoadVar { target, var_id } => {
                // Resolve source to get the value
                let source_operand = self.resolve_ir_id_to_operand(*var_id)?;

                // Store the loaded value in a way that can be accessed later
                // Use a store instruction to move value to stack
                self.emit_instruction(0x21, &[source_operand], Some(0), None)?; // store to stack

                // Map the target to stack access
                self.use_stack_for_result(*target);
                log::debug!(
                    "LoadVar: IR ID {} loaded from IR ID {} -> stack",
                    target,
                    var_id
                );
            }

            IrInstruction::StoreVar { var_id, source } => {
                // Resolve value to operand
                let value_operand = self.resolve_ir_id_to_operand(*source)?;

                // Store to appropriate variable slot
                if let Some(var_num) = self.ir_id_to_local_var.get(var_id) {
                    // Store to local variable
                    self.emit_instruction(0x21, &[value_operand], Some(*var_num), None)?;
                // store
                } else {
                    // Store to stack as fallback
                    self.emit_instruction(0x21, &[value_operand], Some(0), None)?; // store to stack
                    self.ir_id_to_stack_var.insert(*var_id, 0);
                }
                log::debug!(
                    "StoreVar: IR ID {} stored value from IR ID {}",
                    var_id,
                    source
                );
            }

            IrInstruction::Label { id } => {
                // Labels are just markers - register current address for label resolution
                self.reference_context
                    .ir_id_to_address
                    .insert(*id, self.current_address());
                log::debug!(
                    "Label registered: IR ID {} -> address 0x{:04x}",
                    id,
                    self.current_address()
                );
            }

            IrInstruction::UnaryOp {
                target,
                op,
                operand,
            } => {
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
                    IrValue::Boolean(b) => {
                        if *b {
                            1
                        } else {
                            0
                        }
                    }
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

            IrInstruction::GetProperty {
                target,
                object,
                property,
            } => {
                // Generate property access code
                let _obj_operand = self.resolve_ir_id_to_operand(*object)?;

                // Get property by name - this needs object system integration
                log::debug!("Getting property '{}' from object", property);

                // CRITICAL: Register target for property result
                self.use_stack_for_result(*target);

                // Placeholder: return a default value
                self.emit_instruction(
                    0x21,                         // store
                    &[Operand::LargeConstant(0)], // Default property value
                    Some(0),                      // Store to stack
                    None,
                )?;
            }

            IrInstruction::SetProperty {
                object,
                property,
                value,
            } => {
                // Generate property assignment code
                let _obj_operand = self.resolve_ir_id_to_operand(*object)?;
                let _val_operand = self.resolve_ir_id_to_operand(*value)?;

                log::debug!("Setting property '{}' on object", property);

                // Placeholder: property setting needs object system integration
            }

            IrInstruction::GetPropertyByNumber {
                target,
                object,
                property_num,
            } => {
                // Generate numbered property access
                let obj_operand = self.resolve_ir_id_to_operand(*object)?;
                let prop_operand = Operand::LargeConstant(*property_num as u16);

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

            IrInstruction::SetPropertyByNumber {
                object,
                property_num,
                value,
            } => {
                // Generate Z-Machine put_prop instruction (VAR:227, opcode 0x03)
                // Use proper object resolution via global variables
                let operands = vec![
                    self.resolve_ir_id_to_operand(*object)?, // Object (properly resolved)
                    Operand::Constant(*property_num as u16), // Property number
                    self.resolve_ir_id_to_operand(*value)?,  // Value (properly resolved)
                ];
                self.emit_instruction(0x03, &operands, None, None)?;
                log::debug!(
                    "Generated put_prop for property number {} with resolved object",
                    property_num
                );
            }

            IrInstruction::TestProperty {
                target,
                object,
                property_num,
            } => {
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

            IrInstruction::GetNextProperty {
                target,
                object,
                current_property,
            } => {
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

            IrInstruction::ArrayEmpty { target, array: _ } => {
                // Check if array is empty - for now, assume it returns false (non-empty)
                // TODO: Implement proper array empty checking

                // CRITICAL: Register target for array empty result
                self.use_stack_for_result(*target);

                // For now, just load 0 (empty/false) as a placeholder
                self.emit_instruction(
                    0x8D, // load constant 0
                    &[Operand::LargeConstant(0)],
                    Some(0), // Store to stack
                    None,
                )?;
                log::debug!("ArrayEmpty: IR ID {} -> stack (placeholder: false)", target);
            }

            IrInstruction::GetArrayElement {
                target,
                array: _,
                index: _,
            } => {
                // Get array element - for now, return a placeholder value
                // TODO: Implement proper array element access

                // CRITICAL: Register target for array element result
                self.use_stack_for_result(*target);

                // For now, just load placeholder string address
                self.emit_instruction(
                    0x8D,                            // load constant
                    &[Operand::LargeConstant(1000)], // Placeholder string ID
                    Some(0),                         // Store to stack
                    None,
                )?;
                log::debug!(
                    "GetArrayElement: IR ID {} -> stack (placeholder: 1000)",
                    target
                );
            }

            IrInstruction::Nop => {
                // No operation - do nothing
            }

            // Array instructions - placeholder implementations that register target IDs
            IrInstruction::ArrayAdd { array: _, value: _ } => {
                // Array add operation (no return value)
                // TODO: Implement actual array add functionality
                log::debug!("ArrayAdd: placeholder implementation");
            }

            IrInstruction::ArrayRemove {
                target,
                array: _,
                index: _,
            } => {
                // Array remove operation - placeholder returns 0
                self.use_stack_for_result(*target);
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction(0x8F, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:0)
                log::debug!("ArrayRemove: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArrayLength { target, array: _ } => {
                // Array length operation - placeholder returns 0
                self.use_stack_for_result(*target);
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction(0x8F, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:0)
                log::debug!("ArrayLength: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArrayContains {
                target,
                array: _,
                value: _,
            } => {
                // Array contains operation - placeholder returns false (0)
                self.use_stack_for_result(*target);
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction(0x8F, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:0)
                log::debug!("ArrayContains: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArrayIndexOf {
                target,
                array: _,
                value: _,
            } => {
                // Array indexOf operation - placeholder returns -1 (not found)
                self.use_stack_for_result(*target);
                // Emit instruction to push -1 onto stack as placeholder result
                self.emit_instruction(0x8F, &[Operand::LargeConstant(65535)], None, None)?; // push -1 as unsigned
                log::debug!("ArrayIndexOf: IR ID {} -> stack (placeholder: -1)", target);
            }

            IrInstruction::ArrayFilter {
                target,
                array: _,
                predicate: _,
            } => {
                // Array filter operation - placeholder returns empty array (0)
                self.use_stack_for_result(*target);
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction(0x8F, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:0)
                log::debug!("ArrayFilter: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArrayMap {
                target,
                array: _,
                transform: _,
            } => {
                // Array map operation - placeholder returns empty array (0)
                self.use_stack_for_result(*target);
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction(0x8F, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:0)
                log::debug!("ArrayMap: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArrayFind {
                target,
                array: _,
                predicate: _,
            } => {
                // Array find operation - placeholder returns null (0)
                self.use_stack_for_result(*target);
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction(0x8F, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:0)
                log::debug!("ArrayFind: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArrayJoin {
                target,
                array: _,
                separator: _,
            } => {
                // Array join operation - placeholder returns empty string (0)
                self.use_stack_for_result(*target);
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction(0x8F, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:0)
                log::debug!("ArrayJoin: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArrayReverse { target, array: _ } => {
                // Array reverse operation - placeholder returns original array (0)
                self.use_stack_for_result(*target);
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction(0x8F, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:0)
                log::debug!("ArrayReverse: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArraySort {
                target,
                array: _,
                comparator: _,
            } => {
                // Array sort operation - placeholder returns original array (0)
                self.use_stack_for_result(*target);
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction(0x8F, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:0)
                log::debug!("ArraySort: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArrayForEach {
                array: _,
                callback: _,
            } => {
                // Array forEach operation (no return value)
                // TODO: Implement actual array forEach functionality
                log::debug!("ArrayForEach: placeholder implementation");
            }

            IrInstruction::SetArrayElement {
                array: _,
                index: _,
                value: _,
            } => {
                // Set array element operation (no return value)
                // TODO: Implement actual array element setting
                log::debug!("SetArrayElement: placeholder implementation");
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

    /// Emit a Z-Machine instruction with full layout tracking
    ///
    /// This is the core instruction emission function that handles all instruction forms
    /// (Long, Short, Variable) and tracks the exact byte layout of each instruction component.
    /// This precise layout tracking enables accurate reference patching for addresses,
    /// labels, and cross-references without hardcoded offset assumptions.
    ///
    /// # Arguments
    ///
    /// * `opcode` - The Z-Machine opcode (may need form bits applied)
    /// * `operands` - Slice of operands for the instruction
    /// * `store_var` - Optional variable number to store result (None for non-storing instructions)
    /// * `branch_offset` - Optional branch offset for conditional instructions (None for non-branching)
    ///
    /// # Returns
    ///
    /// `InstructionLayout` containing the exact byte locations of instruction components,
    /// or an error if instruction generation fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let layout = self.emit_instruction(0x8D, &[Operand::LargeConstant(placeholder_word())], None, None)?;
    /// // Use layout.operand_location for reference patching instead of current_address - 2
    /// ```
    pub fn emit_instruction(
        &mut self,
        opcode: u8,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<InstructionLayout, CompilerError> {
        let start_address = self.code_address;

        // Comprehensive PC/address tracking for all instructions
        debug!(
            "PC_TRACK: Emitting opcode=0x{:02x} at PC=0x{:04x} operands={:?} store={:?}",
            opcode, start_address, operands, store_var
        );

        // Log stack operations specifically
        for (i, op) in operands.iter().enumerate() {
            if let Operand::Variable(0) = op {
                debug!(
                    "PC_TRACK: Operand[{}] reads from stack at PC=0x{:04x}",
                    i, start_address
                );
            }
        }
        if let Some(0) = store_var {
            debug!(
                "PC_TRACK: Instruction pushes result to stack at PC=0x{:04x}",
                start_address
            );
        }
        // CRITICAL: Detect unimplemented placeholder opcodes at compile time
        if opcode == UNIMPLEMENTED_OPCODE {
            return Err(CompilerError::CodeGenError(format!(
                "UNIMPLEMENTED FEATURE: Opcode 0x{:02x} is a placeholder marker that was not replaced with proper Z-Machine implementation at address 0x{:04x}. This indicates an IR instruction handler needs to be completed with actual Z-Machine opcodes instead of placeholder markers.",
                opcode, self.code_address
            )));
        }

        // CRITICAL: Prevent "Cannot insert object 0" runtime errors by detecting dangerous insert_obj instructions
        if opcode == 0x0E && !operands.is_empty() {
            // This is insert_obj - check if first operand could produce object 0
            match &operands[0] {
                Operand::LargeConstant(0) => {
                    log::debug!(
                        "DANGEROUS: insert_obj with constant object 0 at address 0x{:04x}",
                        self.code_address
                    );
                    return Err(CompilerError::CodeGenError(format!(
                        "DANGEROUS INSTRUCTION: insert_obj with constant object 0 at address 0x{:04x}. Object 0 is invalid and will cause runtime crashes. This indicates a systematic bug in IR->bytecode generation that needs to be fixed.",
                        self.code_address
                    )));
                }
                Operand::Variable(0) => {
                    // Variable 0 is the stack - this is dangerous but temporarily allowing for debugging
                    log::warn!("TEMPORARILY ALLOWING: insert_obj reading object from stack (variable 0) at address 0x{:04x}", self.code_address);
                    log::warn!("         Stack could contain 0, causing 'Cannot insert object 0' error - needs IR generation fix");
                    log::warn!("         This is a temporary bypass to enable address boundary investigation");
                }
                Operand::Variable(var_num) => {
                    // Any variable could contain 0 if not properly initialized
                    log::warn!("POTENTIALLY DANGEROUS: insert_obj reading from variable {} at address 0x{:04x}", var_num, self.code_address);
                    log::warn!("         Variable could contain 0 if not properly initialized, causing runtime 'Cannot insert object 0' error");
                    log::warn!("         Consider using known safe object constants instead of variables for insert_obj operations");
                }
                _ => {
                    log::debug!("insert_obj with operand {:?} - appears safe", operands[0]);
                }
            }
        }
        // Force all store operations to use stack when in init block context
        let actual_store_var = if self.in_init_block && store_var.is_some() && store_var != Some(0)
        {
            log::debug!(
                "Init block context: Forcing store variable {:?} -> stack (0)",
                store_var
            );
            Some(0) // Use stack instead of local variables
        } else {
            store_var
        };

        // COMPREHENSIVE INSTRUCTION GENERATION LOG
        // During final assembly, code_address represents the final runtime address
        // During code generation, we need to calculate what the final address will be
        let final_runtime_address = if !self.final_data.is_empty() {
            // Final assembly phase: code_address is already the runtime address
            self.code_address
        } else {
            // Code generation phase: need to add the base offset for final memory layout
            // final_code_base starts at 0x0b78, so instruction at gen addr 0x0026 -> runtime 0x0b9e
            self.final_code_base + self.code_address
        };

        log::debug!(
            "GEN_INSTR: runtime_addr=0x{:04x} gen_addr=0x{:04x} opcode=0x{:02x} operands={:?} store_var={:?}",
            final_runtime_address, self.code_address, opcode, operands, actual_store_var
        );

        // CRITICAL DEBUG: Track je instructions specifically to verify PC corruption mapping
        if opcode == 0x01 {
            log::debug!(
                " JE_INSTRUCTION_TRACE: runtime_addr=0x{:04x} store_var={:?} branch_offset={:?}",
                final_runtime_address,
                actual_store_var,
                branch_offset
            );
            log::debug!(
                " JE_DETAILS: operands={:?} at code_space[0x{:04x}]",
                operands,
                self.code_address
            );

            // CRITICAL: Print stack trace to find the caller
            log::debug!("    emit_instruction called with opcode 0x01");
        }

        // Record instruction start address
        let instruction_start = self.code_address;

        let form = self.determine_instruction_form_with_operands(operands, opcode);
        log::debug!(
            " FORM_DETERMINATION: opcode=0x{:02x} -> form={:?}",
            opcode,
            form
        );

        let layout = match form {
            InstructionForm::Long => self.emit_long_form_with_layout(
                instruction_start,
                opcode,
                operands,
                actual_store_var,
                branch_offset,
            )?,
            InstructionForm::Short => self.emit_short_form_with_layout(
                instruction_start,
                opcode,
                operands,
                actual_store_var,
                branch_offset,
            )?,
            InstructionForm::Variable => self.emit_variable_form_with_layout(
                instruction_start,
                opcode,
                operands,
                actual_store_var,
                branch_offset,
            )?,
            InstructionForm::Extended => {
                return Err(CompilerError::CodeGenError(
                    "Extended form instructions not yet supported".to_string(),
                ));
            }
        };

        // Track stack operations for debugging
        self.track_stack_operation(opcode, operands, actual_store_var);

        Ok(layout)
    }

    /// Track stack operations for debugging and validation
    fn track_stack_operation(&mut self, opcode: u8, operands: &[Operand], store_var: Option<u8>) {
        // Track stack pushes and pops for common operations
        match opcode {
            // Instructions that push to stack (store result on stack top)
            0x11..=0x13 => {
                // get_prop, get_prop_addr, get_next_prop
                if store_var == Some(0) {
                    self.stack_depth += 1;
                    debug!("Stack push (get_prop*) - depth now: {}", self.stack_depth);
                }
            }
            0x14..=0x18 => {
                // add, sub, mul, div, mod
                if store_var == Some(0) {
                    self.stack_depth += 1;
                    debug!("Stack push (arithmetic) - depth now: {}", self.stack_depth);
                }
            }
            // Instructions that pop from stack
            0x0D => {
                // store
                if let Some(Operand::Variable(0)) = operands.first() {
                    self.stack_depth -= 1;
                    debug!("Stack pop (store) - depth now: {}", self.stack_depth);
                }
            }
            // Function calls affect stack significantly
            0xE0 => {
                // call (VAR form)
                // Function calls consume arguments and push return value
                self.stack_depth -= operands.len() as i32;
                if store_var == Some(0) {
                    self.stack_depth += 1;
                }
                debug!(
                    "Stack after function call - depth now: {}",
                    self.stack_depth
                );
            }
            _ => {
                // For other instructions that might affect stack
                if store_var == Some(0) {
                    self.stack_depth += 1;
                    debug!("Stack push (generic) - depth now: {}", self.stack_depth);
                }
            }
        }

        // Track maximum depth
        if self.stack_depth > self.max_stack_depth {
            self.max_stack_depth = self.stack_depth;
        }

        // Warn about potential stack issues
        if self.stack_depth < 0 {
            debug!(
                "WARNING: Stack underflow detected! Depth: {}",
                self.stack_depth
            );
        }
        if self.stack_depth > 100 {
            debug!(
                "WARNING: Very deep stack detected! Depth: {}",
                self.stack_depth
            );
        }
    }

    /// Check if an opcode is a true VAR opcode (always requires VAR form encoding)
    fn is_true_var_opcode(opcode: u8) -> bool {
        match opcode {
            // Full VAR opcodes (when already combined with VAR form bits)
            0xE0 => true, // CALL_VS (VAR:224 = opcode 0, so 0xE0)
            0xE1 => true, // STOREW (VAR:225 = opcode 1, so 0xE1)
            0xE3 => true, // PUT_PROP (VAR:227 = opcode 3, so 0xE3)
            0xE4 => true, // SREAD (VAR:228 = opcode 4, so 0xE4)
            0xE5 => true, // PRINT_CHAR (VAR:229 = opcode 5, so 0xE5)
            0xE6 => true, // PRINT_NUM (VAR:230 = opcode 6, so 0xE6)
            0xE7 => true, // RANDOM (VAR:231 = opcode 7, so 0xE7)

            // Raw opcodes that should always be VAR form
            0x00 => true, // call_vs (raw opcode 0)
            0x01 => true, // storew (raw opcode 1)
            0x03 => true, // put_prop (raw opcode 3)
            0x04 => true, // sread (raw opcode 4)
            0x05 => true, // print_char (raw opcode 5) - THIS IS THE FIX!
            0x06 => true, // print_num (raw opcode 6)
            0x07 => true, // random (raw opcode 7)

            _ => false,
        }
    }

    /// Determine instruction form based on operand count and opcode
    pub fn determine_instruction_form(&self, operand_count: usize, opcode: u8) -> InstructionForm {
        // Special cases: certain opcodes are always VAR form regardless of operand count
        match opcode {
            0xE0 => InstructionForm::Variable, // call (VAR:224 = opcode 0, full byte 0xE0) is always VAR
            0xE1 => InstructionForm::Variable, // storew (VAR:225 = opcode 1, full byte 0xE1) is always VAR
            0xE3 => InstructionForm::Variable, // put_prop (VAR:227 = opcode 3, full byte 0xE3) is always VAR
            0xE4 => InstructionForm::Variable, // sread (VAR:228 = opcode 4, full byte 0xE4) is always VAR
            0xE5 => InstructionForm::Variable, // print_char (VAR:229 = opcode 5, full byte 0xE5) is always VAR
            0xE6 => InstructionForm::Variable, // print_num (VAR:230 = opcode 6, full byte 0xE6) is always VAR
            0xE7 => InstructionForm::Variable, // random (VAR:231 = opcode 7, full byte 0xE7) is always VAR
            _ => match operand_count {
                0 => InstructionForm::Short, // 0OP
                1 => InstructionForm::Short, // 1OP
                2 => {
                    // Could be 2OP (long form) or VAR form
                    // For now, prefer long form for 2 operands
                    if opcode < 0x80 {
                        InstructionForm::Long
                    } else {
                        InstructionForm::Variable
                    }
                }
                _ => InstructionForm::Variable, // VAR form for 3+ operands
            },
        }
    }

    /// Determine instruction form based on operand count, opcode, and operand constraints
    pub fn determine_instruction_form_with_operands(
        &self,
        operands: &[Operand],
        opcode: u8,
    ) -> InstructionForm {
        // Handle opcodes based on operand count AND context
        match (opcode, operands.len()) {
            // Opcode 0x03: Context-dependent!
            // - 2 operands: jg (jump if greater) - 2OP form
            // - 3 operands: put_prop - VAR form
            (0x03, 2) => {
                // jg (jump if greater) - prefer Long form for 2OP
                let can_use_long_form = operands.iter().all(|op| match op {
                    Operand::LargeConstant(value) => *value <= 255,
                    _ => true,
                });
                if opcode < 0x80 && can_use_long_form {
                    InstructionForm::Long
                } else {
                    InstructionForm::Variable
                }
            }
            (0x03, 3) => InstructionForm::Variable, // put_prop is always VAR

            // Always VAR form opcodes (regardless of operand count)
            (0x04, _) => InstructionForm::Variable, // sread is always VAR
            (0x05, _) => InstructionForm::Variable, // print_char is always VAR
            (0x06, _) => InstructionForm::Variable, // print_num is always VAR
            (0x07, _) => InstructionForm::Variable, // random is always VAR
            (0x20, _) => InstructionForm::Variable, // call_1n is always VAR
            (0xE0, _) => InstructionForm::Variable, // call (VAR:224) is always VAR

            // Default operand-count based logic
            _ => match operands.len() {
                0 => InstructionForm::Short, // 0OP
                1 => InstructionForm::Short, // 1OP
                2 => {
                    // Check if Long form can handle all operands
                    let can_use_long_form = operands.iter().all(|op| {
                        match op {
                            Operand::LargeConstant(value) => *value <= 255,
                            _ => true, // SmallConstant and Variable are fine
                        }
                    });

                    if opcode < 0x80 && can_use_long_form {
                        InstructionForm::Long
                    } else {
                        InstructionForm::Variable
                    }
                }
                _ => InstructionForm::Variable, // VAR form for 3+ operands
            },
        }
    }

    /// Emit long form instruction (2OP)
    fn emit_long_form(
        &mut self,
        opcode: u8,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<(), CompilerError> {
        if operands.len() != 2 {
            return Err(CompilerError::CodeGenError(format!(
                "Long form requires exactly 2 operands, got {}",
                operands.len()
            )));
        }

        // Long form: bit 6 = op1_type, bit 5 = op2_type, bits 4-0 = opcode
        // In Long form: 0 = small constant, 1 = variable (only these 2 types allowed)
        let op1_bit = if matches!(operands[0], Operand::Variable(_)) {
            1
        } else {
            0
        };
        let op2_bit = if matches!(operands[1], Operand::Variable(_)) {
            1
        } else {
            0
        };

        let instruction_byte = (op1_bit << 6) | (op2_bit << 5) | (opcode & 0x1F);
        self.emit_byte(instruction_byte)?;

        // Emit operands
        self.emit_operand(&operands[0])?;
        self.emit_operand(&operands[1])?;

        // Emit store variable if needed
        if let Some(store) = store_var {
            self.emit_byte(store)?;
        }

        // Emit branch offset if needed
        if let Some(offset) = branch_offset {
            self.emit_branch_offset(offset)?;
        }

        Ok(())
    }

    /// Emit short form instruction (0OP or 1OP)
    fn emit_short_form(
        &mut self,
        opcode: u8,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<(), CompilerError> {
        if operands.len() > 1 {
            return Err(CompilerError::CodeGenError(format!(
                "Short form requires 0 or 1 operands, got {}",
                operands.len()
            )));
        }

        let instruction_byte = if operands.is_empty() {
            // 0OP form: bits 7-6 = 11, bits 5-4 = 00, bits 3-0 = opcode
            0xB0 | (opcode & 0x0F)
        } else {
            // 1OP form: bits 7-6 = 10, bits 5-4 = operand type, bits 3-0 = opcode
            let op_type = self.get_operand_type(&operands[0]);
            0x80 | ((op_type as u8) << 4) | (opcode & 0x0F)
        };

        self.emit_byte(instruction_byte)?;

        // Emit operand if present
        if !operands.is_empty() {
            self.emit_operand(&operands[0])?;
        }

        // Emit store variable if needed
        if let Some(store) = store_var {
            self.emit_byte(store)?;
        }

        // Emit branch offset if needed
        if let Some(offset) = branch_offset {
            self.emit_branch_offset(offset)?;
        }

        Ok(())
    }

    /// Emit short form instruction with layout tracking
    ///
    /// This is the layout-aware version of emit_short_form that tracks where
    /// each instruction component is placed for accurate reference resolution.
    fn emit_short_form_with_layout(
        &mut self,
        instruction_start: usize,
        opcode: u8,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<InstructionLayout, CompilerError> {
        if operands.len() > 1 {
            return Err(CompilerError::CodeGenError(format!(
                "Short form requires 0 or 1 operands, got {}",
                operands.len()
            )));
        }

        let instruction_byte = if operands.is_empty() {
            // 0OP form: bits 7-6 = 11, bits 5-4 = 00, bits 3-0 = opcode
            0xB0 | (opcode & 0x0F)
        } else {
            // 1OP form: bits 7-6 = 10, bits 5-4 = operand type, bits 3-0 = opcode
            let op_type = self.get_operand_type(&operands[0]);
            0x80 | ((op_type as u8) << 4) | (opcode & 0x0F)
        };

        self.emit_byte(instruction_byte)?;

        // Track operand location
        let operand_location = if !operands.is_empty() {
            let code_space_offset = self.code_space.len();
            self.emit_operand(&operands[0])?;
            //  FIXED: Convert code space offset to final memory address
            Some(self.final_code_base + code_space_offset)
        } else {
            None
        };

        // Track store variable location
        let store_location = if let Some(store) = store_var {
            let loc = self.code_address;
            self.emit_byte(store)?;
            Some(loc)
        } else {
            None
        };

        // Track branch offset location
        let branch_location = if let Some(offset) = branch_offset {
            let loc = self.code_address;
            self.emit_branch_offset(offset)?;
            Some(loc)
        } else {
            None
        };

        Ok(InstructionLayout {
            instruction_start,
            operand_location,
            store_location,
            branch_location,
            total_size: self.code_address - instruction_start,
        })
    }

    /// Emit variable form instruction (VAR)
    fn emit_variable_form(
        &mut self,
        opcode: u8,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<(), CompilerError> {
        if operands.len() > 4 {
            return Err(CompilerError::CodeGenError(format!(
                "Variable form supports max 4 operands, got {}",
                operands.len()
            )));
        }

        // Variable form: bits 7-6 = 11, bit 5 = VAR (1) or OP2 (0), bits 4-0 = opcode
        // Bit 5 should be set for true VAR opcodes (like RANDOM), regardless of operand count
        let var_bit = if Self::is_true_var_opcode(opcode) {
            0x20
        } else {
            0x00
        };
        let instruction_byte = 0xC0 | var_bit | (opcode & 0x1F);
        debug!("emit_variable_form: opcode=0x{:02x}, var_bit=0x{:02x}, instruction_byte=0x{:02x} at address 0x{:04x}", 
               opcode, var_bit, instruction_byte, self.code_address);
        self.emit_byte(instruction_byte)?;

        // Emit operand types byte
        let mut types_byte = 0u8;
        for (i, operand) in operands.iter().enumerate() {
            let op_type = self.get_operand_type(operand);
            types_byte |= (op_type as u8) << (6 - i * 2);
        }

        // Fill remaining slots with "omitted"
        for i in operands.len()..4 {
            types_byte |= (OperandType::Omitted as u8) << (6 - i * 2);
        }

        self.emit_byte(types_byte)?;

        // Emit operands
        for operand in operands {
            self.emit_operand(operand)?;
        }

        // Emit store variable if needed
        if let Some(store) = store_var {
            self.emit_byte(store)?;
        }

        // Emit branch offset if needed
        if let Some(offset) = branch_offset {
            self.emit_branch_offset(offset)?;
        }

        Ok(())
    }

    /// Emit variable form instruction with layout tracking
    ///
    /// This is the layout-aware version of emit_variable_form that tracks where
    /// each instruction component is placed for accurate reference resolution.
    fn emit_variable_form_with_layout(
        &mut self,
        instruction_start: usize,
        opcode: u8,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<InstructionLayout, CompilerError> {
        log::debug!(
            " VAR_FORM_DEBUG: opcode=0x{:02x}, store_var={:?}, branch_offset={:?}",
            opcode,
            store_var,
            branch_offset
        );
        if operands.len() > 4 {
            return Err(CompilerError::CodeGenError(format!(
                "Variable form supports max 4 operands, got {}",
                operands.len()
            )));
        }

        // Determine if we need VAR (0x20) or VAR2 (0x3C) bit pattern
        let var_bit = if Self::is_true_var_opcode(opcode) {
            0x20
        } else {
            0x00
        };
        let instruction_byte = 0xC0 | var_bit | (opcode & 0x1F);

        debug!("emit_variable_form: opcode=0x{:02x}, var_bit=0x{:02x}, instruction_byte=0x{:02x} at address 0x{:04x}", 
               opcode, var_bit, instruction_byte, self.code_address);

        // CRITICAL DEBUG: Special logging for print_char opcode 0x05
        if opcode == 0x05 {
            log::debug!(" VAR_FORM_0x05: Processing print_char opcode 0x05");
            log::debug!(
                "         is_true_var_opcode(0x05) = {}",
                Self::is_true_var_opcode(opcode)
            );
            log::debug!("         var_bit = 0x{:02x}", var_bit);
            log::debug!(
                "         instruction_byte = 0x{:02x} (should be 0xE5)",
                instruction_byte
            );
            log::debug!(
                "         About to emit instruction_byte = 0x{:02x}",
                instruction_byte
            );
        }

        self.emit_byte(instruction_byte)?;

        // Build operand types byte
        let mut types_byte = 0u8;
        for (i, operand) in operands.iter().enumerate() {
            let op_type = self.get_operand_type(operand);
            types_byte |= (op_type as u8) << (6 - i * 2);
        }

        // Fill remaining operand type slots with "omitted"
        for i in operands.len()..4 {
            types_byte |= (OperandType::Omitted as u8) << (6 - i * 2);
        }

        log::debug!(
            " VAR_TYPES_BYTE: Emitting types_byte=0x{:02x} at address 0x{:04x}",
            types_byte,
            self.code_address
        );
        self.emit_byte(types_byte)?;

        // Track first operand location (most commonly needed for references)
        let operand_location = if !operands.is_empty() {
            // Capture location where first operand data will be written (after opcode and types byte)
            let first_operand_offset = self.code_space.len();

            // Emit all operands
            for operand in operands {
                self.emit_operand(operand)?;
            }

            // Return location of first operand data (not operand types byte)
            Some(self.final_code_base + first_operand_offset)
        } else {
            None
        };

        // Track store variable location
        let store_location = if let Some(store) = store_var {
            let loc = self.code_address;
            self.emit_byte(store)?;
            Some(loc)
        } else {
            None
        };

        // Track branch offset location
        let branch_location = if let Some(offset) = branch_offset {
            let loc = self.code_address;
            self.emit_branch_offset(offset)?;
            Some(loc)
        } else {
            None
        };

        Ok(InstructionLayout {
            instruction_start,
            operand_location,
            store_location,
            branch_location,
            total_size: self.code_address - instruction_start,
        })
    }

    /// Emit long form instruction with layout tracking
    ///
    /// This is the layout-aware version of emit_long_form that tracks where
    /// each instruction component is placed for accurate reference resolution.
    fn emit_long_form_with_layout(
        &mut self,
        instruction_start: usize,
        opcode: u8,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<InstructionLayout, CompilerError> {
        if operands.len() != 2 {
            return Err(CompilerError::CodeGenError(format!(
                "Long form requires exactly 2 operands, got {}",
                operands.len()
            )));
        }

        // Long form can only handle Small Constants and Variables
        // Convert LargeConstants that fit in a byte to SmallConstants
        let op1_adapted = self.adapt_operand_for_long_form(&operands[0])?;
        let op2_adapted = self.adapt_operand_for_long_form(&operands[1])?;

        let op1_type = self.get_operand_type(&op1_adapted);
        let op2_type = self.get_operand_type(&op2_adapted);

        // Long form: bits 7-6 = 00 or 01, bit 6 = op1 type, bit 5 = op2 type, bits 4-0 = opcode
        let op1_bit = if op1_type == OperandType::Variable {
            0x40
        } else {
            0x00
        };
        let op2_bit = if op2_type == OperandType::Variable {
            0x20
        } else {
            0x00
        };
        let instruction_byte = op1_bit | op2_bit | (opcode & 0x1F);

        // Debug: What opcode is trying to be generated as 0x3E?
        if instruction_byte == 0x3E {
            panic!("FOUND THE BUG: Original opcode 0x{:02X} is generating instruction byte 0x3E which decodes to invalid opcode 0x1E. op1_bit=0x{:02X}, op2_bit=0x{:02X}, operands={:?}, address=0x{:04X}", 
                   opcode, op1_bit, op2_bit, operands, self.code_address);
        }

        self.emit_byte(instruction_byte)?;

        // Track first operand location
        let code_space_offset = self.code_space.len();
        //  FIXED: Convert code space offset to final memory address
        let operand_location = Some(self.final_code_base + code_space_offset);

        // Emit adapted operands
        self.emit_operand(&op1_adapted)?;
        self.emit_operand(&op2_adapted)?;

        // Track store variable location
        let store_location = if let Some(store) = store_var {
            let loc = self.code_address;
            self.emit_byte(store)?;
            Some(loc)
        } else {
            None
        };

        // Track branch offset location
        let branch_location = if let Some(offset) = branch_offset {
            let loc = self.code_address;
            self.emit_branch_offset(offset)?;
            Some(loc)
        } else {
            None
        };

        Ok(InstructionLayout {
            instruction_start,
            operand_location,
            store_location,
            branch_location,
            total_size: self.code_address - instruction_start,
        })
    }

    /// Adapt operand for Long form instruction constraints
    /// Long form can only handle Small Constants and Variables
    fn adapt_operand_for_long_form(&self, operand: &Operand) -> Result<Operand, CompilerError> {
        match operand {
            Operand::LargeConstant(value) => {
                if *value <= 255 {
                    // Convert to SmallConstant if it fits
                    Ok(Operand::SmallConstant(*value as u8))
                } else {
                    // Large values require Variable form instruction
                    Err(CompilerError::CodeGenError(format!(
                        "Long form cannot handle large constant {} (> 255). Use Variable form instead.",
                        value
                    )))
                }
            }
            _ => Ok(operand.clone()),
        }
    }

    pub fn get_operand_type(&self, operand: &Operand) -> OperandType {
        match operand {
            Operand::SmallConstant(_) => OperandType::SmallConstant,
            Operand::LargeConstant(_) => OperandType::LargeConstant,
            Operand::Variable(_) => OperandType::Variable,
            Operand::Constant(value) => {
                // Choose optimal encoding based on value
                if *value <= 255 {
                    OperandType::SmallConstant
                } else {
                    OperandType::LargeConstant
                }
            }
        }
    }

    /// Emit a single operand
    fn emit_operand(&mut self, operand: &Operand) -> Result<(), CompilerError> {
        match operand {
            Operand::SmallConstant(value) => {
                self.emit_byte(*value)?;
            }
            Operand::Variable(value) => {
                // CRITICAL FIX: Properly encode Z-Machine variables
                // Variable(0) = stack top (0x00)
                // Variable(1-15) = local variables L01-L15 (0x01-0x0F)
                // Variable(16+) = global variables G00+ (0x10+)
                let zmachine_var = if *value == 0 {
                    0x00 // Stack top
                } else if *value <= 15 {
                    *value // Local variables L01-L15 (0x01-0x0F)
                } else {
                    0x10 + (*value - 16) // Global variables G00+ (0x10+)
                };

                log::debug!(
                    "Variable({}) -> Z-Machine variable 0x{:02x}",
                    value,
                    zmachine_var
                );
                log::trace!(" VARIABLE_EMIT: About to emit Variable({}) as zmachine_var=0x{:02x} at addr=0x{:04x}", 
                           value, zmachine_var, self.final_code_base + self.code_address);
                self.emit_byte(zmachine_var)?;
            }
            Operand::LargeConstant(value) => {
                self.emit_word(*value)?;
            }
            Operand::Constant(value) => {
                // Choose encoding based on value size
                if *value <= 255 {
                    self.emit_byte(*value as u8)?;
                } else {
                    self.emit_word(*value)?;
                }
            }
        }
        Ok(())
    }

    /// Emit branch offset (1 or 2 bytes depending on size)
    pub fn emit_branch_offset(&mut self, offset: i16) -> Result<(), CompilerError> {
        // Z-Machine branch format:
        // - Bit 7: branch condition (1 = branch on true, 0 = branch on false)
        // - Bit 6: 0 = 2-byte offset, 1 = 1-byte offset
        // - Bits 5-0 or 13-0: signed offset

        // For now, assume positive condition and handle offset size
        if (0..=63).contains(&offset) {
            // 1-byte format: bit 7 = condition, bit 6 = 1, bits 5-0 = offset
            let branch_byte = 0x80 | 0x40 | (offset as u8 & 0x3F);
            self.emit_byte(branch_byte)?;
        } else {
            // 2-byte format: bit 7 = condition, bit 6 = 0, bits 13-0 = offset
            let branch_word = 0x8000 | ((offset as u16) & 0x3FFF);
            self.emit_word(branch_word)?;
        }

        Ok(())
    }

    /// Generate user function call with proper UnresolvedReference registration
    fn generate_user_function_call(
        &mut self,
        function_id: crate::grue_compiler::ir::IrId,
        args: &[crate::grue_compiler::ir::IrId],
        target: Option<crate::grue_compiler::ir::IrId>,
    ) -> Result<(), CompilerError> {
        use crate::grue_compiler::codegen::{
            LegacyReferenceType, MemorySpace, UnresolvedReference,
        };

        // Generate function call instruction with placeholder address
        let mut operands = Vec::new();
        operands.push(Operand::LargeConstant(0xFFFF)); // Placeholder for function address

        // Add arguments
        for &arg_id in args {
            let arg_operand = self.resolve_ir_id_to_operand(arg_id)?;
            operands.push(arg_operand);
        }

        // Determine store variable
        let store_var = target.map(|_| 0); // Store to stack if target specified

        // Emit the call instruction (VAR form call_vs)
        let layout = self.emit_instruction(0xE0, &operands, store_var, None)?;

        // CRITICAL: Register function reference for patching
        if let Some(operand_loc) = layout.operand_location {
            self.reference_context
                .unresolved_refs
                .push(UnresolvedReference {
                    reference_type: LegacyReferenceType::FunctionCall,
                    location: operand_loc,
                    target_id: function_id,
                    is_packed_address: true, // Function addresses are packed in Z-Machine
                    offset_size: 2,
                    location_space: MemorySpace::Code,
                });

            log::debug!(
                "Generated call to function ID {} with unresolved reference at 0x{:04x}",
                function_id,
                operand_loc
            );
        }

        Ok(())
    }
}
