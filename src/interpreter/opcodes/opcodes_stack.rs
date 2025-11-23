/// Stack and routine call operations for Z-Machine interpreter
///
/// This module handles all stack-related opcodes including:
/// - Stack manipulation (push, pull, pop)
/// - Routine calls and returns (call, ret, ret_popped)
/// - Call stack management
///
/// These operations are fundamental to Z-Machine execution as they manage
/// the call stack, local/global variable access, and routine execution flow.
use crate::interpreter::core::instruction::Instruction;
use crate::interpreter::{ExecutionResult, Interpreter};
use log::debug;

impl Interpreter {
    /// Handle stack and call-related opcodes
    pub fn execute_stack_op(
        &mut self,
        inst: &Instruction,
        operands: &[u16],
    ) -> Result<ExecutionResult, String> {
        match (inst.opcode, &inst.operand_count) {
            // ---- 0OP STACK OPERATIONS ----

            // 0OP:0x08 - ret_popped
            (0x08, crate::interpreter::core::instruction::OperandCount::OP0) => {
                debug!("ret_popped");
                let value = self.vm.pop()?;
                self.do_return(value)
            }

            // 0OP:0x09 - pop (V1-4) / catch (V5+)
            (0x09, crate::interpreter::core::instruction::OperandCount::OP0) => {
                if self.vm.game.header.version <= 4 {
                    debug!("pop");
                    self.vm.pop()?;
                    Ok(ExecutionResult::Continue)
                } else {
                    debug!("catch");
                    // catch: store call stack depth
                    if let Some(store_var) = inst.store_var {
                        self.vm
                            .write_variable(store_var, self.vm.call_stack.len() as u16)?;
                    }
                    Ok(ExecutionResult::Continue)
                }
            }

            // ---- 1OP STACK OPERATIONS ----

            // 1OP:0x0B - ret (return with value)
            (0x0B, crate::interpreter::core::instruction::OperandCount::OP1) => {
                debug!("ret {}", operands[0]);
                self.do_return(operands[0])
            }

            // 1OP:0x08 - call_1s (call routine, store result)
            (0x08, crate::interpreter::core::instruction::OperandCount::OP1) => {
                debug!("call_1s {:04x}", operands[0]);
                self.do_call(operands[0], &[], inst.store_var)?;
                Ok(ExecutionResult::Called)
            }

            // ---- VAR STACK OPERATIONS ----

            // VAR:0x00 - call (call routine with arguments)
            (0x00, crate::interpreter::core::instruction::OperandCount::VAR) => {
                let packed_addr = operands[0];
                debug!("call routine at packed address {:04x}", packed_addr);

                if packed_addr == 0 {
                    // Call to address 0 returns false (0)
                    if let Some(store_var) = inst.store_var {
                        self.vm.write_variable(store_var, 0)?;
                    }
                    Ok(ExecutionResult::Continue)
                } else {
                    let args = if operands.len() > 1 {
                        &operands[1..operands.len()]
                    } else {
                        &[]
                    };
                    self.do_call(packed_addr, args, inst.store_var)?;
                    Ok(ExecutionResult::Called)
                }
            }

            // VAR:0x08 - push (push value onto stack)
            (0x08, crate::interpreter::core::instruction::OperandCount::VAR) => {
                debug!("push {}", operands[0]);
                if !operands.is_empty() {
                    self.vm.push(operands[0])?;
                }
                Ok(ExecutionResult::Continue)
            }

            // VAR:0x09 - pull (pop from stack and store)
            (0x09, crate::interpreter::core::instruction::OperandCount::VAR) => {
                if !inst.operands.is_empty() {
                    let current_pc = self.vm.pc - inst.size as u32;
                    if (0x06f70..=0x06fa0).contains(&current_pc) {
                        debug!(
                            "pull at {:05x}: stack depth before pop: {}",
                            current_pc,
                            self.vm.stack.len()
                        );
                    }
                    let value = self.vm.pop()?;
                    // Use the raw operand value, not the resolved one
                    // (Variable 0 as destination means V00, not pop)
                    let var_num = inst.operands[0] as u8;
                    if (0x06f70..=0x06fa0).contains(&current_pc) {
                        debug!(
                            "pull at {:05x}: storing popped value {:04x} into V{:02x}",
                            current_pc, value, var_num
                        );
                    }
                    self.vm.write_variable(var_num, value)?;
                }
                Ok(ExecutionResult::Continue)
            }

            _ => Err(format!(
                "Unhandled stack opcode: {:02x} with operand count {:?}",
                inst.opcode, inst.operand_count
            )),
        }
    }

    /// Check if an opcode is a stack operation
    pub fn is_stack_opcode(
        opcode: u8,
        operand_count: &crate::interpreter::core::instruction::OperandCount,
    ) -> bool {
        matches!(
            (opcode, operand_count),
            // 0OP stack operations
            (0x08, crate::interpreter::core::instruction::OperandCount::OP0) |  // ret_popped
            (0x09, crate::interpreter::core::instruction::OperandCount::OP0) |  // pop/catch
            // 1OP stack operations  
            (0x0B, crate::interpreter::core::instruction::OperandCount::OP1) |  // ret
            (0x08, crate::interpreter::core::instruction::OperandCount::OP1) |  // call_1s
            // VAR stack operations
            (0x00, crate::interpreter::core::instruction::OperandCount::VAR) |  // call
            (0x08, crate::interpreter::core::instruction::OperandCount::VAR) |  // push
            (0x09, crate::interpreter::core::instruction::OperandCount::VAR) // pull
        )
    }
}
