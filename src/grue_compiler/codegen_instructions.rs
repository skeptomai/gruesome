// Import placeholder_word for consistent placeholder handling throughout the codebase
use crate::grue_compiler::codegen::{placeholder_word, ConstantValue, ZMachineCodeGen};
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

        // CRITICAL DEBUG: Track all branch and jump instructions
        match instruction {
            IrInstruction::Branch { .. } | IrInstruction::Jump { .. } => {
                eprintln!(
                    "CONTROL FLOW at 0x{:04x}: {:?}",
                    self.code_address, instruction
                );
            }
            _ => {}
        }

        // Store initial code address to detect if label should be processed
        let code_address_before = self.code_address;
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
            IrInstruction::StringIndexOf { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: StringIndexOf creates target IR ID {}",
                    target
                );
            }
            IrInstruction::StringSlice { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: StringSlice creates target IR ID {}",
                    target
                );
            }
            IrInstruction::StringSubstring { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: StringSubstring creates target IR ID {}",
                    target
                );
            }
            IrInstruction::StringToLowerCase { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: StringToLowerCase creates target IR ID {}",
                    target
                );
            }
            IrInstruction::StringToUpperCase { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: StringToUpperCase creates target IR ID {}",
                    target
                );
            }
            IrInstruction::StringTrim { target, .. } => {
                log::debug!("IR INSTRUCTION: StringTrim creates target IR ID {}", target);
            }
            IrInstruction::StringCharAt { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: StringCharAt creates target IR ID {}",
                    target
                );
            }
            IrInstruction::StringSplit { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: StringSplit creates target IR ID {}",
                    target
                );
            }
            IrInstruction::StringReplace { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: StringReplace creates target IR ID {}",
                    target
                );
            }
            IrInstruction::StringStartsWith { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: StringStartsWith creates target IR ID {}",
                    target
                );
            }
            IrInstruction::StringEndsWith { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: StringEndsWith creates target IR ID {}",
                    target
                );
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
                // CRITICAL DEBUG: Track Jump instructions near problem area
                if self.code_address >= 0x330 && self.code_address <= 0x340 {
                    eprintln!(
                        "IR JUMP INSTRUCTION at code_address=0x{:04x}, jumping to label {}",
                        self.code_address, label
                    );
                }
                // Generate a proper jump instruction with correct unresolved reference
                // This delegates to translate_jump which handles the Z-Machine jump encoding
                self.translate_jump(*label)?;
            }

            IrInstruction::Branch {
                condition,
                true_label,
                false_label,
            } => {
                if self.code_address == 0x0001 || self.code_address == 0x32f {
                    eprintln!("CRITICAL BRANCH at code_address=0x{:04x}: condition={}, true_label={}, false_label={}", 
                              self.code_address, condition, true_label, false_label);
                    eprintln!("  Before emit_conditional_branch_instruction");
                }
                let before_addr = self.code_address;
                // Delegate to the proper conditional branch function that handles binary operations
                self.emit_conditional_branch_instruction(*condition, *true_label, *false_label)?;
                let after_addr = self.code_address;
                if before_addr == 0x0001 || before_addr == 0x32f {
                    eprintln!(
                        "  After Branch: moved from 0x{:04x} to 0x{:04x} ({} bytes)",
                        before_addr,
                        after_addr,
                        after_addr - before_addr
                    );
                    eprintln!("  Expected: opcode + 2 operands + 2 branch bytes = 5 bytes");
                    eprintln!("  Actual: {} bytes", after_addr - before_addr);
                    if after_addr - before_addr != 5 {
                        eprintln!("  ERROR: Branch instruction missing branch placeholder!");
                    }
                }
            }

            IrInstruction::LoadVar { target, var_id } => {
                // Create operand for the variable number to load from
                let var_operand = Operand::SmallConstant(*var_id as u8);

                // Use load instruction (0x0E) to load variable to stack
                self.emit_instruction(0x0E, &[var_operand], Some(0), None)?; // load variable to stack

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
                    // Store to local variable using 2OP:13 store instruction
                    let var_operand = Operand::SmallConstant(*var_num);
                    self.emit_instruction(0x0D, &[var_operand, value_operand], None, None)?;
                } else {
                    // Store to stack (variable 0) using 2OP:13 store instruction
                    let stack_operand = Operand::SmallConstant(0);
                    self.emit_instruction(0x0D, &[stack_operand, value_operand], None, None)?;
                    self.ir_id_to_stack_var.insert(*var_id, 0);
                }
                log::debug!(
                    "StoreVar: IR ID {} stored value from IR ID {}",
                    var_id,
                    source
                );
            }

            IrInstruction::Label { id } => {
                // CRITICAL FIX: Labels should point to the NEXT instruction, not current position
                // The label address will be recorded when the next instruction is processed
                // This ensures labels point to actual executable code, not empty space
                log::debug!(
                    "Label {} encountered at code_address=0x{:04x} - deferring address recording until next instruction",
                    id, self.code_address
                );

                // Store the label ID to be processed when the next instruction is emitted
                // Multiple labels can be pending at the same address (e.g., converging control flow)
                self.pending_labels.push(*id);
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

                // For empty arrays, push null reference to stack
                if size_value == 0 {
                    // Empty array - push null reference (1) to stack
                    // Using load_w with address 0 to get a safe null-like value
                    self.emit_instruction(
                        0x0F,                         // load_w (1OP:15) - load word from address
                        &[Operand::SmallConstant(1)], // Load from address 1 (safe)
                        Some(0),                      // Store result to stack
                        None,
                    )?;
                } else {
                    // Non-empty arrays not yet implemented
                    return Err(CompilerError::CodeGenError(format!(
                        "CreateArray with non-zero size ({}) not yet implemented. Only empty arrays [] are supported.",
                        size_value
                    )));
                }
            }

            IrInstruction::GetProperty {
                target,
                object,
                property,
            } => {
                // Generate property access code using get_prop instruction
                let obj_operand = self.resolve_ir_id_to_operand(*object)?;

                // Map property name to property number using the global property registry
                let prop_num = *self.property_numbers.get(property).ok_or_else(|| {
                    CompilerError::CodeGenError(format!(
                        "Unknown property '{}' in GetProperty (not found in registry)",
                        property
                    ))
                })?;

                log::debug!(
                    "Getting property '{}' (#{}) from object",
                    property,
                    prop_num
                );

                // CRITICAL: Register target for property result
                self.use_stack_for_result(*target);

                // PROPERTY ACCESS CORRECTION (Sept 28, 2025): Fixed branch out of bounds bug
                //
                // BUG DISCOVERY: The "property fix" commit 604f7b4 incorrectly changed get_prop
                // from 0x11 to 0x01, causing all property access to generate branch out of bounds errors.
                //
                // Z-MACHINE SPECIFICATION (sect15.html):
                // - get_prop (0x11): 2OP:17 - Returns property VALUE (correct for property access)
                // - je (0x01): 2OP:1 - Jump if equal with BRANCHING (wrong instruction entirely!)
                //
                // ROOT CAUSE ANALYSIS:
                // 1. Changed get_prop opcode from 0x11 → 0x01 in commit 604f7b4
                // 2. This made property access use je (jump-if-equal) instead of get_prop
                // 3. je is a branching instruction that generates branch data
                // 4. Branch targets calculated incorrectly, causing out-of-bounds jumps
                // 5. All property access programs crashed with "Branch to address 0xXXX is outside memory bounds"
                //
                // RESOLUTION: Reverted to correct opcode 0x11 (get_prop per Z-Machine spec)
                // IMPACT: Property access now works without branch errors, mini_zork reaches command prompt
                self.emit_instruction(
                    0x11, // get_prop (2OP:17) - returns property value, not address
                    &[obj_operand, Operand::SmallConstant(prop_num)],
                    Some(0), // Store to stack
                    None,
                )?;
                log::debug!("GetProperty: IR ID {} -> stack", target);
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

                // PROPERTY ACCESS CORRECTION (Sept 28, 2025): Fixed branch out of bounds bug
                // Same fix as GetProperty above - reverted from 0x01 (je) back to 0x11 (get_prop).
                // This handles numbered property access (property_num instead of property name).
                self.emit_instruction(
                    0x11, // get_prop (2OP:17) - returns property value, not address
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
                self.emit_instruction(0xE8, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:8)
                log::debug!("ArrayRemove: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArrayLength { target, array: _ } => {
                // Array length operation - placeholder returns 0
                self.use_stack_for_result(*target);
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction(0xE8, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:8)
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
                self.emit_instruction(0xE8, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:8)
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
                self.emit_instruction(0xE8, &[Operand::LargeConstant(65535)], None, None)?; // push -1 as unsigned
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
                self.emit_instruction(0xE8, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:8)
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
                self.emit_instruction(0xE8, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:8)
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
                self.emit_instruction(0xE8, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:8)
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
                self.emit_instruction(0xE8, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:8)
                log::debug!("ArrayJoin: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArrayReverse { target, array: _ } => {
                // Array reverse operation - placeholder returns original array (0)
                self.use_stack_for_result(*target);
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction(0xE8, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:8)
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
                self.emit_instruction(0xE8, &[Operand::SmallConstant(0)], None, None)?; // push (VAR:8)
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

            IrInstruction::StringIndexOf {
                target,
                string,
                substring,
            } => {
                // COMPILE-TIME ONLY: String operations only work with compile-time constants
                log::debug!("StringIndexOf: attempting compile-time evaluation");

                match (
                    self.get_compile_time_string(*string),
                    self.get_compile_time_string(*substring),
                ) {
                    (Some(haystack), Some(needle)) => {
                        let result = match haystack.find(&needle) {
                            Some(pos) => pos as i16,
                            None => -1,
                        };
                        log::debug!(
                            "StringIndexOf: compile-time result '{}' in '{}' = {}",
                            needle,
                            haystack,
                            result
                        );
                        self.ir_id_to_integer.insert(*target, result);
                        self.constant_values
                            .insert(*target, ConstantValue::Integer(result));
                    }
                    _ => {
                        log::warn!(
                            "StringIndexOf: runtime string operations not supported - returning -1"
                        );
                        self.ir_id_to_integer.insert(*target, -1);
                        self.constant_values
                            .insert(*target, ConstantValue::Integer(-1));
                    }
                }
            }

            IrInstruction::StringSlice {
                target,
                string,
                start,
            } => {
                // COMPILE-TIME ONLY: String operations only work with compile-time constants
                log::debug!("StringSlice: attempting compile-time evaluation");

                match (
                    self.get_compile_time_string(*string),
                    self.get_compile_time_integer(*start),
                ) {
                    (Some(text), Some(start_idx)) => {
                        let start_pos = start_idx.max(0) as usize;
                        let result = if start_pos >= text.len() {
                            String::new()
                        } else {
                            text[start_pos..].to_string()
                        };
                        log::debug!(
                            "StringSlice: compile-time result slice('{}', {}) = '{}'",
                            text,
                            start_idx,
                            result
                        );
                        self.ir_id_to_string.insert(*target, result.clone());
                        self.constant_values
                            .insert(*target, ConstantValue::String(result));
                    }
                    _ => {
                        log::warn!("StringSlice: runtime string operations not supported - returning empty string");
                        self.ir_id_to_string.insert(*target, String::new());
                        self.constant_values
                            .insert(*target, ConstantValue::String(String::new()));
                    }
                }
            }

            IrInstruction::StringSubstring {
                target,
                string,
                start,
                end,
            } => {
                // COMPILE-TIME ONLY: String operations only work with compile-time constants
                log::debug!("StringSubstring: attempting compile-time evaluation");

                match (
                    self.get_compile_time_string(*string),
                    self.get_compile_time_integer(*start),
                    self.get_compile_time_integer(*end),
                ) {
                    (Some(text), Some(start_idx), Some(end_idx)) => {
                        let start_pos = start_idx.max(0) as usize;
                        let end_pos = end_idx.max(0) as usize;
                        let result = if start_pos >= text.len() || end_pos <= start_pos {
                            String::new()
                        } else {
                            let actual_end = end_pos.min(text.len());
                            text[start_pos..actual_end].to_string()
                        };
                        log::debug!(
                            "StringSubstring: compile-time result substring('{}', {}, {}) = '{}'",
                            text,
                            start_idx,
                            end_idx,
                            result
                        );
                        self.ir_id_to_string.insert(*target, result.clone());
                        self.constant_values
                            .insert(*target, ConstantValue::String(result));
                    }
                    _ => {
                        log::warn!("StringSubstring: runtime string operations not supported - returning empty string");
                        self.ir_id_to_string.insert(*target, String::new());
                        self.constant_values
                            .insert(*target, ConstantValue::String(String::new()));
                    }
                }
            }

            IrInstruction::StringToLowerCase { target, string } => {
                // COMPILE-TIME ONLY: String operations only work with compile-time constants
                log::debug!("StringToLowerCase: attempting compile-time evaluation");

                match self.get_compile_time_string(*string) {
                    Some(text) => {
                        let result = text.to_lowercase();
                        log::debug!(
                            "StringToLowerCase: compile-time result toLowerCase('{}') = '{}'",
                            text,
                            result
                        );
                        self.ir_id_to_string.insert(*target, result.clone());
                        self.constant_values
                            .insert(*target, ConstantValue::String(result));
                    }
                    _ => {
                        log::warn!("StringToLowerCase: runtime string operations not supported - returning empty string");
                        self.ir_id_to_string.insert(*target, String::new());
                        self.constant_values
                            .insert(*target, ConstantValue::String(String::new()));
                    }
                }
            }

            IrInstruction::StringToUpperCase { target, string } => {
                // COMPILE-TIME ONLY: String operations only work with compile-time constants
                log::debug!("StringToUpperCase: attempting compile-time evaluation");

                match self.get_compile_time_string(*string) {
                    Some(text) => {
                        let result = text.to_uppercase();
                        log::debug!(
                            "StringToUpperCase: compile-time result toUpperCase('{}') = '{}'",
                            text,
                            result
                        );
                        self.ir_id_to_string.insert(*target, result.clone());
                        self.constant_values
                            .insert(*target, ConstantValue::String(result));
                    }
                    _ => {
                        log::warn!("StringToUpperCase: runtime string operations not supported - returning empty string");
                        self.ir_id_to_string.insert(*target, String::new());
                        self.constant_values
                            .insert(*target, ConstantValue::String(String::new()));
                    }
                }
            }

            IrInstruction::StringTrim { target, string } => {
                // COMPILE-TIME ONLY: String operations only work with compile-time constants
                log::debug!("StringTrim: attempting compile-time evaluation");

                match self.get_compile_time_string(*string) {
                    Some(text) => {
                        let result = text.trim().to_string();
                        log::debug!(
                            "StringTrim: compile-time result trim('{}') = '{}'",
                            text,
                            result
                        );
                        self.ir_id_to_string.insert(*target, result.clone());
                        self.constant_values
                            .insert(*target, ConstantValue::String(result));
                    }
                    _ => {
                        log::warn!("StringTrim: runtime string operations not supported - returning empty string");
                        self.ir_id_to_string.insert(*target, String::new());
                        self.constant_values
                            .insert(*target, ConstantValue::String(String::new()));
                    }
                }
            }

            IrInstruction::StringCharAt {
                target,
                string,
                index,
            } => {
                // COMPILE-TIME ONLY: String operations only work with compile-time constants
                log::debug!("StringCharAt: attempting compile-time evaluation");

                match (
                    self.get_compile_time_string(*string),
                    self.get_compile_time_integer(*index),
                ) {
                    (Some(text), Some(idx)) => {
                        let result = if idx >= 0 && (idx as usize) < text.len() {
                            text.chars().nth(idx as usize).unwrap_or('\0').to_string()
                        } else {
                            String::new()
                        };
                        log::debug!(
                            "StringCharAt: compile-time result charAt('{}', {}) = '{}'",
                            text,
                            idx,
                            result
                        );
                        self.ir_id_to_string.insert(*target, result.clone());
                        self.constant_values
                            .insert(*target, ConstantValue::String(result));
                    }
                    _ => {
                        log::warn!("StringCharAt: runtime string operations not supported - returning empty string");
                        self.ir_id_to_string.insert(*target, String::new());
                        self.constant_values
                            .insert(*target, ConstantValue::String(String::new()));
                    }
                }
            }

            IrInstruction::StringSplit {
                target,
                string: _,
                delimiter: _,
            } => {
                // For now, implement as a placeholder that returns empty array
                // TODO: Implement actual string split functionality
                log::debug!("StringSplit: placeholder implementation returning empty array");

                // This should return an array, but for now treat as an integer (array length 0)
                self.ir_id_to_integer.insert(*target, 0);
                self.constant_values
                    .insert(*target, ConstantValue::Integer(0));
            }

            IrInstruction::StringReplace {
                target,
                string,
                search,
                replacement,
            } => {
                // COMPILE-TIME ONLY: String operations only work with compile-time constants
                log::debug!("StringReplace: attempting compile-time evaluation");

                match (
                    self.get_compile_time_string(*string),
                    self.get_compile_time_string(*search),
                    self.get_compile_time_string(*replacement),
                ) {
                    (Some(text), Some(search_str), Some(replacement_str)) => {
                        let result = text.replace(&search_str, &replacement_str);
                        log::debug!(
                            "StringReplace: compile-time result replace('{}', '{}', '{}') = '{}'",
                            text,
                            search_str,
                            replacement_str,
                            result
                        );
                        self.ir_id_to_string.insert(*target, result.clone());
                        self.constant_values
                            .insert(*target, ConstantValue::String(result));
                    }
                    _ => {
                        log::warn!("StringReplace: runtime string operations not supported - returning empty string");
                        self.ir_id_to_string.insert(*target, String::new());
                        self.constant_values
                            .insert(*target, ConstantValue::String(String::new()));
                    }
                }
            }

            IrInstruction::StringStartsWith {
                target,
                string,
                prefix,
            } => {
                // COMPILE-TIME ONLY: String operations only work with compile-time constants
                log::debug!("StringStartsWith: attempting compile-time evaluation");

                match (
                    self.get_compile_time_string(*string),
                    self.get_compile_time_string(*prefix),
                ) {
                    (Some(text), Some(prefix_str)) => {
                        let result = if text.starts_with(&prefix_str) { 1 } else { 0 };
                        log::debug!(
                            "StringStartsWith: compile-time result startsWith('{}', '{}') = {}",
                            text,
                            prefix_str,
                            result == 1
                        );
                        self.ir_id_to_integer.insert(*target, result);
                        self.constant_values
                            .insert(*target, ConstantValue::Integer(result));
                    }
                    _ => {
                        log::warn!("StringStartsWith: runtime string operations not supported - returning false");
                        self.ir_id_to_integer.insert(*target, 0);
                        self.constant_values
                            .insert(*target, ConstantValue::Integer(0));
                    }
                }
            }

            IrInstruction::StringEndsWith {
                target,
                string,
                suffix,
            } => {
                // COMPILE-TIME ONLY: String operations only work with compile-time constants
                log::debug!("StringEndsWith: attempting compile-time evaluation");

                match (
                    self.get_compile_time_string(*string),
                    self.get_compile_time_string(*suffix),
                ) {
                    (Some(text), Some(suffix_str)) => {
                        let result = if text.ends_with(&suffix_str) { 1 } else { 0 };
                        log::debug!(
                            "StringEndsWith: compile-time result endsWith('{}', '{}') = {}",
                            text,
                            suffix_str,
                            result == 1
                        );
                        self.ir_id_to_integer.insert(*target, result);
                        self.constant_values
                            .insert(*target, ConstantValue::Integer(result));
                    }
                    _ => {
                        log::warn!("StringEndsWith: runtime string operations not supported - returning false");
                        self.ir_id_to_integer.insert(*target, 0);
                        self.constant_values
                            .insert(*target, ConstantValue::Integer(0));
                    }
                }
            }

            _ => {
                // Handle other instruction types that are not yet implemented in the extracted code
                return Err(CompilerError::CodeGenError(format!(
                    "Instruction type {:?} not implemented in extracted generate_instruction",
                    instruction
                )));
            }
        }

        // CRITICAL FIX: Process pending labels AFTER instruction is emitted
        // This ensures labels point to the actual instruction location
        // Multiple labels can be pending at the same address (converging control flow)
        if !self.pending_labels.is_empty() {
            // Only process if the instruction actually emitted code (address changed)
            if self.code_address > code_address_before {
                let label_address = code_address_before; // Use the address where instruction started

                // Process all pending labels - they all point to the same address
                let labels_to_process: Vec<crate::grue_compiler::ir::IrId> =
                    self.pending_labels.drain(..).collect();
                for label_id in labels_to_process {
                    log::debug!(
                        "DEFERRED_LABEL_AFTER: Processing pending label {} at instruction_start=0x{:04x} (after instruction: {:?})",
                        label_id, label_address, instruction
                    );

                    // Record the address manually instead of using allocate_label_address which uses current position
                    self.label_addresses.insert(label_id, label_address);
                    self.record_final_address(label_id, label_address);
                }
            } else {
                // Instruction didn't emit code, keep labels pending for next instruction
                log::debug!(
                    "DEFERRED_LABEL_SKIP: {} labels deferred again - instruction {:?} didn't emit code",
                    self.pending_labels.len(), instruction
                );
            }
        }

        Ok(())
    }

    /// Helper method to get compile-time string value for an IR ID
    /// Returns Some(string) if the IR ID represents a compile-time constant string,
    /// None if it's a runtime variable or not a string
    fn get_compile_time_string(&self, ir_id: crate::grue_compiler::ir::IrId) -> Option<String> {
        // Check if it's a string literal
        if let Some(string_value) = self.ir_id_to_string.get(&ir_id) {
            return Some(string_value.clone());
        }

        // Check constant values mapping
        if let Some(constant_value) = self.constant_values.get(&ir_id) {
            if let ConstantValue::String(s) = constant_value {
                return Some(s.clone());
            }
        }

        None
    }

    /// Helper method to get compile-time integer value for an IR ID
    fn get_compile_time_integer(&self, ir_id: crate::grue_compiler::ir::IrId) -> Option<i16> {
        // Check if it's an integer constant
        if let Some(int_value) = self.ir_id_to_integer.get(&ir_id) {
            return Some(*int_value);
        }

        // Check constant values mapping
        if let Some(constant_value) = self.constant_values.get(&ir_id) {
            if let ConstantValue::Integer(i) = constant_value {
                return Some(*i);
            }
        }

        None
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

        // Log unimplemented opcodes for debugging
        if opcode == 0x00 {
            log::error!(
                "🚨 OPCODE_0x00_DETECTED: Emitting opcode 0x00 at address 0x{:04x} with operands: {:?}, store_var: {:?}",
                start_address,
                operands,
                store_var
            );
            // Print stack trace to see where this comes from
            log::error!("🚨 STACK_TRACE: This 0x00 opcode emission comes from:");
        }

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
                _ => {
                    log::debug!(
                        "insert_obj with operand {:?} at address 0x{:04x}",
                        operands[0],
                        self.code_address
                    );
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
            0x21 => {
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
    fn should_not_emit_store_variable(opcode: u8) -> bool {
        match opcode {
            // Call instructions that don't store results (void calls)
            0x20 => true, // call_1n - no return value
            0x8F => true, // call_1n (1OP form) - no return value
            0x1A => true, // call_2n - no return value

            // IMPORTANT: call_vs (0x00/0xE0) stores results and MUST emit store variable byte!

            // Print instructions - no result to store
            0x8D => true, // print_paddr (1OP:141)
            0x8A => true, // print_obj (1OP:138)
            0x87 => true, // print_addr (1OP:135)
            0xE5 => true, // print_char (VAR:229)
            0xE6 => true, // print_num (VAR:230)
            0xB3 => true, // print_ret (0OP:179)
            0xBB => true, // new_line (0OP:187)

            // Other instructions that don't store results
            0xBA => true, // quit (0OP:186)

            _ => false,
        }
    }

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
            (0x8b, _) => InstructionForm::Variable, // quit (0OP:139) - too large for short form
            (0x8f, _) => InstructionForm::Variable, // call_1n (1OP:143) - too large for short form
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

        // Emit branch placeholder if needed (resolved later via UnresolvedReference)
        if let Some(_offset) = branch_offset {
            // INSTRUMENT: What value are we actually passing to emit_word?
            if self.code_address >= 0x335 && self.code_address <= 0x340 {
                eprintln!(
                    "CRITICAL: About to emit_word at 0x{:04x} with value 0xFFFF",
                    self.code_address
                );
                eprintln!("  But wait, let's check if this is actually being called...");
            }

            // Always emit 2-byte placeholder for branches to be resolved later
            self.emit_word(placeholder_word())?; // Will be replaced during branch resolution
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

        // Emit branch placeholder if needed (resolved later via UnresolvedReference)
        if let Some(_offset) = branch_offset {
            // INSTRUMENT: What value are we actually passing to emit_word?
            if self.code_address >= 0x335 && self.code_address <= 0x340 {
                eprintln!(
                    "CRITICAL: About to emit_word at 0x{:04x} with value 0xFFFF",
                    self.code_address
                );
                eprintln!("  But wait, let's check if this is actually being called...");
            }

            // Always emit 2-byte placeholder for branches to be resolved later
            self.emit_word(placeholder_word())?; // Will be replaced during branch resolution
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
        // DEBUG jz at critical location
        if opcode == 0x00 && self.code_address == 0x032f {
            eprintln!("JZ_CRITICAL: Emitting jz at 0x032f");
            eprintln!(
                "  operands={:?}, store_var={:?}, branch_offset={:?}",
                operands, store_var, branch_offset
            );
        }
        // CRITICAL DEBUG: Track short form emission at 0x338-0x339
        if self.code_address >= 0x337 && self.code_address <= 0x33a {
            eprintln!(
                "emit_short_form_with_layout at code_address=0x{:04x}",
                self.code_address
            );
            eprintln!("  opcode=0x{:02x}", opcode);
            eprintln!("  operands={:?}", operands);
            eprintln!("  store_var={:?}", store_var);
            eprintln!("  branch_offset={:?}", branch_offset);

            if !operands.is_empty() {
                if let Operand::SmallConstant(val) = &operands[0] {
                    if *val == 159 {
                        eprintln!("FOUND THE CULPRIT: Short form instruction with operand 159!");
                        eprintln!("This will emit 0x9f as the operand");
                    }
                }
            }
        }

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
        // CRITICAL FIX: Some instructions do NOT emit store variable bytes
        let store_location = if let Some(store) = store_var {
            if !Self::should_not_emit_store_variable(opcode) {
                let loc = self.code_address;
                self.emit_byte(store)?;
                Some(loc)
            } else {
                None
            }
        } else {
            None
        };

        // Handle branch encoding - distinguish between hardcoded offsets and label references
        let branch_location = if let Some(offset) = branch_offset {
            // Check if this is a small hardcoded offset (0-63) that can be encoded directly
            if offset >= 0 && offset <= 63 {
                log::debug!(
                    "BRANCH_DIRECT: Encoding hardcoded offset {} directly as single byte",
                    offset
                );
                // Encode as single-byte branch: bit 7=0, bit 6=1 (branch on true), offset in bits 0-5
                let branch_byte = 0x40 | (offset as u8 & 0x3F);
                self.emit_byte(branch_byte)?;
                None // No placeholder needed
            } else {
                // This is either a large offset or a label reference - emit placeholder
                let loc = self.code_address;
                log::debug!("BRANCH_PLACEHOLDER: Emitting 0xFFFF at code_address=0x{:04x} for branch (offset={})", loc, offset);
                self.emit_word(placeholder_word())?; // Will be replaced during branch resolution
                Some(loc)
            }
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

        // Emit branch placeholder if needed (resolved later via UnresolvedReference)
        if let Some(_offset) = branch_offset {
            // INSTRUMENT: What value are we actually passing to emit_word?
            if self.code_address >= 0x335 && self.code_address <= 0x340 {
                eprintln!(
                    "CRITICAL: About to emit_word at 0x{:04x} with value 0xFFFF",
                    self.code_address
                );
                eprintln!("  But wait, let's check if this is actually being called...");
            }

            // Always emit 2-byte placeholder for branches to be resolved later
            self.emit_word(placeholder_word())?; // Will be replaced during branch resolution
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

        log::debug!(
            "VAR instruction: opcode=0x{:02x} at address 0x{:04x}",
            opcode,
            self.code_address
        );

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
        // CRITICAL FIX: Some instructions do NOT emit store variable bytes
        // Print instructions, call instructions, etc. handle results differently
        let store_location = if let Some(store) = store_var {
            if !Self::should_not_emit_store_variable(opcode) {
                let loc = self.code_address;
                self.emit_byte(store)?;
                Some(loc)
            } else {
                None
            }
        } else {
            None
        };

        // Handle branch encoding - distinguish between hardcoded offsets and label references
        let branch_location = if let Some(offset) = branch_offset {
            // Check if this is a small hardcoded offset (0-63) that can be encoded directly
            if offset >= 0 && offset <= 63 {
                log::debug!(
                    "BRANCH_DIRECT: Encoding hardcoded offset {} directly as single byte",
                    offset
                );
                // Encode as single-byte branch: bit 7=0, bit 6=1 (branch on true), offset in bits 0-5
                let branch_byte = 0x40 | (offset as u8 & 0x3F);
                self.emit_byte(branch_byte)?;
                None // No placeholder needed
            } else {
                // This is either a large offset or a label reference - emit placeholder
                let loc = self.code_address;
                log::debug!("BRANCH_PLACEHOLDER: Emitting 0xFFFF at code_address=0x{:04x} for branch (offset={})", loc, offset);
                self.emit_word(placeholder_word())?; // Will be replaced during branch resolution
                Some(loc)
            }
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
        // CRITICAL CHECK: Are we emitting an instruction with operand 415?
        if self.code_address >= 0x334 && self.code_address <= 0x340 {
            eprintln!(
                "emit_long_form_with_layout at 0x{:04x}: opcode=0x{:02x}, operands={:?}",
                self.code_address, opcode, operands
            );

            for (i, op) in operands.iter().enumerate() {
                match op {
                    Operand::LargeConstant(415) | Operand::Constant(415) => {
                        panic!(
                            "BUG FOUND: Operand {} is value 415 at code_address 0x{:04x}!",
                            i, self.code_address
                        );
                    }
                    Operand::SmallConstant(1) if self.code_address == 0x335 => {
                        eprintln!(
                            "WARNING: SmallConstant(1) at critical location - might be part of 415"
                        );
                    }
                    _ => {}
                }
            }
        }
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

        log::debug!(
            "LONG_FORM_EMIT: About to emit instruction_byte=0x{:02x} at code_address=0x{:04x}",
            instruction_byte,
            self.code_address
        );
        self.emit_byte(instruction_byte)?;
        log::debug!(
            "LONG_FORM_EMIT: After instruction byte, code_address=0x{:04x}",
            self.code_address
        );

        // Track first operand location
        let code_space_offset = self.code_space.len();
        //  FIXED: Convert code space offset to final memory address
        let operand_location = Some(self.final_code_base + code_space_offset);

        // Emit adapted operands
        log::debug!(
            "LONG_FORM_EMIT: About to emit operand1={:?} at code_address=0x{:04x}",
            op1_adapted,
            self.code_address
        );
        self.emit_operand(&op1_adapted)?;
        log::debug!(
            "LONG_FORM_EMIT: After operand1, code_address=0x{:04x}",
            self.code_address
        );

        log::debug!(
            "LONG_FORM_EMIT: About to emit operand2={:?} at code_address=0x{:04x}",
            op2_adapted,
            self.code_address
        );
        self.emit_operand(&op2_adapted)?;
        log::debug!(
            "LONG_FORM_EMIT: After operand2, code_address=0x{:04x}",
            self.code_address
        );

        // Track store variable location
        // CRITICAL FIX: Some instructions do NOT emit store variable bytes
        let store_location = if let Some(store) = store_var {
            if !Self::should_not_emit_store_variable(opcode) {
                let loc = self.code_address;
                self.emit_byte(store)?;
                Some(loc)
            } else {
                None
            }
        } else {
            None
        };

        // Handle branch encoding - distinguish between hardcoded offsets and label references
        let branch_location = if let Some(offset) = branch_offset {
            // Check if this is a small hardcoded offset (0-63) that can be encoded directly
            if offset >= 0 && offset <= 63 {
                log::debug!(
                    "BRANCH_DIRECT: Encoding hardcoded offset {} directly as single byte",
                    offset
                );
                // Encode as single-byte branch: bit 7=0, bit 6=1 (branch on true), offset in bits 0-5
                let branch_byte = 0x40 | (offset as u8 & 0x3F);
                self.emit_byte(branch_byte)?;
                None // No placeholder needed
            } else {
                // This is either a large offset or a label reference - emit placeholder
                let loc = self.code_address;
                log::debug!("BRANCH_PLACEHOLDER: Emitting 0xFFFF at code_address=0x{:04x} for branch (offset={})", loc, offset);
                self.emit_word(placeholder_word())?; // Will be replaced during branch resolution
                Some(loc)
            }
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
                // CRITICAL ARCHITECTURAL DECISION: Do NOT attempt IR ID resolution here
                //
                // REASONING: By the time operands reach emit_instruction(), all IR IDs should
                // already be resolved to their final operand values. The previous logic attempted
                // to "second-guess" whether a LargeConstant(N) might actually be an unresolved
                // IR ID N, but this created systematic bugs:
                //
                // BUG EXAMPLE:
                // 1. IR ID 5 correctly resolves to LargeConstant(1) (literal constant 1)
                // 2. emit_instruction receives operands=[Variable(0), LargeConstant(1)]
                // 3. adapt_operand_for_long_form sees LargeConstant(1) and incorrectly thinks
                //    "maybe this 1 is actually unresolved IR ID 1"
                // 4. Attempts to resolve IR ID 1 → gets function address → emits 0xFFFF placeholder
                // 5. Result: je instruction gets 0xFF 0xFF instead of correct operands
                //
                // CONCLUSION: If an operand reaches this point, it's already fully resolved.
                // Any remaining IR ID resolution should have happened earlier in the pipeline.
                //
                // REMOVED: try_resolve_ir_id_if_needed() call that caused LargeConstant(1) → 0xFFFF bug

                if *value <= 255 {
                    // Convert to SmallConstant if it fits (only for resolved constants)
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

    /// Check if an IR ID has unresolved references pending
    fn has_unresolved_reference(&self, ir_id: u32) -> bool {
        self.reference_context
            .unresolved_refs
            .iter()
            .any(|r| r.target_id == ir_id)
    }

    /// Try to resolve a value as an IR ID if it has a mapping
    /// Returns Some(resolved_operand) if it was an IR ID, None if it's a literal constant
    fn try_resolve_ir_id_if_needed(&self, value: u32) -> Option<Operand> {
        // Try to resolve this value as an IR ID
        if let Ok(resolved_operand) = self.resolve_ir_id_to_operand(value) {
            // If it resolved to something different than LargeConstant(value),
            // then it was actually an IR ID that needed resolution
            match &resolved_operand {
                Operand::LargeConstant(resolved_value) if *resolved_value == value as u16 => {
                    // It resolved to the same value - probably a literal constant
                    None
                }
                _ => {
                    // It resolved to something different - it was an IR ID that needed resolution
                    log::debug!("AUTO-RESOLVED: IR ID {} -> {:?}", value, resolved_operand);
                    Some(resolved_operand)
                }
            }
        } else {
            // Couldn't resolve - probably a literal constant
            None
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
        // CRITICAL CHECK
        if self.code_address >= 0x336 && self.code_address <= 0x33a {
            eprintln!(
                "emit_operand at code_address=0x{:04x}: operand={:?}",
                self.code_address, operand
            );
        }

        match operand {
            Operand::SmallConstant(value) => {
                if *value == 1 && self.code_address == 0x338 {
                    eprintln!("CRITICAL: Emitting SmallConstant(1) at 0x338");
                    eprintln!("This is part of the 415 problem!");
                }
                if *value == 159 && self.code_address == 0x339 {
                    eprintln!("CRITICAL: Emitting SmallConstant(159) at 0x339");
                    eprintln!("Together with previous byte, this is 0x019f = 415!");
                }

                // CRITICAL CHECK: Looking for what makes the 0x9f byte
                if self.code_address == 0x339 {
                    eprintln!(
                        "FOUND: About to emit byte 0x{:02x} at code_address=0x339",
                        value
                    );
                    if *value == 0x9f {
                        eprintln!("THIS IS THE 0x9f BYTE! SmallConstant(159)");
                    }
                }

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
                // CRITICAL DEBUG: Check if this is the problematic LargeConstant(1)
                if *value == 1 && self.code_address >= 0x336 && self.code_address <= 0x340 {
                    eprintln!(
                        "CRITICAL: emit_operand emitting LargeConstant(1) at code_address=0x{:04x}",
                        self.code_address
                    );
                    eprintln!("This should emit 0x00 0x01 but might be causing 0x01 0x9f");
                    eprintln!("About to call emit_word(0x{:04x})", value);
                }
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

    // NOTE: emit_branch_offset was removed as dead code.
    // Branch offsets are now handled through placeholder emission and fixup
    // during the resolve_references phase, which is the correct pattern.
    // The old function was only used in tests and didn't follow the
    // established pattern of emit placeholder -> record reference -> fixup later.

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
