/// Object system operations for Z-Machine interpreter
///
/// This module handles all object-related operations including:
/// - Object hierarchy operations (get_sibling, get_child, get_parent, insert_obj, remove_obj)
/// - Object properties (get_prop, put_prop, get_prop_addr, get_next_prop, get_prop_len)
/// - Object attributes (test_attr, set_attr, clear_attr)
/// - Object relationships (jin - test if object is inside another)
/// - Object display (print_obj - print object's short name)
///
/// These operations form the core of Z-Machine's object-oriented game world,
/// enabling complex interactions between game objects, rooms, and items.
use crate::instruction::Instruction;
use crate::interpreter::{ExecutionResult, Interpreter};
use log::{debug, error};

impl Interpreter {
    /// Handle object system opcodes
    pub fn execute_object_op(
        &mut self,
        inst: &Instruction,
        operands: &[u16],
    ) -> Result<ExecutionResult, String> {
        match (inst.opcode, &inst.operand_count) {
            // ---- 1OP OBJECT OPERATIONS ----

            // 1OP:0x01 - get_sibling
            (0x01, crate::instruction::OperandCount::OP1) => {
                // get_sibling
                let sibling = self.vm.get_sibling(operands[0])?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, sibling)?;
                }
                self.do_branch(inst, sibling != 0)
            }

