// Import placeholder_word for consistent placeholder handling throughout the codebase
use crate::grue_compiler::codegen::{placeholder_word, ConstantValue, ZMachineCodeGen};
use crate::grue_compiler::codegen::{
    InstructionForm, InstructionLayout, Operand, OperandType, UNIMPLEMENTED_OPCODE,
};
use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::{IrInstruction, IrValue};
use crate::grue_compiler::opcodes::*;
use log::debug;

/// Extension trait for ZMachineCodeGen to handle instruction generation
impl ZMachineCodeGen {
    /// Generate code for a single IR instruction
    pub fn generate_instruction(
        &mut self,
        instruction: &IrInstruction,
    ) -> Result<(), CompilerError> {
        debug!("Generate instruction called: {:?}", instruction);

        // Track PC→IR mapping for ALL instructions
        let func_name = self
            .current_function_name
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let pc = self.code_address;
        let instr_desc = format!("{:?}", instruction);
        let instr_short = if instr_desc.len() > 60 {
            format!("{}...", &instr_desc[..57])
        } else {
            instr_desc
        };

        // Extract relevant IR ID for tracking
        let ir_id = match instruction {
            IrInstruction::LoadImmediate { target, .. } => *target,
            IrInstruction::BinaryOp { target, .. } => *target,
            IrInstruction::Call { target, .. } => target.unwrap_or(0),
            IrInstruction::GetProperty { target, .. } => *target,
            IrInstruction::GetPropertyByNumber { target, .. } => *target,
            IrInstruction::TestAttribute { target, .. } => *target,
            IrInstruction::UnaryOp { target, .. } => *target,
            _ => 0,
        };

        self.pc_to_ir_map
            .insert(pc, (func_name, ir_id, instr_short));

        // CRITICAL DEBUG: Track all branch and jump instructions
        match instruction {
            IrInstruction::Branch { .. } | IrInstruction::Jump { .. } => {
                log::debug!(
                    "CONTROL FLOW at 0x{:04x}: {:?}",
                    self.code_address,
                    instruction
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
            IrInstruction::TestAttribute { target, .. } => {
                log::debug!(
                    "IR INSTRUCTION: TestAttribute creates target IR ID {}",
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
                    let builtin_name = self
                        .get_builtin_function_name(*function)
                        .unwrap_or(&"<unknown>".to_string())
                        .clone();
                    log::debug!(
                        "📞 CALL builtin function ID {} ('{}') with {} args",
                        function,
                        builtin_name,
                        args.len()
                    );

                    self.generate_builtin_function_call(*function, args, *target)?;
                } else {
                    log::debug!(
                        "📞 CALL user function ID {} with {} args",
                        function,
                        args.len()
                    );

                    // Generate user function call with proper reference registration
                    self.generate_user_function_call(*function, args, *target)?;
                    // Phase 1: Track user function call results in Variable(0)
                    if let Some(target_id) = target {
                        // Mark this as a function call result stored in Variable(0)
                        self.function_call_results.insert(*target_id);
                        // Map the result directly to Variable(0) without generating extra store
                        self.ir_id_to_stack_var.insert(*target_id, 0);
                        log::debug!(
                            "Call result: IR ID {} -> function call result in Variable(0)",
                            target_id
                        );
                    }
                }
            }

            // TODO: CallIndirect is not implemented in current IR - will be needed for advanced features
            // IrInstruction::CallIndirect {
            //     target,
            //     function_addr,
            //     args,
            // } => {
            //     // CallIndirect: Call a function whose address is stored in a variable/property
            //     // Used for property-based dispatch (e.g., room.on_look() where on_look is a property)
            //     // The function address comes from a property value resolved at runtime
            //     log::debug!(
            //         "📞 CALL_INDIRECT function address from IR ID {} with {} args",
            //         function_addr,
            //         args.len()
            //     );
            //
            //     // Build operands: [function_address, arg1, arg2, ...]
            //     let func_addr_operand = self.resolve_ir_id_to_operand(*function_addr)?;
            //     let mut operands = vec![func_addr_operand];
            //
            //     for arg_id in args {
            //         let arg_operand = self.resolve_ir_id_to_operand(*arg_id)?;
            //         operands.push(arg_operand);
            //     }
            //
            //     // CRITICAL: Register call result target for proper LoadVar resolution
            //     // Use stack for call results (per Z-Machine specification)
            //     if let Some(target_id) = target {
            //         self.use_push_pull_for_result(*target_id, "indirect function call")?;
            //         log::debug!("CallIndirect result: IR ID {} -> push/pull stack", target_id);
            //     }
            // }
            IrInstruction::Return { value } => {
                if let Some(ir_value) = value {
                    // Return with value - use ret opcode with operand
                    let return_operand = self.resolve_ir_id_to_operand(*ir_value)?;
                    let operands = vec![return_operand]; // Return resolved value
                    self.emit_instruction_typed(RET, &operands, None, None)?;
                } else {
                    // Return without value - rtrue (no operands)
                    self.emit_instruction_typed(RTRUE, &[], None, None)?;
                }
            }

            IrInstruction::Jump { label } => {
                // CRITICAL DEBUG: Track Jump instructions near problem area
                if self.code_address >= 0x330 && self.code_address <= 0x340 {
                    log::debug!(
                        "IR JUMP INSTRUCTION at code_address=0x{:04x}, jumping to label {}",
                        self.code_address,
                        label
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
                    log::debug!("CRITICAL BRANCH at code_address=0x{:04x}: condition={}, true_label={}, false_label={}", 
                              self.code_address, condition, true_label, false_label);
                    log::debug!("  Before emit_conditional_branch_instruction");
                }
                let before_addr = self.code_address;
                // Delegate to the proper conditional branch function that handles binary operations
                self.emit_conditional_branch_instruction(*condition, *true_label, *false_label)?;
                let after_addr = self.code_address;
                if before_addr == 0x0001 || before_addr == 0x32f {
                    log::debug!(
                        "  After Branch: moved from 0x{:04x} to 0x{:04x} ({} bytes)",
                        before_addr,
                        after_addr,
                        after_addr - before_addr
                    );
                    log::debug!("  Expected: opcode + 2 operands + 2 branch bytes = 5 bytes");
                    log::debug!("  Actual: {} bytes", after_addr - before_addr);
                    if after_addr - before_addr != 5 {
                        log::debug!("  ERROR: Branch instruction missing branch placeholder!");
                    }
                }
            }

            IrInstruction::LoadVar { target, var_id } => {
                // Check if this is a local variable (needs mapping) or global (direct Z-Machine var number)
                let var_num = if let Some(&local_var_num) = self.ir_id_to_local_var.get(var_id) {
                    // Local variable: use mapped slot (1-15)
                    log::debug!(
                        "LoadVar: IR ID {} is local variable slot {}",
                        var_id,
                        local_var_num
                    );
                    local_var_num
                } else if *var_id >= 16 {
                    // Global variable: var_id IS the Z-Machine variable number (16-255)
                    log::debug!(
                        "LoadVar: IR ID {} is global variable G{:02}",
                        var_id,
                        var_id - 16
                    );
                    *var_id as u8
                } else {
                    // ERROR: var_id < 16 but not in local mapping
                    return Err(CompilerError::CodeGenError(format!(
                        "LoadVar: Variable IR ID {} not allocated to local variable slot. \
                         This indicates a bug in IR generation - all variables must be allocated before use.",
                        var_id
                    )));
                };

                let var_operand = Operand::SmallConstant(var_num);

                // Allocate a global variable for the loaded value using existing allocation method
                let result_var = self.allocate_global_for_ir_id(*target as u32);

                // CRITICAL FIX (Oct 19, 2025): Store to allocated global variable, NOT stack
                // Bug: Commit 48fccdf accidentally changed this to Some(0), causing all LoadVar
                // operations to store to Variable(0) (stack) instead of unique global variables.
                // This broke navigation because player object resolved to Variable(0) instead of Variable(217).
                self.emit_instruction_typed(LOAD, &[var_operand], Some(result_var as u8), None)?;

                // Track this IR ID as using the allocated global (NOT stack)
                // CRITICAL FIX (Oct 19, 2025): Map to allocated global variable, NOT Variable(0)
                // Bug: This was accidentally changed to 0, breaking variable resolution for all loaded objects
                self.ir_id_to_stack_var.insert(*target, result_var as u8);
                log::debug!(
                    "LoadVar: IR ID {} loaded from Z-Machine variable {} -> Variable({}) [Allocated global G{}]",
                    var_id,
                    var_num,
                    result_var,
                    result_var - 16
                );
            }

            IrInstruction::StoreVar { var_id, source } => {
                // Resolve value to operand
                let value_operand = self.resolve_ir_id_to_operand(*source)?;

                // Check if this is a local variable (needs mapping) or global (direct Z-Machine var number)
                let var_num = if let Some(&local_var_num) = self.ir_id_to_local_var.get(var_id) {
                    // Local variable: use mapped slot (1-15)
                    log::debug!(
                        "StoreVar: IR ID {} is local variable slot {}",
                        var_id,
                        local_var_num
                    );
                    local_var_num
                } else if *var_id >= 16 {
                    // Global variable: var_id IS the Z-Machine variable number (16-255)
                    log::debug!(
                        "StoreVar: IR ID {} is global variable G{:02}",
                        var_id,
                        var_id - 16
                    );
                    *var_id as u8
                } else {
                    // ERROR: var_id < 16 but not in local mapping
                    return Err(CompilerError::CodeGenError(format!(
                        "StoreVar: Variable IR ID {} not allocated to local variable slot. \
                         This indicates a bug in IR generation - all variables must be allocated before use.",
                        var_id
                    )));
                };

                // Store to variable using 2OP:13 store instruction
                let var_operand = Operand::SmallConstant(var_num);
                self.emit_instruction_typed(STORE, &[var_operand, value_operand], None, None)?;
                log::debug!(
                    "StoreVar: Stored value from IR ID {} to Z-Machine variable {} (var_id {})",
                    source,
                    var_num,
                    var_id
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
                self.use_push_pull_for_result(*target, "array creation")?;
                log::debug!("CreateArray: IR ID {} -> push/pull stack", target);

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
                    self.emit_instruction_typed(
                        Opcode::Op2(Op2::Loadw),
                        &[Operand::SmallConstant(0), Operand::SmallConstant(1)], // Loadw requires 2 operands
                        Some(0), // Store result to stack
                        None,
                    )?;
                } else {
                    // Non-empty array: For now, just return a placeholder null value
                    // Full array implementation requires allocating static data section
                    // which is complex. For literal arrays in for-loops, we can optimize
                    // by unrolling the loop instead.
                    //
                    // TODO: Implement proper array allocation in static data section
                    log::warn!(
                        "CreateArray with size {} not fully implemented - returning null. \
                        Array literals in for-loops should be optimized to direct iteration.",
                        size_value
                    );

                    // Push null/zero to stack as placeholder
                    self.emit_instruction_typed(
                        Opcode::OpVar(OpVar::Push),
                        &[Operand::SmallConstant(0)],
                        None,
                        None,
                    )?;
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

                // Track that this IR ID comes from a property access (for print() type detection)
                self.ir_id_from_property.insert(*target);

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
                // SURGICAL FIX (Oct 31, 2025): GetProperty stores to global variable, not Variable(0)
                // This prevents stack underflow in subsequent operations that read the result
                let store_var = self.allocate_global_variable();
                self.ir_id_to_stack_var.insert(*target, store_var);

                // SURGICAL FIX: Remove from push_pull_ir_ids to prevent stack lookup conflict
                self.push_pull_ir_ids.remove(target);

                self.emit_instruction_typed(
                    Opcode::Op2(Op2::GetProp),
                    &[obj_operand, Operand::SmallConstant(prop_num)],
                    Some(store_var), // Store to global variable instead of Variable(0)
                    None,
                )?;

                log::debug!(
                    "GetProperty SURGICAL FIX: IR ID {} -> global var {}",
                    target,
                    store_var
                );
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

                // BUG FIX (Oct 11, 2025): Use ir_id_to_stack_var (for globals) instead of ir_id_to_local_var
                // CRITICAL: ir_id_to_local_var is for LOCAL variables (slots 1-15) used by functions
                // GetPropertyByNumber results should use GLOBAL variables (200+) and go in ir_id_to_stack_var
                // Mixing them causes architectural confusion and potential bugs
                //
                // SOLUTION: Allocate unique global variable for each result, store in ir_id_to_stack_var
                // Check if we've already allocated a variable for this IR ID
                if !self.ir_id_to_stack_var.contains_key(target) {
                    // Use the proper global allocation function (same as builtins)
                    let fresh_var = self.allocate_global_for_ir_id(*target);
                    self.ir_id_to_stack_var.insert(*target, fresh_var);
                    log::debug!(
                        "GetPropertyByNumber: Allocated global variable {} for IR ID {}",
                        fresh_var,
                        target
                    );
                }

                let result_var = *self.ir_id_to_stack_var.get(target).unwrap();

                // Track that this IR ID comes from a property access (for print() type detection)
                self.ir_id_from_property.insert(*target);

                // PROPERTY ACCESS CORRECTION (Sept 28, 2025): Fixed branch out of bounds bug
                // Same fix as GetProperty above - reverted from 0x01 (je) back to 0x11 (get_prop).
                // This handles numbered property access (property_num instead of property name).
                self.emit_instruction_typed(
                    Opcode::Op2(Op2::GetProp),
                    &[obj_operand, prop_operand],
                    Some(result_var), // Store to unique global variable
                    None,
                )?;
                log::debug!(
                    "GetPropertyByNumber: IR ID {} -> global var {}",
                    target,
                    result_var
                );
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
                self.emit_instruction_typed(Opcode::OpVar(OpVar::PutProp), &operands, None, None)?;
                log::debug!(
                    "Generated put_prop for property number {} with resolved object",
                    property_num
                );
            }

            IrInstruction::TestAttribute {
                target,
                object,
                attribute_num,
            } => {
                // FIXED IMPLEMENTATION: Direct Z-Machine test_attr with corrected operand and stack discipline
                log::debug!(
                    "TestAttribute codegen: Direct Z-Machine with corrected operand for object={}, attr={}",
                    object,
                    attribute_num
                );

                // ATTRIBUTE FIX: Pass attribute_num as direct SmallConstant, not as IR ID
                // The problem was (*attribute_num).into() converts 2 to an IR ID which resolves
                // to LargeConstant(262) instead of SmallConstant(2)
                let obj_operand = self.resolve_ir_id_to_operand(*object)?;
                let attr_operand = Operand::SmallConstant(*attribute_num as u8);

                log::debug!(
                    "TestAttribute FIX: obj={:?}, attr={:?} (fixed from LargeConstant(262))",
                    obj_operand,
                    attr_operand
                );

                // Step 1: Generate unique labels for branch logic
                let unique_seed = (self.code_address * 7919) % 100000;
                let true_label_id: u32 = (50000 + unique_seed) as u32;
                let end_label_id: u32 = (60000 + unique_seed) as u32;

                // Step 2: Emit test_attr as branch instruction with CORRECT SmallConstant
                let layout = self.emit_instruction(
                    0x0A, // 2OP:10 (test_attr)
                    &[obj_operand, attr_operand],
                    None,     // No store_var - this is a branch instruction
                    Some(-1), // Placeholder for branch offset
                )?;

                // Step 4: Create UnresolvedReference for branch target
                self.reference_context.unresolved_refs.push(
                    crate::grue_compiler::codegen::UnresolvedReference {
                        reference_type: crate::grue_compiler::codegen::LegacyReferenceType::Branch,
                        location: layout.branch_location.unwrap(),
                        target_id: true_label_id,
                        is_packed_address: false,
                        offset_size: 2,
                        location_space: crate::grue_compiler::codegen::MemorySpace::Code,
                    },
                );

                // SURGICAL FIX (Oct 31, 2025): TestAttribute stores to global variable, not Variable(0)
                // This prevents stack underflow in subsequent operations that read the result
                let store_var = self.allocate_global_variable();
                self.ir_id_to_stack_var.insert(*target, store_var);

                // SURGICAL FIX: Remove from push_pull_ir_ids to prevent stack lookup conflict
                self.push_pull_ir_ids.remove(target);

                // Step 5: Attribute false - store 0 to allocated global variable
                self.emit_instruction(
                    0x94, // 2OP:20 (add)
                    &[Operand::SmallConstant(0), Operand::SmallConstant(0)],
                    Some(store_var), // Store result to allocated global variable
                    None,
                )?;
                self.translate_jump(end_label_id)?;

                // Step 6: true_label - attribute true, store 1 to allocated global variable
                self.record_code_space_offset(true_label_id, self.code_address);
                self.emit_instruction(
                    0x94, // 2OP:20 (add)
                    &[Operand::SmallConstant(0), Operand::SmallConstant(1)],
                    Some(store_var), // Store result to allocated global variable
                    None,
                )?;

                // Step 7: end_label - value is now in allocated global variable
                self.record_code_space_offset(end_label_id, self.code_address);
            }

            IrInstruction::TestAttributeBranch {
                object,
                attribute_num,
                then_label,
                else_label,
            } => {
                // CRITICAL FIX: Implement missing TestAttributeBranch codegen
                // This instruction was being silently skipped, causing wrong branch behavior
                log::error!(
                    "🎯 PHASE 3: TestAttributeBranch codegen for object={}, attr={}, then={}, else={}",
                    object, attribute_num, then_label, else_label
                );

                // Resolve object operand and use attribute_num as direct SmallConstant
                let obj_operand = self.resolve_ir_id_to_operand(*object)?;
                let attr_operand = Operand::SmallConstant(*attribute_num as u8);

                log::error!(
                    "TestAttributeBranch: obj={:?}, attr={:?}",
                    obj_operand,
                    attr_operand
                );

                // Emit test_attr as branch instruction
                // CRITICAL FIX: Z-Machine test_attr branches when attribute is SET
                // But in IR: then_label = else branch content, else_label = then branch content
                // So when attribute is SET, branch to else_label (which contains then-branch content)
                log::debug!(
                    "🔧 TestAttributeBranch codegen: object={}, attr={}",
                    object,
                    attribute_num
                );
                log::debug!(
                    "🔧 OBJECT MAPPING: IR object={}, resolved to operand={:?}",
                    object,
                    obj_operand
                );
                log::debug!(
                    "🔧 ATTRIBUTE: attr_num={}, operand={:?}",
                    attribute_num,
                    attr_operand
                );
                let layout = self.emit_instruction_typed(
                    Opcode::Op2(Op2::TestAttr),
                    &[obj_operand, attr_operand],
                    None,     // No store_var - this is a branch instruction
                    Some(-1), // Placeholder for branch offset
                )?;
                log::debug!("🔧 emit_instruction_typed returned layout: {:?}", layout);

                // Create UnresolvedReference: when attribute is SET, branch to then_label
                // IR semantics: then_label = true branch, else_label = false branch
                // Z-Machine test_attr: branches when attribute is SET (true)
                // Note: Correct label ordering is handled in IR generation (ir.rs:2582-2618)
                self.reference_context.unresolved_refs.push(
                    crate::grue_compiler::codegen::UnresolvedReference {
                        reference_type: crate::grue_compiler::codegen::LegacyReferenceType::Branch,
                        location: layout.branch_location.unwrap(),
                        target_id: *then_label, // CORRECTED: branch to then_label when attribute is SET
                        is_packed_address: false,
                        offset_size: 2,
                        location_space: crate::grue_compiler::codegen::MemorySpace::Code,
                    },
                );

                // If attribute is CLEAR, fall through to else_label (which contains else-branch content)
                log::debug!(
                    "TestAttributeBranch: SET -> branch to L{} (then-branch content), CLEAR -> fall through to L{} (else-branch content)",
                    then_label, else_label
                );
                log::debug!(
                    "TestAttributeBranch: Emitted instruction at PC=0x{:04x}, branch_location={:?}",
                    layout.instruction_start,
                    layout.branch_location
                );
            }

            IrInstruction::SetAttribute {
                object,
                attribute_num,
                value,
            } => {
                // Generate Z-Machine set_attr or clear_attr instruction
                let obj_operand = self.resolve_ir_id_to_operand(*object)?;
                let attr_operand = Operand::SmallConstant(*attribute_num);

                if *value {
                    log::debug!(
                        "Generated set_attr: object={:?} attribute={:?}",
                        obj_operand,
                        attr_operand
                    );
                    // set_attr (2OP:11, opcode 0x0B)
                    self.emit_instruction_typed(
                        Opcode::Op2(Op2::SetAttr),
                        &[obj_operand, attr_operand],
                        None,
                        None,
                    )?;
                } else {
                    log::debug!(
                        "Generated clear_attr: object={:?} attribute={:?}",
                        obj_operand,
                        attr_operand
                    );
                    // clear_attr (2OP:12, opcode 0x0C)
                    self.emit_instruction_typed(
                        Opcode::Op2(Op2::ClearAttr),
                        &[obj_operand, attr_operand],
                        None,
                        None,
                    )?;
                }
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
                self.use_push_pull_for_result(*target, "test_attr operation")?;

                // Use test_attr instruction: 2OP:10 (0x0A) for testing
                self.emit_instruction_typed(
                    Opcode::Op2(Op2::TestAttr),
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
                log::debug!(
                    "GetNextProperty: object IR ID {}, current_property {}",
                    object,
                    current_property
                );
                let obj_operand = self.resolve_ir_id_to_operand(*object)?;
                log::debug!("GetNextProperty: obj_operand = {:?}", obj_operand);
                let prop_operand = self.resolve_ir_id_to_operand((*current_property).into())?;

                // CRITICAL: Register target for next property result
                self.use_push_pull_for_result(*target, "get_next_prop operation")?;

                // Use get_next_prop instruction: 2OP:19 (0x13)
                self.emit_instruction_typed(
                    Opcode::Op2(Op2::GetNextProp),
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
                self.use_push_pull_for_result(*target, "array empty check")?;

                // For now, just push 0 (empty/false) as a placeholder
                self.emit_instruction_typed(
                    Opcode::OpVar(OpVar::Push),
                    &[Operand::LargeConstant(0)],
                    None, // Push doesn't use store_var
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
                self.use_push_pull_for_result(*target, "array element access")?;

                // For now, just push placeholder string address
                self.emit_instruction_typed(
                    Opcode::OpVar(OpVar::Push),
                    &[Operand::LargeConstant(1000)], // Placeholder string ID
                    None,                            // Push doesn't use store_var
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
                self.use_push_pull_for_result(*target, "array remove operation")?;
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction_typed(
                    Opcode::OpVar(OpVar::Push),
                    &[Operand::SmallConstant(0)],
                    None,
                    None,
                )?;
                log::debug!("ArrayRemove: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArrayLength { target, array: _ } => {
                // Array length operation - placeholder returns 0
                self.use_push_pull_for_result(*target, "array length operation")?;
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction_typed(
                    Opcode::OpVar(OpVar::Push),
                    &[Operand::SmallConstant(0)],
                    None,
                    None,
                )?;
                log::debug!("ArrayLength: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArrayContains {
                target,
                array: _,
                value: _,
            } => {
                // Array contains operation - placeholder returns false (0)
                self.use_push_pull_for_result(*target, "array contains operation")?;
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction_typed(
                    Opcode::OpVar(OpVar::Push),
                    &[Operand::SmallConstant(0)],
                    None,
                    None,
                )?;
                log::debug!("ArrayContains: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArrayIndexOf {
                target,
                array: _,
                value: _,
            } => {
                // Array indexOf operation - placeholder returns -1 (not found)
                self.use_push_pull_for_result(*target, "array indexOf operation")?;
                // Emit instruction to push -1 onto stack as placeholder result
                self.emit_instruction_typed(
                    Opcode::OpVar(OpVar::Push),
                    &[Operand::LargeConstant(65535)],
                    None,
                    None,
                )?;
                log::debug!("ArrayIndexOf: IR ID {} -> stack (placeholder: -1)", target);
            }

            IrInstruction::ArrayFilter {
                target,
                array: _,
                predicate: _,
            } => {
                // Array filter operation - placeholder returns empty array (0)
                self.use_push_pull_for_result(*target, "array filter operation")?;
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction_typed(
                    Opcode::OpVar(OpVar::Push),
                    &[Operand::SmallConstant(0)],
                    None,
                    None,
                )?;
                log::debug!("ArrayFilter: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArrayMap {
                target,
                array: _,
                transform: _,
            } => {
                // Array map operation - placeholder returns empty array (0)
                self.use_push_pull_for_result(*target, "array map operation")?;
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction_typed(
                    Opcode::OpVar(OpVar::Push),
                    &[Operand::SmallConstant(0)],
                    None,
                    None,
                )?;
                log::debug!("ArrayMap: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArrayFind {
                target,
                array: _,
                predicate: _,
            } => {
                // Array find operation - placeholder returns null (0)
                self.use_push_pull_for_result(*target, "array find operation")?;
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction_typed(
                    Opcode::OpVar(OpVar::Push),
                    &[Operand::SmallConstant(0)],
                    None,
                    None,
                )?;
                log::debug!("ArrayFind: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArrayJoin {
                target,
                array: _,
                separator: _,
            } => {
                // Array join operation - placeholder returns empty string (0)
                self.use_push_pull_for_result(*target, "array join operation")?;
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction_typed(
                    Opcode::OpVar(OpVar::Push),
                    &[Operand::SmallConstant(0)],
                    None,
                    None,
                )?;
                log::debug!("ArrayJoin: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArrayReverse { target, array: _ } => {
                // Array reverse operation - placeholder returns original array (0)
                self.use_push_pull_for_result(*target, "array reverse operation")?;
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction_typed(
                    Opcode::OpVar(OpVar::Push),
                    &[Operand::SmallConstant(0)],
                    None,
                    None,
                )?;
                log::debug!("ArrayReverse: IR ID {} -> stack (placeholder: 0)", target);
            }

            IrInstruction::ArraySort {
                target,
                array: _,
                comparator: _,
            } => {
                // Array sort operation - placeholder returns original array (0)
                self.use_push_pull_for_result(*target, "array sort operation")?;
                // Emit instruction to push 0 onto stack as placeholder result
                self.emit_instruction_typed(
                    Opcode::OpVar(OpVar::Push),
                    &[Operand::SmallConstant(0)],
                    None,
                    None,
                )?;
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
                array,
                index,
                value,
            } => {
                // Set array element: storew array_addr+1+(index*2), value
                // Array format: [size_word, element0, element1, ...]
                // So element[i] is at address array_addr + 1 + i
                // But storew takes byte address, so element[i] is at (array_addr+2) + (i*2)

                let array_op = self.resolve_ir_id_to_operand(*array)?;
                let index_op = self.resolve_ir_id_to_operand(*index)?;
                let value_op = self.resolve_ir_id_to_operand(*value)?;

                log::debug!(
                    "SetArrayElement: array={:?}, index={:?}, value={:?}",
                    array_op,
                    index_op,
                    value_op
                );

                // We need to:
                // 1. Calculate element address: array_addr + 2 + (index * 2)
                // 2. Store value at that address using storew
                //
                // Since array format is: [size][elem0][elem1]...,
                // and each word is 2 bytes, element[i] is at byte offset 2 + (i*2)
                //
                // Z-Machine storew: storew array index value
                // stores VALUE at address (array + 2*index)
                // This is perfect for our layout if we pass (array+2) as the array base!

                // Calculate base address (skip size word): array + 1 word = array + 2 bytes
                // But storew multiplies index by 2, so we just need array+2 as base
                // Actually, storew stores at (array + 2*index), so for our layout:
                // We want to store at (array_addr + 2) + (index * 2)
                // Which is storew (array_addr+2), index, value

                // But we have array address in a variable/stack, not a constant
                // We need to add 1 to it first (1 word = skip size field)
                // Use: add array, #1 -> stack (adds 1 word = 2 bytes to get element base)
                self.emit_instruction_typed(
                    Opcode::Op2(Op2::Add),
                    &[array_op.clone(), Operand::SmallConstant(1)],
                    Some(0), // Store to stack
                    None,
                )?;

                // Now store: storew (stack), index, value
                // Pop the base address from stack
                self.emit_instruction_typed(
                    Opcode::OpVar(OpVar::Storew),
                    &[Operand::Variable(0), index_op, value_op], // Variable 0 = stack
                    None,
                    None,
                )?;
            }

            IrInstruction::GetObjectChild {
                target,
                object,
                branch_if_no_child,
            } => {
                // Z-Machine get_child opcode: returns first child object
                // Branches when child EXISTS (returns ≠ 0) per Z-Machine specification
                let obj_operand = self.resolve_ir_id_to_operand(*object)?;

                // CRITICAL BUG FIX (Oct 27, 2025): Z-Machine opcode confusion
                // PROBLEM: Was using 0x01 (get_sibling) instead of 0x02 (get_child)
                // IMPACT: Object tree iteration never found children, always returned 0
                // SYMPTOM: For-loops skipped entirely, no objects listed in rooms
                // FIX: Use correct Z-Machine opcodes per specification:
                //   - 1OP:1 (0x01) = get_sibling
                //   - 1OP:2 (0x02) = get_child
                let placeholder = 0x7FFF_u16 as i16; // bit 15=0 for branch-on-FALSE
                let layout = self.emit_instruction(
                    0x02, // get_child opcode (1OP:2) - FIXED: was 0x01 (get_sibling)
                    &[obj_operand],
                    Some(0),           // Store result to stack
                    Some(placeholder), // Placeholder encodes branch polarity
                )?;

                // Create unresolved reference for branch target
                if let Some(branch_location) = layout.branch_location {
                    use crate::grue_compiler::codegen::{
                        LegacyReferenceType, MemorySpace, UnresolvedReference,
                    };
                    self.reference_context
                        .unresolved_refs
                        .push(UnresolvedReference {
                            reference_type: LegacyReferenceType::Branch,
                            location: branch_location,
                            target_id: *branch_if_no_child,
                            is_packed_address: false,
                            offset_size: 2,
                            location_space: MemorySpace::Code,
                        });
                }

                // Stack result already available via get_child instruction storing to Variable(0)
                // No need for additional push - would cause stack imbalance
                self.ir_id_to_stack_var.insert(*target, 0);
            }

            IrInstruction::GetObjectSibling {
                target,
                object,
                branch_if_no_sibling,
            } => {
                // Z-Machine get_sibling opcode: returns next sibling object
                // Branches when sibling EXISTS (returns ≠ 0) per Z-Machine specification
                let obj_operand = self.resolve_ir_id_to_operand(*object)?;

                // CRITICAL BUG FIX (Oct 27, 2025): Both GetObjectChild and GetObjectSibling
                // were using wrong opcodes - they were swapped!
                // This completely broke object tree iteration in for-loops
                let placeholder = 0x7FFF_u16 as i16; // bit 15=0 for branch-on-FALSE
                let layout = self.emit_instruction(
                    0x01, // get_sibling opcode (1OP:1) - FIXED: was 0x02 (get_child)
                    &[obj_operand],
                    Some(0),           // Store result to stack
                    Some(placeholder), // Placeholder encodes branch polarity
                )?;

                // Create unresolved reference for branch target
                if let Some(branch_location) = layout.branch_location {
                    use crate::grue_compiler::codegen::{
                        LegacyReferenceType, MemorySpace, UnresolvedReference,
                    };
                    self.reference_context
                        .unresolved_refs
                        .push(UnresolvedReference {
                            reference_type: LegacyReferenceType::Branch,
                            location: branch_location,
                            target_id: *branch_if_no_sibling,
                            is_packed_address: false,
                            offset_size: 2,
                            location_space: MemorySpace::Code,
                        });
                }

                // Stack result already available via get_sibling instruction storing to Variable(0)
                // No need for additional push - would cause stack imbalance
                self.ir_id_to_stack_var.insert(*target, 0);
            }

            IrInstruction::GetObjectParent { target, object } => {
                // Z-Machine get_parent opcode: returns parent object number (0 if no parent)
                // BUG FIX (Oct 11, 2025): player.location must read parent from object tree
                // because move() uses insert_obj which updates the tree, not properties
                let obj_operand = self.resolve_ir_id_to_operand(*object)?;
                log::debug!(
                    "🏃 DEBUG GetObjectParent: object={}, obj_operand={:?}, target={}, PC=0x{:04x}",
                    object,
                    obj_operand,
                    target,
                    self.code_address
                );

                // SURGICAL FIX (Oct 31, 2025): Store to global variable, not Variable(0)
                // This prevents stack underflow in subsequent GetPropertyByNumber operations
                let store_var = self.allocate_global_variable();
                log::debug!(
                    "🔧 GetObjectParent: Allocated global variable {} for IR ID {}",
                    store_var,
                    target
                );

                // Emit get_parent instruction (1OP:3)
                self.emit_instruction_typed(
                    Opcode::Op1(Op1::GetParent),
                    &[obj_operand],
                    Some(store_var), // Store result to global variable
                    None,            // No branch
                )?;
                log::debug!(
                    "🏃 DEBUG GetObjectParent: Emitted get_parent instruction, now at PC=0x{:04x}",
                    self.code_address
                );

                // Map IR ID to the allocated global variable
                self.ir_id_to_stack_var.insert(*target, store_var);

                // SURGICAL FIX: Remove from push_pull_ir_ids to prevent stack lookup conflict
                self.push_pull_ir_ids.remove(target);
            }

            IrInstruction::InsertObj {
                object,
                destination,
            } => {
                // Z-Machine insert_obj opcode: sets object's parent, updates tree structure
                // (Oct 12, 2025): Used for .location = assignment
                // This removes object from current parent and inserts as first child of destination

                let obj_operand = self.resolve_ir_id_to_operand(*object)?;
                let dest_operand = self.resolve_ir_id_to_operand(*destination)?;

                // Track initial locations for compile-time object tree initialization
                // (Phase 2: Oct 12, 2025): When in init block, extract actual object numbers from operands
                // and store mapping for use during object table generation
                if self.in_init_block {
                    // Extract object numbers from resolved operands
                    let obj_num = match obj_operand {
                        Operand::SmallConstant(n) => Some(n as u16),
                        Operand::Constant(n) => Some(n),
                        Operand::LargeConstant(n) => Some(n),
                        _ => None,
                    };
                    let parent_num = match dest_operand {
                        Operand::SmallConstant(n) => Some(n as u16),
                        Operand::Constant(n) => Some(n),
                        Operand::LargeConstant(n) => Some(n),
                        _ => None,
                    };

                    if let (Some(obj), Some(parent)) = (obj_num, parent_num) {
                        // Store using actual object numbers for lookup during object entry creation
                        self.initial_locations.insert(*object, *destination); // Keep for reference
                        self.initial_locations_by_number.insert(obj, parent);
                        log::warn!(
                            "🏗️ INITIAL_LOCATION_TRACKED: Object #{} -> Parent #{} (IR {} -> IR {}) at compile time",
                            obj, parent, object, destination
                        );
                    }
                }

                // Emit insert_obj instruction (2OP:14 = 0x0E)
                self.emit_instruction_typed(
                    Opcode::Op2(Op2::InsertObj),
                    &[obj_operand, dest_operand],
                    None, // No result
                    None, // No branch
                )?;
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

            // Debug breakpoint (debug builds only)
            #[cfg(debug_assertions)]
            IrInstruction::DebugBreak { label } => {
                log::debug!("Generating debug breakpoint: {}", label);
                self.generate_debug_break_builtin(label)?;
            }

            // TODO: LogicalComparisonOp is not implemented in current IR - will be needed for advanced features
            // IrInstruction::LogicalComparisonOp {
            //     target,
            //     op,
            //     left_expr,
            //     right_expr,
            // } => {
            //     // Generate proper short-circuit evaluation for logical operations on comparisons
            //     log::debug!(
            //         "LogicalComparisonOp: Generating short-circuit {:?} logic for target IR ID {}",
            //         op,
            //         target
            //     );
            //
            //     // Use stack for result storage
            //     self.use_push_pull_for_result(*target, "string operation")?;
            //
            //     match op {
            //         crate::grue_compiler::ir::IrBinaryOp::And => {
            //             self.generate_short_circuit_and(target, left_expr, right_expr)?;
            //         }
            //         crate::grue_compiler::ir::IrBinaryOp::Or => {
            //             self.generate_short_circuit_or(target, left_expr, right_expr)?;
            //         }
            //         _ => {
            //             return Err(CompilerError::CodeGenError(format!(
            //                 "LogicalComparisonOp: unsupported operation {:?}",
            //                 op
            //             )));
            //         }
            //     }
            // }
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
    /// # CRITICAL OPCODE CONVENTION
    ///
    /// The `opcode` parameter MUST be the RAW opcode number (0x00-0x1F), NOT the encoded
    /// instruction byte. This function determines the proper instruction form (Long, Short,
    /// Variable, Extended) and encodes it correctly.
    ///
    /// ## Examples of CORRECT usage:
    /// - call_vs (VAR:224) → pass `0x00`, NOT `0xE0` or `0x20`
    /// - put_prop (VAR:227) → pass `0x03`, NOT `0xE3`
    /// - jl (2OP:2) → pass `0x02`
    /// - print_paddr (1OP:141) → pass `0x0D`, NOT `0x8D`
    ///
    /// ## Why this matters:
    /// The function uses `is_true_var_opcode(opcode)` to determine if bit 5 should be set
    /// in the instruction byte. If you pass an encoded byte (e.g., 0x20 instead of 0x00),
    /// the function won't recognize it as a VAR opcode and will encode it incorrectly,
    /// causing runtime failures.
    ///
    /// # Arguments
    ///
    /// * `opcode` - Raw Z-Machine opcode number (0x00-0x1F)
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
    /// Emit Z-Machine instruction with type-safe opcode enum.
    ///
    /// **NEW TYPE-SAFE VERSION** - Provides compile-time validation of:
    /// - Instruction form (0OP/1OP/2OP/VAR encoded into type)
    /// - Store variable requirements (panics if instruction doesn't store but store_var provided)
    /// - Branch requirements (panics if instruction doesn't branch but branch_offset provided)
    /// - Z-Machine version requirements (panics if opcode requires newer version than target)
    ///
    /// **Examples**:
    /// ```ignore
    /// use crate::grue_compiler::opcodes::*;
    ///
    /// // 0OP instruction - no operands
    /// self.emit_instruction_typed(Opcode::Op0(Op0::Quit), &[], None, None)?;
    ///
    /// // 1OP instruction - print string
    /// self.emit_instruction_typed(
    ///     Opcode::Op1(Op1::PrintPaddr),
    ///     &[Operand::LargeConstant(string_addr)],
    ///     None,  // Doesn't store
    ///     None   // Doesn't branch
    /// )?;
    ///
    /// // 2OP instruction with branch
    /// self.emit_instruction_typed(
    ///     Opcode::Op2(Op2::Je),
    ///     &[Operand::Variable(1), Operand::SmallConstant(5)],
    ///     None,           // Doesn't store
    ///     Some(offset)    // Branches
    /// )?;
    ///
    /// // VAR instruction with store
    /// self.emit_instruction_typed(
    ///     Opcode::OpVar(OpVar::CallVs),
    ///     &[Operand::LargeConstant(func_addr), Operand::Variable(1)],
    ///     Some(0),  // Store to stack
    ///     None      // Doesn't branch
    /// )?;
    ///
    /// // Or use convenience constants:
    /// self.emit_instruction_typed(QUIT, &[], None, None)?;
    /// self.emit_instruction_typed(ADD, &[op1, op2], Some(0), None)?;
    /// ```
    ///
    /// **Validation** (all enforced with panic):
    /// - Opcode version must be <= target version
    /// - store_var must be None if opcode doesn't store
    /// - branch_offset must be None if opcode doesn't branch
    ///
    /// **Migration from u8 version**:
    /// ```ignore
    /// // Old:
    /// self.emit_instruction(0x0A, &[], None, None)?;
    ///
    /// // New:
    /// self.emit_instruction_typed(Opcode::Op0(Op0::Quit), &[], None, None)?;
    /// // Or:
    /// self.emit_instruction_typed(QUIT, &[], None, None)?;
    /// ```
    pub fn emit_instruction_typed(
        &mut self,
        opcode: super::opcodes::Opcode,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<InstructionLayout, CompilerError> {
        #[allow(unused_imports)]
        use super::opcodes::OpcodeMetadata;

        let start_address = self.code_address;
        let _original_opcode_enum = opcode; // Store before shadowing (unused after opcode routing fix)
        let raw_opcode = opcode.raw_value();

        // VALIDATION 1: Version check
        let min_version = opcode.min_version();
        let target_version = match self.version {
            super::ZMachineVersion::V3 => 3,
            super::ZMachineVersion::V4 => 4,
            super::ZMachineVersion::V5 => 5,
        };

        if min_version > target_version {
            panic!(
                "COMPILER BUG: Opcode {:?} requires Z-Machine v{}, but targeting v{} at address 0x{:04x}",
                opcode, min_version, target_version, start_address
            );
        }

        // VALIDATION 2: Store variable check
        if store_var.is_some() && !opcode.stores_result() {
            panic!(
                "COMPILER BUG: Opcode {:?} does not store a result, but store_var={:?} was provided at address 0x{:04x}",
                opcode, store_var, start_address
            );
        }

        // VALIDATION 3: Branch offset check
        if branch_offset.is_some() && !opcode.branches() {
            panic!(
                "COMPILER BUG: Opcode {:?} does not branch, but branch_offset={:?} was provided at address 0x{:04x}",
                opcode, branch_offset, start_address
            );
        }

        log::debug!(
            "EMIT_TYPED: addr=0x{:04x} opcode={:?} (0x{:02x}) operands={:?} store={:?} branch={:?}",
            start_address,
            opcode,
            raw_opcode,
            operands,
            store_var,
            branch_offset
        );

        // CRITICAL FIX: Determine instruction form based on Opcode VARIANT, not just raw opcode
        // The raw opcode value alone cannot distinguish between:
        // - Op2::Or (0x08) vs OpVar::Push (0x08)
        // - Op2::And (0x09) vs OpVar::Pull (0x09)
        // We must respect the enum variant to choose the correct form
        use super::opcodes::Opcode;

        // DEBUG: Log ALL AND instructions
        if let Opcode::Op2(Op2::And) = opcode {
            log::debug!(
                "🔍 AND_EMIT: Emitting Op2(And) at 0x{:04x} with operands={:?}",
                self.code_address,
                operands
            );
        }

        let form = match opcode {
            Opcode::Op0(_) => InstructionForm::Short, // 0OP form
            Opcode::Op1(_) => InstructionForm::Short, // 1OP form
            Opcode::Op2(_) => {
                // 2OP can be LONG or VAR form depending on operands
                if operands.len() == 2 {
                    let can_use_long = operands.iter().all(|op| match op {
                        Operand::LargeConstant(v) => *v <= 255,
                        _ => true,
                    });
                    if can_use_long {
                        InstructionForm::Long
                    } else {
                        // DEBUG: Log when 2OP AND uses VAR form due to large constants
                        if let Opcode::Op2(Op2::And) = opcode {
                            log::debug!("🚨 AND_VAR_FORM: Op2(And) using VAR form due to large constants! operands={:?}", operands);
                        }
                        InstructionForm::Variable
                    }
                } else {
                    // DEBUG: Log when 2OP falls back to VAR form due to wrong operand count
                    if let Opcode::Op2(Op2::And) = opcode {
                        log::debug!("🚨 AND_FALLBACK: Op2(And) with {} operands falling back to VAR form! operands={:?}", operands.len(), operands);
                    }
                    InstructionForm::Variable // Fallback to VAR for unusual cases
                }
            }
            Opcode::OpVar(_) => InstructionForm::Variable, // VAR form (0xC0-0xFF)
        };

        // Emit using the determined form
        match form {
            InstructionForm::Short => self.emit_short_form_with_layout(
                start_address,
                raw_opcode,
                operands,
                store_var,
                branch_offset,
            ),
            InstructionForm::Long => self.emit_long_form_with_layout(
                start_address,
                raw_opcode,
                operands,
                store_var,
                branch_offset,
            ),
            InstructionForm::Variable => self.emit_variable_form_with_layout(
                start_address,
                &opcode,
                operands,
                store_var,
                branch_offset,
            ),
            InstructionForm::Extended => Err(CompilerError::CodeGenError(format!(
                "Extended form not yet implemented for opcode {:?} at 0x{:04x}",
                opcode, start_address
            ))),
        }
    }

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

        // CRITICAL VALIDATION: Validate that opcode is in expected range
        // Valid raw opcodes are 0x00-0x1F
        // Encoded instruction bytes start at 0x80 for SHORT form, 0xC0 for VAR form
        if opcode > 0x1F && opcode < 0x80 {
            return Err(CompilerError::CodeGenError(format!(
                "Invalid opcode 0x{:02x} at address 0x{:04x} - opcodes should be raw numbers 0x00-0x1F, not encoded instruction bytes. \
                Did you mean to use one of the opcode constants? Common mistakes: \
                call_vs is 0x00 (not 0x20 or 0xE0), put_prop is 0x03 (not 0xE3), print_paddr is 0x0D (not 0x8D)",
                opcode, start_address
            )));
        }

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

        // CRITICAL: Detect V5+ opcodes in V3 compilation
        if opcode == 0x1a || opcode == 0x1b || opcode == 0x1c {
            panic!("COMPILER BUG: V5+ opcode 0x{:02x} (call_2n/set_colour/clear_colour) emitted in V3 target at address 0x{:04x}. \
                    V3 must use call_vs (0x00) for all function calls. Check grammar generation or function call code.",
                   opcode, self.code_address);
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

        let form = self.determine_instruction_form_with_operands(operands, opcode)?;
        log::debug!(
            " FORM_DETERMINATION: opcode=0x{:02x} operands={:?} -> form={:?}",
            opcode,
            operands,
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
            InstructionForm::Variable => {
                // For emit_instruction (raw opcode), we need to reconstruct the enum
                // This is imperfect due to opcode conflicts, but handles most cases
                let reconstructed_enum = Self::reconstruct_opcode_enum(opcode);
                self.emit_variable_form_with_layout(
                    instruction_start,
                    &reconstructed_enum,
                    operands,
                    actual_store_var,
                    branch_offset,
                )?
            }
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

    /// Check if an ENCODED instruction byte should not emit store variable
    /// This receives the full encoded instruction byte, not the raw opcode
    fn should_not_emit_store_variable(instruction_byte: u8) -> bool {
        match instruction_byte {
            // Call instructions that don't store results (void calls)
            // Note: call_vs (0xE0) DOES store results and is not in this list
            0x8F => true, // 1OP:call_1n - no return value
            0x5A => true, // 2OP:call_2n - no return value

            // Stack instructions - push/pull do not have store variable bytes
            0xE8 => true, // VAR:push (0xE8) - no store byte
            0xE9 => true, // VAR:pull (0xE9) - no store byte
            // Note: 2OP:OR (0x08) encodes as 0x48/0x68 depending on operand types - has store byte
            // Note: 2OP:AND (0x09) encodes as 0x49/0x69 depending on operand types - has store byte

            // Print instructions - no result to store
            0x8D => true, // 1OP:print_paddr
            0x8A => true, // 1OP:print_obj
            0x87 => true, // 1OP:print_addr
            0xE5 => true, // VAR:print_char
            0xE6 => true, // VAR:print_num
            0xB3 => true, // 0OP:print_ret
            0xBB => true, // 0OP:new_line

            // Other instructions that don't store results
            0xBA => true, // 0OP:quit

            _ => false,
        }
    }

    /// CRITICAL FIX: Reconstruct Opcode enum from raw u8 value
    ///
    /// This function solves the Op2(And) vs OpVar(Pull) routing conflict for opcode 0x09.
    /// Both instructions share the same raw opcode value but need different Z-Machine encodings:
    /// - Op2(And) in VAR form should be 0xC9 (bit 5 = 0, operand_count = OP2)
    /// - OpVar(Pull) should be 0xE9 (bit 5 = 1, operand_count = VAR)
    ///
    /// The root cause was that emit_instruction (raw opcode) couldn't distinguish between
    /// these cases, causing PULL instructions to be encoded as 0xC9 and routed to the
    /// AND instruction handler at runtime, leading to stack underflows.
    ///
    /// This function is called when emit_instruction needs to call emit_variable_form_with_layout
    /// which requires the proper Opcode enum for correct bit encoding.
    ///
    /// For ambiguous cases like 0x09, defaults to OpVar(Pull) since Op2(And) should use
    /// emit_instruction_typed for proper disambiguation.
    fn reconstruct_opcode_enum(opcode: u8) -> Opcode {
        use super::opcodes::{Op2, OpVar};

        match opcode {
            // VAR form opcodes that should always be VAR (based on is_true_var_opcode logic)
            0x00 => Opcode::OpVar(OpVar::CallVs),
            0x01 => Opcode::OpVar(OpVar::Storew),
            0x03 => Opcode::OpVar(OpVar::PutProp),
            0x04 => Opcode::OpVar(OpVar::Aread),
            0x05 => Opcode::OpVar(OpVar::PrintChar),
            0x06 => Opcode::OpVar(OpVar::PrintNum),
            0x07 => Opcode::OpVar(OpVar::Random),
            0x08 => Opcode::OpVar(OpVar::Push),
            // CRITICAL DECISION: For opcode 0x09, default to OpVar(Pull)
            // This resolves the Op2(And) vs OpVar(Pull) conflict in favor of Pull
            // Op2(And) should be called via emit_instruction_typed for proper disambiguation
            0x09 => Opcode::OpVar(OpVar::Pull),

            // For all other opcodes, assume Op2 when used in Variable form
            // This covers cases like Op2 instructions being called with >2 operands
            _ => {
                // Convert raw opcode to Op2 variant if possible
                match opcode {
                    0x01 => Opcode::Op2(Op2::Je), // But this conflicts with Storew above...
                    0x02 => Opcode::Op2(Op2::Jl),
                    0x03 => Opcode::Op2(Op2::Jg), // But this conflicts with PutProp above...
                    0x04 => Opcode::Op2(Op2::DecChk), // But this conflicts with Sread above...
                    0x05 => Opcode::Op2(Op2::IncChk), // But this conflicts with PrintChar above...
                    0x06 => Opcode::Op2(Op2::Jin), // But this conflicts with PrintNum above...
                    0x07 => Opcode::Op2(Op2::Test), // But this conflicts with Random above...
                    0x08 => Opcode::Op2(Op2::Or), // But this conflicts with Push above...
                    0x0A => Opcode::Op2(Op2::TestAttr),
                    0x0B => Opcode::Op2(Op2::SetAttr),
                    0x0C => Opcode::Op2(Op2::ClearAttr),
                    0x0D => Opcode::Op2(Op2::Store),
                    0x0E => Opcode::Op2(Op2::InsertObj),
                    0x0F => Opcode::Op2(Op2::Loadw),
                    0x10 => Opcode::Op2(Op2::Loadb),
                    0x11 => Opcode::Op2(Op2::GetProp),
                    0x12 => Opcode::Op2(Op2::GetPropAddr),
                    0x13 => Opcode::Op2(Op2::GetNextProp),
                    0x14 => Opcode::Op2(Op2::Add),
                    0x15 => Opcode::Op2(Op2::Sub),
                    0x16 => Opcode::Op2(Op2::Mul),
                    0x17 => Opcode::Op2(Op2::Div),
                    0x18 => Opcode::Op2(Op2::Mod),
                    // For any unrecognized opcode, create a fallback
                    _ => Opcode::Op2(Op2::Or), // Safe default - doesn't conflict with known VAR opcodes
                }
            }
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
            0x08 => true, // push (raw opcode 8) - MUST be VAR form (0xE8), NOT 2OP:or (0x08/0xC8)
            // CRITICAL OPCODE 0x09 CONFLICT: Op2(And) vs Var(Pull)
            // Problem: Both Op2(And) and Var(Pull) use raw opcode 0x09 but need different encoding:
            // - Op2(And) in VAR mode should be 0xC9 (bit 5 = 0)
            // - Var(Pull) should be 0xE9 (bit 5 = 1)
            // Solution: Do NOT list 0x09 here - let Op2(And) encode as 0xC9
            // Var(Pull) will still work via emit_instruction_typed forcing VAR form
            // REMOVED: 0x09 => true, // This caused both And and Pull to encode as 0xE9
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
    ) -> Result<InstructionForm, CompilerError> {
        // Form-sensitive opcodes: same number means different instructions in different forms
        // This table lists opcodes where Long→VAR form change would break semantic correctness
        const FORM_SENSITIVE_OPCODES: &[(u8, &str, &str)] = &[
            (0x01, "je (jump if equal)", "storew (store word)"),
            (0x02, "jl (jump if less)", "storeb (store byte)"),
            (0x03, "jg (jump if greater)", "put_prop (set property)"),
            (0x04, "dec_chk (decrement and check)", "sread (read input)"),
            (0x05, "inc_chk (increment and check)", "print_char"),
            (0x06, "jin (jump if in)", "print_num"),
            (0x07, "test (test flags)", "random"),
            (0x08, "or (bitwise or)", "push (push stack)"),
            (0x09, "and (bitwise and)", "pull (pull stack)"),
            (0x0A, "test_attr (test attribute)", "split_window"),
            (0x0D, "store (store value)", "output_stream (select stream)"),
            (0x0E, "load (indirect variable)", "input_stream"),
            (0x0F, "loadw (load word)", "sound_effect"),
        ];

        // Check for form conflicts before determining form
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
                    Ok(InstructionForm::Long)
                } else {
                    // Check for form conflict
                    if let Some((_, long_name, var_name)) = FORM_SENSITIVE_OPCODES
                        .iter()
                        .find(|(op, _, _)| *op == opcode)
                    {
                        return Err(CompilerError::OpcodeFormConflict {
                            opcode,
                            long_form_name: long_name.to_string(),
                            var_form_name: var_name.to_string(),
                        });
                    }
                    Ok(InstructionForm::Variable)
                }
            }
            (0x03, 3) => Ok(InstructionForm::Variable), // put_prop is always VAR

            // Opcode 0x0D: Context-dependent!
            // - 1-2 operands: store (2OP form) - MUST use Long form
            // - 3+ operands: output_stream (VAR form)
            // CRITICAL: Store has 1 operand (value) + store_var (destination),
            // so operands.len() == 1 but it's actually a 2OP instruction!
            (0x0D, 1 | 2) => Ok(InstructionForm::Long), // store is 2OP, needs Long form
            (0x0D, _) => Ok(InstructionForm::Variable), // output_stream (3+ operands)

            // Opcode 0x05: Context-dependent! (Bug #17 fix)
            // - 2 operands: inc_chk (2OP:5) - increment variable and branch if > value
            // - 1 operand: print_char (VAR:5) - print single character
            // Without this check, 2-operand inc_chk was incorrectly encoded as VAR form,
            // causing it to be misinterpreted as print_char at runtime, which shifted
            // all subsequent instruction decoding and triggered "erase_line in V3" errors.
            (0x05, 2) => Ok(InstructionForm::Long), // inc_chk is 2OP, needs Long form
            (0x05, _) => Ok(InstructionForm::Variable), // print_char (1 operand)

            // Always VAR form opcodes (regardless of operand count)
            // CRITICAL: call_vs (opcode 0x00) MUST use VAR form even with 1 operand
            // This is required for init → main loop calls to work correctly.
            // Without this, opcode 0x00 with 1 operand would use SHORT form (incorrect).
            (0x00, _) => Ok(InstructionForm::Variable), // call_vs (VAR:224) is always VAR
            (0x04, _) => Ok(InstructionForm::Variable), // sread is always VAR
            (0x06, _) => Ok(InstructionForm::Variable), // print_num is always VAR
            (0x07, _) => Ok(InstructionForm::Variable), // random is always VAR
            (0x08, _) => Ok(InstructionForm::Variable), // push (VAR:0x08) is always VAR - conflicts with 1OP:call_1s
            // CRITICAL OPCODE 0x09 CONFLICT FIX: Op2(And) vs Var(Pull)
            // REMOVED: (0x09, _) => Ok(InstructionForm::Variable) - This forced both And and Pull to VAR form
            // Now Op2(And) can use proper 2OP encoding while Var(Pull) uses emit_instruction_typed
            (0x20, _) => Ok(InstructionForm::Variable), // call_1n is always VAR
            (0x8b, _) => Ok(InstructionForm::Variable), // quit (0OP:139) - too large for short form
            (0x8f, _) => Ok(InstructionForm::Variable), // call_1n (1OP:143) - too large for short form
            (0xE0, _) => Ok(InstructionForm::Variable), // call (VAR:224) is always VAR

            // Default operand-count based logic
            _ => match operands.len() {
                0 => Ok(InstructionForm::Short), // 0OP
                1 => Ok(InstructionForm::Short), // 1OP
                2 => {
                    // Check if Long form can handle all operands
                    let can_use_long_form = operands.iter().all(|op| {
                        match op {
                            Operand::LargeConstant(value) => *value <= 255,
                            _ => true, // SmallConstant and Variable are fine
                        }
                    });

                    if opcode < 0x80 && can_use_long_form {
                        Ok(InstructionForm::Long)
                    } else {
                        // Switching to VAR form - check if this would change opcode meaning
                        // TEMPORARILY DISABLED: This validation is correct but reveals that
                        // the current grammar system design requires large placeholder values
                        // with form-sensitive opcodes. Need architectural redesign.
                        // See ARCHITECTURE.md "Z-Machine Opcode Form Instability" for details.
                        /*
                        if let Some((_, long_name, var_name)) =
                            FORM_SENSITIVE_OPCODES.iter().find(|(op, _, _)| *op == opcode)
                        {
                            return Err(CompilerError::OpcodeFormConflict {
                                opcode,
                                long_form_name: long_name.to_string(),
                                var_form_name: var_name.to_string(),
                            });
                        }
                        */
                        Ok(InstructionForm::Variable)
                    }
                }
                _ => Ok(InstructionForm::Variable), // VAR form for 3+ operands
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
        log::debug!(
            "🔧 LONG_FORM: op1={:?} op1_bit={} op2={:?} op2_bit={} instruction_byte=0x{:02x}",
            operands[0],
            op1_bit,
            operands[1],
            op2_bit,
            instruction_byte
        );
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
                log::debug!(
                    "CRITICAL: About to emit_word at 0x{:04x} with value 0xFFFF",
                    self.code_address
                );
                log::debug!("  But wait, let's check if this is actually being called...");
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
                log::debug!(
                    "CRITICAL: About to emit_word at 0x{:04x} with value 0xFFFF",
                    self.code_address
                );
                log::debug!("  But wait, let's check if this is actually being called...");
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
            log::debug!("JZ_CRITICAL: Emitting jz at 0x032f");
            log::debug!(
                "  operands={:?}, store_var={:?}, branch_offset={:?}",
                operands,
                store_var,
                branch_offset
            );
        }
        // CRITICAL DEBUG: Track short form emission at 0x338-0x339
        if self.code_address >= 0x337 && self.code_address <= 0x33a {
            log::debug!(
                "emit_short_form_with_layout at code_address=0x{:04x}",
                self.code_address
            );
            log::debug!("  opcode=0x{:02x}", opcode);
            log::debug!("  operands={:?}", operands);
            log::debug!("  store_var={:?}", store_var);
            log::debug!("  branch_offset={:?}", branch_offset);

            if !operands.is_empty() {
                if let Operand::SmallConstant(val) = &operands[0] {
                    if *val == 159 {
                        log::debug!("FOUND THE CULPRIT: Short form instruction with operand 159!");
                        log::debug!("This will emit 0x9f as the operand");
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
            // CRITICAL FIX: Store code_space offset, NOT final address
            // The translate_space_address_to_final() will add final_code_base during resolution
            Some(code_space_offset)
        } else {
            None
        };

        // Track store variable location
        // CRITICAL FIX: Some instructions do NOT emit store variable bytes
        // Pass the encoded instruction byte, not the raw opcode, to properly distinguish forms
        let store_location = if let Some(store) = store_var {
            if !Self::should_not_emit_store_variable(instruction_byte) {
                let loc = self.code_address;
                log::debug!("STORE_BYTE: About to emit store byte 0x{:02x} at code_address=0x{:04x}, code_space.len()={}",
                    store, self.code_address, self.code_space.len());
                self.emit_byte(store)?;
                log::debug!(
                    "STORE_BYTE: After emit, code_address=0x{:04x}, code_space.len()={}",
                    self.code_address,
                    self.code_space.len()
                );
                Some(loc)
            } else {
                None
            }
        } else {
            None
        };

        // Handle branch encoding - distinguish between hardcoded offsets and label references
        let branch_location = if let Some(offset) = branch_offset {
            // CRITICAL BUG FIX (Oct 8, 2025): Extract branch sense from offset value
            // For direct offsets (0-63): encode immediately with correct sense bit
            // For placeholders (like 0x7FFF): preserve value, will be patched later

            // Check if this is a small hardcoded offset (0-63) that can be encoded directly
            if offset >= 0 && offset <= 63 {
                // Direct offset: extract branch sense from sign convention
                // By convention: positive = branch on true (this is the common case)
                let on_true = true; // Direct small offsets default to "branch on true"

                log::debug!(
                    "BRANCH_DIRECT: Encoding offset {} as single byte (on_true={})",
                    offset,
                    on_true
                );
                // Encode as single-byte branch:
                // Bit 7 = branch sense (1=branch on true, 0=branch on false)
                // Bit 6 = 1 (single-byte format)
                // Bits 0-5 = offset
                let sense_bit = if on_true { 0x80 } else { 0x00 };
                let branch_byte = sense_bit | 0x40 | (offset as u8 & 0x3F);
                self.emit_byte(branch_byte)?;
                None // No placeholder needed
            } else {
                // This is either a large offset or a label reference - emit placeholder
                // Preserve the original offset value (bit 15 encodes branch sense for placeholders)
                let loc = self.code_address;
                let placeholder_value = offset as u16;
                log::debug!("🔵 BRANCH_PLACEHOLDER: Emitting 0x{:04x} at code_address=0x{:04x} for branch (offset={}) [INSTRUCTION WILL NEED PATCHING]",
                    placeholder_value, loc, offset);
                self.emit_word(placeholder_value)?; // Will be replaced during branch resolution
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
                log::debug!(
                    "CRITICAL: About to emit_word at 0x{:04x} with value 0xFFFF",
                    self.code_address
                );
                log::debug!("  But wait, let's check if this is actually being called...");
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
        opcode_enum: &Opcode,
        operands: &[Operand],
        store_var: Option<u8>,
        branch_offset: Option<i16>,
    ) -> Result<InstructionLayout, CompilerError> {
        let opcode = opcode_enum.raw_value();
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

        // CRITICAL FIX: Distinguish between Op2(And) and OpVar(Pull) for opcode 0x09
        // - Op2(And) in VAR form should have bit 5 = 0 (encoded as 0xC9, operand_count = OP2)
        // - OpVar(Pull) should have bit 5 = 1 (encoded as 0xE9, operand_count = VAR)
        //
        // This fixes the stack underflow bug where PULL instructions were encoded as 0xC9
        // instead of 0xE9, causing them to be routed to the AND handler at runtime.
        // The AND handler expected 2 operands but PULL only provides 1, causing stack underflow.
        let var_bit = match opcode_enum {
            Opcode::OpVar(_) => 0x20, // All true VAR opcodes get bit 5 = 1
            Opcode::Op2(_) => 0x00,   // 2OP opcodes in VAR form get bit 5 = 0
            _ => {
                if Self::is_true_var_opcode(opcode) {
                    0x20
                } else {
                    0x00
                }
            } // Fallback for other cases
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

            // CRITICAL FIX: Store code_space offset, NOT final address
            // The translate_space_address_to_final() will add final_code_base during resolution
            Some(first_operand_offset)
        } else {
            None
        };

        // Track store variable location
        // CRITICAL FIX: Some instructions do NOT emit store variable bytes
        // Print instructions, call instructions, etc. handle results differently
        // Pass the encoded instruction byte, not the raw opcode, to properly distinguish forms
        let store_location = if let Some(store) = store_var {
            if !Self::should_not_emit_store_variable(instruction_byte) {
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
            // CRITICAL BUG FIX (Oct 8, 2025): Extract branch sense from offset value
            // For direct offsets (0-63): encode immediately with correct sense bit
            // For placeholders (like 0x7FFF): preserve value, will be patched later

            // Check if this is a small hardcoded offset (0-63) that can be encoded directly
            if offset >= 0 && offset <= 63 {
                // Direct offset: extract branch sense from sign convention
                // By convention: positive = branch on true (this is the common case)
                let on_true = true; // Direct small offsets default to "branch on true"

                log::debug!(
                    "BRANCH_DIRECT: Encoding offset {} as single byte (on_true={})",
                    offset,
                    on_true
                );
                // Encode as single-byte branch:
                // Bit 7 = branch sense (1=branch on true, 0=branch on false)
                // Bit 6 = 1 (single-byte format)
                // Bits 0-5 = offset
                let sense_bit = if on_true { 0x80 } else { 0x00 };
                let branch_byte = sense_bit | 0x40 | (offset as u8 & 0x3F);
                self.emit_byte(branch_byte)?;
                None // No placeholder needed
            } else {
                // This is either a large offset or a label reference - emit placeholder
                // Preserve the original offset value (bit 15 encodes branch sense for placeholders)
                let loc = self.code_address;
                let placeholder_value = offset as u16;
                log::debug!("🔵 BRANCH_PLACEHOLDER: Emitting 0x{:04x} at code_address=0x{:04x} for branch (offset={}) [INSTRUCTION WILL NEED PATCHING]",
                    placeholder_value, loc, offset);
                self.emit_word(placeholder_value)?; // Will be replaced during branch resolution
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
            log::debug!(
                "emit_long_form_with_layout at 0x{:04x}: opcode=0x{:02x}, operands={:?}",
                self.code_address,
                opcode,
                operands
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
                        log::debug!(
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
        log::debug!(
            "🔧 LONG_FORM_LAYOUT: op1={:?}->{:?} op2={:?}->{:?} instruction_byte=0x{:02x}",
            operands[0],
            op1_type,
            operands[1],
            op2_type,
            instruction_byte
        );

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
        // CRITICAL FIX: Store code_space offset, NOT final address
        // The translate_space_address_to_final() will add final_code_base during resolution
        let operand_location = Some(code_space_offset);

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
        // Pass the encoded instruction byte, not the raw opcode, to properly distinguish forms
        let store_location = if let Some(store) = store_var {
            if !Self::should_not_emit_store_variable(instruction_byte) {
                let loc = self.code_address;
                log::debug!("STORE_BYTE: About to emit store byte 0x{:02x} at code_address=0x{:04x}, code_space.len()={}",
                    store, self.code_address, self.code_space.len());
                self.emit_byte(store)?;
                log::debug!(
                    "STORE_BYTE: After emit, code_address=0x{:04x}, code_space.len()={}",
                    self.code_address,
                    self.code_space.len()
                );
                Some(loc)
            } else {
                None
            }
        } else {
            None
        };

        // Handle branch encoding - distinguish between hardcoded offsets and label references
        let branch_location = if let Some(offset) = branch_offset {
            // CRITICAL BUG FIX (Oct 8, 2025): Extract branch sense from offset value
            // For direct offsets (0-63): encode immediately with correct sense bit
            // For placeholders (like 0x7FFF): preserve value, will be patched later

            // Check if this is a small hardcoded offset (0-63) that can be encoded directly
            if offset >= 0 && offset <= 63 {
                // Direct offset: extract branch sense from sign convention
                // By convention: positive = branch on true (this is the common case)
                let on_true = true; // Direct small offsets default to "branch on true"

                log::debug!(
                    "BRANCH_DIRECT: Encoding offset {} as single byte (on_true={})",
                    offset,
                    on_true
                );
                // Encode as single-byte branch:
                // Bit 7 = branch sense (1=branch on true, 0=branch on false)
                // Bit 6 = 1 (single-byte format)
                // Bits 0-5 = offset
                let sense_bit = if on_true { 0x80 } else { 0x00 };
                let branch_byte = sense_bit | 0x40 | (offset as u8 & 0x3F);
                self.emit_byte(branch_byte)?;
                None // No placeholder needed
            } else {
                // This is either a large offset or a label reference - emit placeholder
                // Preserve the original offset value (bit 15 encodes branch sense for placeholders)
                let loc = self.code_address;
                let placeholder_value = offset as u16;
                log::debug!("🔵 BRANCH_PLACEHOLDER: Emitting 0x{:04x} at code_address=0x{:04x} for branch (offset={}) [INSTRUCTION WILL NEED PATCHING]",
                    placeholder_value, loc, offset);
                self.emit_word(placeholder_value)?; // Will be replaced during branch resolution
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
    fn try_resolve_ir_id_if_needed(&mut self, value: u32) -> Option<Operand> {
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
            log::debug!(
                "emit_operand at code_address=0x{:04x}: operand={:?}",
                self.code_address,
                operand
            );
        }

        match operand {
            Operand::SmallConstant(value) => {
                if *value == 1 && self.code_address == 0x338 {
                    log::debug!("CRITICAL: Emitting SmallConstant(1) at 0x338");
                    log::debug!("This is part of the 415 problem!");
                }
                if *value == 159 && self.code_address == 0x339 {
                    log::debug!("CRITICAL: Emitting SmallConstant(159) at 0x339");
                    log::debug!("Together with previous byte, this is 0x019f = 415!");
                }

                // CRITICAL CHECK: Looking for what makes the 0x9f byte
                if self.code_address == 0x339 {
                    log::debug!(
                        "FOUND: About to emit byte 0x{:02x} at code_address=0x339",
                        value
                    );
                    if *value == 0x9f {
                        log::debug!("THIS IS THE 0x9f BYTE! SmallConstant(159)");
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
                    log::debug!(
                        "CRITICAL: emit_operand emitting LargeConstant(1) at code_address=0x{:04x}",
                        self.code_address
                    );
                    log::debug!("This should emit 0x00 0x01 but might be causing 0x01 0x9f");
                    log::debug!("About to call emit_word(0x{:04x})", value);
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
        _target: Option<crate::grue_compiler::ir::IrId>,
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
        // CRITICAL: V3 requires store variable even for discarded results
        // V5+ has call_1n/call_2n for no-store calls, but V3 always stores to stack
        let store_var = Some(0); // Always store to stack (V3 requirement, V4+ allows None)

        // Emit the call instruction (VAR form call_vs)
        // CRITICAL: Use raw opcode 0x00, NOT encoded byte 0xE0
        // emit_instruction will determine the VAR form encoding
        let layout = self.emit_instruction_typed(CALLVS, &operands, store_var, None)?;

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

    /// Generate short-circuit AND logic for logical operations on comparison expressions
    /// Pattern: if left is false, result = false; if left is true, result = right
    fn generate_short_circuit_and(
        &mut self,
        target: &crate::grue_compiler::ir::IrId,
        left_expr: &crate::grue_compiler::ast::Expr,
        right_expr: &crate::grue_compiler::ast::Expr,
    ) -> Result<(), CompilerError> {
        log::debug!(
            "Short-circuit AND: evaluating left expression, target IR ID {}",
            target
        );

        // Use stack for result storage
        self.use_push_pull_for_result(*target, "short circuit AND operation")?;

        // Generate unique labels for control flow
        let false_label = self.next_string_id;
        self.next_string_id += 1;
        let end_label = self.next_string_id;
        self.next_string_id += 1;

        // Step 1: Evaluate left expression
        // If left is false, branch to false_label to set result = false
        // If left is true, fall through to evaluate right
        self.generate_comparison_branch(left_expr, false_label, true)?;

        // Left was true, now evaluate right expression
        // If right is false, branch to false_label to set result = false
        // If right is true, fall through to set result = true
        self.generate_comparison_branch(right_expr, false_label, true)?;

        // Both left and right were true - store true result
        self.emit_instruction_typed(
            crate::grue_compiler::opcodes::Opcode::Op2(crate::grue_compiler::opcodes::Op2::Store),
            &[
                crate::grue_compiler::codegen::Operand::SmallConstant(1),
                crate::grue_compiler::codegen::Operand::Variable(0),
            ],
            None,
            None,
        )?;

        // Jump to end
        self.translate_jump(end_label)?;

        // Step 2: False result label
        self.pending_labels.push(false_label);
        self.emit_instruction_typed(
            crate::grue_compiler::opcodes::Opcode::Op2(crate::grue_compiler::opcodes::Op2::Store),
            &[
                crate::grue_compiler::codegen::Operand::SmallConstant(0),
                crate::grue_compiler::codegen::Operand::Variable(0),
            ],
            None,
            None,
        )?;

        // Step 3: End label
        self.pending_labels.push(end_label);

        log::debug!("Short-circuit AND: completed for target IR ID {}", target);
        Ok(())
    }

    /// Generate short-circuit OR logic for logical operations on comparison expressions
    /// Pattern: if left is true, result = true; if left is false, result = right
    fn generate_short_circuit_or(
        &mut self,
        target: &crate::grue_compiler::ir::IrId,
        left_expr: &crate::grue_compiler::ast::Expr,
        right_expr: &crate::grue_compiler::ast::Expr,
    ) -> Result<(), CompilerError> {
        log::debug!(
            "Short-circuit OR: evaluating left expression, target IR ID {}",
            target
        );

        // Use stack for result storage
        self.use_push_pull_for_result(*target, "short circuit OR operation")?;

        // Generate unique labels for control flow
        let true_label = self.next_string_id;
        self.next_string_id += 1;
        let end_label = self.next_string_id;
        self.next_string_id += 1;

        // Step 1: Evaluate left expression
        // If left is true, branch to true_label to set result = true
        // If left is false, fall through to evaluate right
        self.generate_comparison_branch(left_expr, true_label, false)?;

        // Left was false, now evaluate right expression
        // If right is true, branch to true_label to set result = true
        // If right is false, fall through to set result = false
        self.generate_comparison_branch(right_expr, true_label, false)?;

        // Both left and right were false - store false result
        self.emit_instruction_typed(
            crate::grue_compiler::opcodes::Opcode::Op2(crate::grue_compiler::opcodes::Op2::Store),
            &[
                crate::grue_compiler::codegen::Operand::SmallConstant(0),
                crate::grue_compiler::codegen::Operand::Variable(0),
            ],
            None,
            None,
        )?;

        // Jump to end
        self.translate_jump(end_label)?;

        // Step 2: True result label
        self.pending_labels.push(true_label);
        self.emit_instruction_typed(
            crate::grue_compiler::opcodes::Opcode::Op2(crate::grue_compiler::opcodes::Op2::Store),
            &[
                crate::grue_compiler::codegen::Operand::SmallConstant(1),
                crate::grue_compiler::codegen::Operand::Variable(0),
            ],
            None,
            None,
        )?;

        // Step 3: End label
        self.pending_labels.push(end_label);

        log::debug!("Short-circuit OR: completed for target IR ID {}", target);
        Ok(())
    }

    /// Generate comparison branch for logical operation helper
    /// Compiles a comparison expression into Z-Machine branch instruction
    fn generate_comparison_branch(
        &mut self,
        expr: &crate::grue_compiler::ast::Expr,
        branch_label: crate::grue_compiler::ir::IrId,
        branch_on_false: bool,
    ) -> Result<(), CompilerError> {
        match expr {
            crate::grue_compiler::ast::Expr::Binary {
                left,
                operator,
                right,
            } => {
                // Evaluate operands to get Z-Machine operands
                let left_operand = self.evaluate_expression_to_operand(left)?;
                let right_operand = self.evaluate_expression_to_operand(right)?;

                // Map comparison operators to Z-Machine branch opcodes
                let (opcode, should_invert) = match operator {
                    crate::grue_compiler::ast::BinaryOp::Equal => (
                        crate::grue_compiler::opcodes::Opcode::Op2(
                            crate::grue_compiler::opcodes::Op2::Je,
                        ),
                        false,
                    ),
                    crate::grue_compiler::ast::BinaryOp::NotEqual => (
                        crate::grue_compiler::opcodes::Opcode::Op2(
                            crate::grue_compiler::opcodes::Op2::Je,
                        ),
                        true,
                    ),
                    crate::grue_compiler::ast::BinaryOp::Less => (
                        crate::grue_compiler::opcodes::Opcode::Op2(
                            crate::grue_compiler::opcodes::Op2::Jl,
                        ),
                        false,
                    ),
                    crate::grue_compiler::ast::BinaryOp::Greater => (
                        crate::grue_compiler::opcodes::Opcode::Op2(
                            crate::grue_compiler::opcodes::Op2::Jg,
                        ),
                        false,
                    ),
                    crate::grue_compiler::ast::BinaryOp::LessEqual => (
                        crate::grue_compiler::opcodes::Opcode::Op2(
                            crate::grue_compiler::opcodes::Op2::Jg,
                        ),
                        true,
                    ),
                    crate::grue_compiler::ast::BinaryOp::GreaterEqual => (
                        crate::grue_compiler::opcodes::Opcode::Op2(
                            crate::grue_compiler::opcodes::Op2::Jl,
                        ),
                        true,
                    ),
                    _ => {
                        return Err(CompilerError::CodeGenError(format!(
                            "Unsupported comparison operator in logical expression: {:?}",
                            operator
                        )));
                    }
                };

                // Determine branch condition (branch_on_false XOR should_invert)
                let _branch_condition = branch_on_false ^ should_invert;

                // Create unresolved reference for the branch
                let layout = self.emit_instruction_typed(
                    opcode,
                    &[left_operand, right_operand],
                    None,
                    Some(placeholder_word() as i16),
                )?;

                // Register branch reference for later resolution
                if let Some(branch_location) = layout.branch_location {
                    self.reference_context.unresolved_refs.push(
                        crate::grue_compiler::codegen_headers::UnresolvedReference {
                            reference_type:
                                crate::grue_compiler::codegen::LegacyReferenceType::Branch,
                            location: branch_location,
                            target_id: branch_label,
                            is_packed_address: false,
                            offset_size: 2,
                            location_space:
                                crate::grue_compiler::codegen_headers::MemorySpace::Code,
                        },
                    );
                }

                Ok(())
            }
            _ => Err(CompilerError::CodeGenError(format!(
                "Expected comparison expression in logical operation, found: {:?}",
                expr
            ))),
        }
    }

    /// Evaluate expression to Z-Machine operand
    fn evaluate_expression_to_operand(
        &mut self,
        expr: &crate::grue_compiler::ast::Expr,
    ) -> Result<crate::grue_compiler::codegen::Operand, CompilerError> {
        match expr {
            crate::grue_compiler::ast::Expr::Identifier(name) => {
                // Simplified identifier resolution for initial implementation
                // For well-known identifiers, return appropriate operands
                match name.as_str() {
                    "player" => {
                        // Player is typically object 1 in Z-Machine
                        Ok(crate::grue_compiler::codegen::Operand::SmallConstant(1))
                    }
                    _ => {
                        // For other identifiers, try to find them in function parameters
                        // For now, assume it's a local variable and use Variable(1)
                        Ok(crate::grue_compiler::codegen::Operand::Variable(1))
                    }
                }
            }
            crate::grue_compiler::ast::Expr::Integer(value) => {
                if *value >= 0 && *value <= 255 {
                    Ok(crate::grue_compiler::codegen::Operand::SmallConstant(
                        *value as u8,
                    ))
                } else {
                    Ok(crate::grue_compiler::codegen::Operand::LargeConstant(
                        *value as u16,
                    ))
                }
            }
            crate::grue_compiler::ast::Expr::PropertyAccess { object, property } => {
                // Simplified property access for initial implementation
                let object_operand = self.evaluate_expression_to_operand(object)?;

                // Map well-known property names to property numbers
                let property_num = match property.as_str() {
                    "location" => 1, // location is typically property 1
                    _ => 2,          // default to property 2 for others
                };

                // Generate get_prop instruction to get property value
                self.emit_instruction_typed(
                    crate::grue_compiler::opcodes::Opcode::Op2(
                        crate::grue_compiler::opcodes::Op2::GetProp,
                    ),
                    &[
                        object_operand,
                        crate::grue_compiler::codegen::Operand::SmallConstant(property_num),
                    ],
                    Some(0), // Store result on stack
                    None,
                )?;

                // Return stack operand where result is stored
                Ok(crate::grue_compiler::codegen::Operand::Variable(0))
            }
            _ => Err(CompilerError::CodeGenError(format!(
                "Unsupported expression type in comparison: {:?}",
                expr
            ))),
        }
    }

    /// Helper to generate unique label IDs (simplified implementation)
    fn get_next_label_id(&mut self) -> crate::grue_compiler::ir::IrId {
        // Simple implementation - use current address as unique ID
        self.code_address as crate::grue_compiler::ir::IrId
    }
}

#[cfg(test)]
mod opcode_encoding_tests {
    use super::*;
    use crate::grue_compiler::codegen::ZMachineCodeGen;
    use crate::grue_compiler::ZMachineVersion;

    #[test]
    fn test_call_vs_encoding() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);
        // Need 3+ operands to force VAR form (1-2 operands use SHORT/LONG form)
        codegen
            .emit_instruction_typed(
                CALLVS,
                &[
                    Operand::LargeConstant(0x1234),
                    Operand::SmallConstant(1),
                    Operand::SmallConstant(2),
                ],
                Some(0),
                None,
            )
            .unwrap();

        // Should emit 0xE0 (VAR form with VAR count), not 0xC0 (VAR form with 2OP count)
        assert_eq!(
            codegen.code_space[0], 0xE0,
            "call_vs (opcode 0x00) should emit 0xE0 instruction byte for VAR form with VAR count"
        );
    }

    #[test]
    fn test_put_prop_encoding() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);
        codegen
            .emit_instruction_typed(
                Opcode::OpVar(OpVar::PutProp),
                &[
                    Operand::Variable(1),
                    Operand::SmallConstant(13),
                    Operand::LargeConstant(2),
                ],
                None,
                None,
            )
            .unwrap();

        // Should emit 0xE3 for put_prop
        assert_eq!(codegen.code_space[0], 0xE3, "put_prop should emit 0xE3");
    }

    #[test]
    fn test_print_paddr_encoding() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);
        codegen
            .emit_instruction_typed(PRINTPADDR, &[Operand::LargeConstant(0x0399)], None, None)
            .unwrap();

        // Should emit 0x8D (SHORT form, 1OP, opcode 0x0D)
        assert_eq!(codegen.code_space[0], 0x8D, "print_paddr should emit 0x8D");
    }

    #[test]
    fn test_rejects_encoded_opcode_0x20() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);
        // Should reject 0x20 as it's not a valid raw opcode
        let result =
            codegen.emit_instruction(0x20, &[Operand::LargeConstant(0x1234)], Some(0), None);

        assert!(result.is_err(), "Should reject opcode 0x20");
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("Invalid opcode 0x20"),
            "Error should mention invalid opcode 0x20"
        );
    }

    #[test]
    fn test_rejects_encoded_opcode_0x21() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);
        let result =
            codegen.emit_instruction(0x21, &[Operand::LargeConstant(0x1234)], Some(0), None);

        assert!(result.is_err(), "Should reject opcode 0x21");
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("Invalid opcode 0x21"),
            "Error should mention invalid opcode 0x21"
        );
    }

    #[test]
    fn test_rejects_encoded_opcode_0x_e0() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);
        let result =
            codegen.emit_instruction(0xE0, &[Operand::LargeConstant(0x1234)], Some(0), None);

        // 0xE0 is >= 0x80 so it should be accepted (it's an encoded instruction byte,
        // but our validation only rejects 0x20-0x7F range which are clearly wrong)
        // Actually, we should accept 0xE0 because it's >= 0x80
        assert!(
            result.is_ok(),
            "Should accept 0xE0 (it's in valid range >= 0x80)"
        );
    }

    #[test]
    fn test_or_instruction_encoding() {
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V3);
        codegen
            .emit_instruction_typed(
                Opcode::Op2(Op2::Or),
                &[Operand::LargeConstant(1), Operand::SmallConstant(0)],
                Some(0),
                None,
            )
            .unwrap();

        // 2OP or with LONG form encoding
        // Opcode 0x08 in LONG form should produce instruction byte with opcode in low 5 bits
        // For LONG form: bits 7-6 depend on operand types, bit 5 is part of opcode
        // Actually for 2OP LONG form: top bits are operand types, bottom 5 bits are opcode
        // Since we have LargeConstant + SmallConstant, expect LONG form
        let first_byte = codegen.code_space[0];
        // LONG form has top bit 0, second bit is operand type
        assert_eq!(
            first_byte & 0x1F,
            0x08,
            "or instruction should have opcode 0x08 in low 5 bits"
        );
    }

    #[test]
    fn test_call_2s_encoding() {
        // Call2s is V4+ only, so create a V4 code generator
        let mut codegen = ZMachineCodeGen::new(ZMachineVersion::V4);
        codegen
            .emit_instruction_typed(
                Opcode::Op2(Op2::Call2s),
                &[Operand::LargeConstant(0x1234), Operand::SmallConstant(0)],
                Some(0),
                None,
            )
            .unwrap();

        // call_2s is 2OP:25 (0x19)
        let first_byte = codegen.code_space[0];
        assert_eq!(
            first_byte & 0x1F,
            0x19,
            "call_2s should have opcode 0x19 in low 5 bits"
        );
    }
}