            // 1OP:0x02 - get_child
            (0x02, crate::instruction::OperandCount::OP1) => {
                // get_child
                let child = self.vm.get_child(operands[0])?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, child)?;
                }
                self.do_branch(inst, child != 0)
            }

            // 1OP:0x03 - get_parent
            (0x03, crate::instruction::OperandCount::OP1) => {
                // get_parent
                debug!(
                    "get_parent: obj_num={} at PC {:05x}",
                    operands[0],
                    self.vm.pc - inst.size as u32
                );
                let parent = self.vm.get_parent(operands[0])?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, parent)?;
                }
                Ok(ExecutionResult::Continue)
            }

            // 1OP:0x04 - get_prop_len
            (0x04, crate::instruction::OperandCount::OP1) => {
                // get_prop_len - get the length of a property given its data address
                debug!(
                    "get_prop_len: prop_addr={:04x} at PC {:05x}",
                    operands[0],
                    self.vm.pc - inst.size as u32
                );

                let prop_len = if operands[0] == 0 {
                    0
                } else {
                    // In Z-Machine v3, the size byte is immediately before the property data
                    // The size byte encodes: top 3 bits = size-1, bottom 5 bits = property number
                    let size_byte_addr = (operands[0] as u32).saturating_sub(1);
                    let size_byte = self.vm.read_byte(size_byte_addr);
                    let size = ((size_byte >> 5) & 0x07) + 1;
                    debug!(
                        "  Size byte at {:04x}: {:02x}, property size: {}",
                        size_byte_addr, size_byte, size
                    );
                    size as u16
                };

                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, prop_len)?;
                }
                Ok(ExecutionResult::Continue)
            }

            // 1OP:0x09 - remove_obj
            (0x09, crate::instruction::OperandCount::OP1) => {
                // remove_obj
                let obj_num = operands[0];
                debug!(
                    "remove_obj: obj_num={} at PC {:05x}",
                    obj_num,
                    self.vm.pc - inst.size as u32
                );
                self.vm.remove_object(obj_num)?;
                Ok(ExecutionResult::Continue)
            }

            // 1OP:0x0A - print_obj
            (0x0A, crate::instruction::OperandCount::OP1) => {
                // print_obj - print short name of object
                let obj_num = operands[0];
                log::error!(
                    "🎯 PRINT_OBJ DEBUG: obj_num={} at PC {:05x}",
                    obj_num,
                    self.vm.pc - inst.size as u32
                );
                log::error!(
                    "🎯 Stack depth: {}, Call stack depth: {}",
                    self.vm.stack.len(),
                    self.vm.call_stack.len()
                );

                // Validate object number range BEFORE accessing
                if obj_num == 0 || obj_num > 255 {
                    error!(
                        "INVALID OBJECT NUMBER: {} is out of valid range (1-255)",
                        obj_num
                    );
                    error!("This suggests stack corruption or invalid instruction operand");
                    error!(
                        "Current instruction size: {}, PC before: {:05x}",
                        inst.size,
                        self.vm.pc - inst.size as u32
                    );
                    return Err(format!("Invalid object number: {}", obj_num));
                }

                // Get object's short description
                match self.vm.get_object_name(obj_num) {
                    Ok(name) => {
                        log::error!("🎯 Successfully got object name: '{}'", name);
                        self.output_text(&name)?;
                    }
                    Err(e) => {
                        debug!("Failed to get object {} name: {}", obj_num, e);
                        // Print nothing for invalid objects (standard behavior)
                    }
                }
                Ok(ExecutionResult::Continue)
            }

            // ---- 2OP OBJECT OPERATIONS ----

            // 2OP:0x06 - jin (test if object is inside another)
            (0x06, crate::instruction::OperandCount::OP2) => {
                // jin
                // Check if obj1 is inside obj2 (obj1's parent is obj2)
                let parent = self.vm.get_parent(operands[0])?;
                let condition = parent == operands[1];
                self.do_branch(inst, condition)
            }

            // 2OP:0x0A - test_attr
            (0x0A, crate::instruction::OperandCount::OP2) => {
                // test_attr
                let obj_num = operands[0];
                let attr_num = operands[1] as u8;
                let result = self.vm.test_attribute(obj_num, attr_num)?;
                let current_pc = self.vm.pc - inst.size as u32;

                // Let's follow the natural flow
                if current_pc == 0x4f7e {
                    debug!(
                        "test_attr at {:05x}: obj={}, attr={}, result={}",
                        current_pc, obj_num, attr_num, result
                    );
                }

                self.do_branch(inst, result)
            }

            // 2OP:0x0B - set_attr
            (0x0B, crate::instruction::OperandCount::OP2) => {
                // set_attr
                let obj_num = operands[0];
                let attr_num = operands[1] as u8;
                self.vm.set_attribute(obj_num, attr_num, true)?;
                Ok(ExecutionResult::Continue)
            }

            // 2OP:0x0C - clear_attr
            (0x0C, crate::instruction::OperandCount::OP2) => {
                // clear_attr
                let obj_num = operands[0];
                let attr_num = operands[1] as u8;
                if attr_num > 31 {
                    debug!(
                        "clear_attr: obj={}, attr={} at PC {:05x}",
                        obj_num,
                        attr_num,
                        self.vm.pc - inst.size as u32
                    );
                }
                self.vm.set_attribute(obj_num, attr_num, false)?;
                Ok(ExecutionResult::Continue)
            }

            // 2OP:0x0E - insert_obj
            (0x0E, crate::instruction::OperandCount::OP2) => {
                // insert_obj
                let current_pc = self.vm.pc - inst.size as u32;
                debug!(
                    "insert_obj: obj={}, dest={} at PC {:05x}",
                    operands[0], operands[1], current_pc
                );
                if operands[0] == 0 {
                    debug!("❌ insert_obj Z-Machine opcode called with object 0!");
                    debug!("   operands: {:?}", operands);
                    debug!("   instruction: {:?}", inst);
                }
                self.vm.insert_object(operands[0], operands[1])?;
                Ok(ExecutionResult::Continue)
            }

            // 2OP:0x11 - get_prop
            (0x11, crate::instruction::OperandCount::OP2) => {
                // get_prop
                let obj_num = operands[0];
                let prop_num = operands[1] as u8;

                // Debug logging for object 0 case (the Frotz compatibility issue)
                if obj_num == 0 {
                    debug!("WARNING: get_prop called with object 0 at PC {:05x} - this should not happen!", self.vm.pc);
                    debug!("  Property number: {}", prop_num);
                    debug!("  This likely means Variable(16) returned 0 instead of 1 (player object number)");
                }

                let value = self.vm.get_property(obj_num, prop_num)?;

                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, value)?;
                }
                Ok(ExecutionResult::Continue)
            }

            // 2OP:0x12 - get_prop_addr
            (0x12, crate::instruction::OperandCount::OP2) => {
                // get_prop_addr
                let obj_num = operands[0];
                let prop_num = operands[1] as u8;
                let addr = self.vm.get_property_addr(obj_num, prop_num)? as u16;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, addr)?;
                }
                Ok(ExecutionResult::Continue)
            }

            // 2OP:0x13 - get_next_prop
            (0x13, crate::instruction::OperandCount::OP2) => {
                // get_next_prop
                let obj_num = operands[0];
                let prop_num = if operands.len() >= 2 {
                    operands[1] as u8
                } else {
                    0u8
                };
                let next_prop = self.vm.get_next_property(obj_num, prop_num)? as u16;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, next_prop)?;
                }
                Ok(ExecutionResult::Continue)
            }

            // ---- VAR OBJECT OPERATIONS ----

            // VAR:0x03 - put_prop
            (0x03, crate::instruction::OperandCount::VAR) => {
                // put_prop
                debug!(
                    "put_prop at PC {:05x}: operands={:?}",
                    self.vm.pc - inst.size as u32,
                    operands
                );
                if operands.len() < 3 {
                    return Err("put_prop requires 3 operands".to_string());
                }
                let obj_num = operands[0];
                let prop_num = operands[1] as u8;
                let value = operands[2];
                debug!(
                    "put_prop: obj={}, prop={}, value={}",
                    obj_num, prop_num, value
                );
                self.vm.put_property(obj_num, prop_num, value)?;
                debug!("put_prop completed successfully");
                Ok(ExecutionResult::Continue)
            }

            // VAR:0x13 - Special case: get_next_prop or output_stream disambiguation
            (0x13, crate::instruction::OperandCount::VAR) => {
                // This opcode can be either get_next_prop or output_stream in VAR form
                // get_next_prop stores a result, output_stream does not
                if inst.store_var.is_some() {
                    // This is get_next_prop (VAR form of 2OP:19)
                    let obj_num = operands[0];
                    let prop_num = if operands.len() >= 2 {
                        operands[1] as u8
                    } else {
                        0u8
                    };
                    debug!("VAR get_next_prop: obj={:04x}, prop={}", obj_num, prop_num);
                    let next_prop = self.vm.get_next_property(obj_num, prop_num)? as u16;
                    if let Some(store_var) = inst.store_var {
                        self.vm.write_variable(store_var, next_prop)?;
                    }
                    Ok(ExecutionResult::Continue)
                } else {
                    // This is output_stream - not an object operation, should not reach here
                    Err(
                        "VAR:0x13 without store_var should be output_stream, not object operation"
                            .to_string(),
                    )
                }
            }

            _ => Err(format!(
                "Unhandled object opcode: {:02x} with operand count {:?}",
                inst.opcode, inst.operand_count
            )),
        }
    }

    /// Check if an opcode is an object operation
    /// Note: For VAR:0x13, this requires additional context (store_var check) in the caller
    pub fn is_object_opcode(opcode: u8, operand_count: &crate::instruction::OperandCount) -> bool {
        matches!(
            (opcode, operand_count),
            // 1OP object operations
            (0x01, crate::instruction::OperandCount::OP1) |  // get_sibling
            (0x02, crate::instruction::OperandCount::OP1) |  // get_child
            (0x03, crate::instruction::OperandCount::OP1) |  // get_parent
            (0x04, crate::instruction::OperandCount::OP1) |  // get_prop_len
            (0x09, crate::instruction::OperandCount::OP1) |  // remove_obj
            (0x0A, crate::instruction::OperandCount::OP1) |  // print_obj
            // 2OP object operations
            (0x06, crate::instruction::OperandCount::OP2) |  // jin
            (0x0A, crate::instruction::OperandCount::OP2) |  // test_attr
            (0x0B, crate::instruction::OperandCount::OP2) |  // set_attr
            (0x0C, crate::instruction::OperandCount::OP2) |  // clear_attr
            (0x0E, crate::instruction::OperandCount::OP2) |  // insert_obj
            (0x11, crate::instruction::OperandCount::OP2) |  // get_prop
            (0x12, crate::instruction::OperandCount::OP2) |  // get_prop_addr
            (0x13, crate::instruction::OperandCount::OP2) |  // get_next_prop
            // VAR object operations
            (0x03, crate::instruction::OperandCount::VAR) // put_prop
                                                          // Note: VAR:0x13 is handled specially in the interpreter routing
        )
    }

    /// Check if a VAR:0x13 opcode should be routed to the object module
    /// This handles the get_next_prop vs output_stream disambiguation
    pub fn is_var_13_object_opcode(inst: &crate::instruction::Instruction) -> bool {
        inst.opcode == 0x13
            && inst.operand_count == crate::instruction::OperandCount::VAR
            && inst.store_var.is_some()
    }
}
