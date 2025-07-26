/// Mathematical and logical operations for Z-Machine interpreter
///
/// This module handles all arithmetic and bitwise operations including:
/// - Arithmetic operations (add, sub, mul, div, mod)
/// - Bitwise operations (and, or, not)
/// - Version-specific handling for operations that changed between versions
///
/// These operations form the computational core of Z-Machine games,
/// enabling everything from simple counters to complex game mechanics.
use crate::instruction::Instruction;
use crate::interpreter::{ExecutionResult, Interpreter};
use log::debug;

impl Interpreter {
    /// Handle mathematical and logical opcodes
    pub fn execute_math_op(
        &mut self,
        inst: &Instruction,
        operands: &[u16],
    ) -> Result<ExecutionResult, String> {
        match (inst.opcode, &inst.operand_count) {
            // ---- 1OP MATH OPERATIONS ----

            // 1OP:0x0F - not (V1-4) / call_1n (V5+)
            (0x0F, crate::instruction::OperandCount::OP1) => {
                if self.vm.game.header.version <= 4 {
                    // Bitwise NOT
                    debug!("not {}", operands[0]);
                    if let Some(store_var) = inst.store_var {
                        self.vm.write_variable(store_var, !operands[0])?;
                    }
                    Ok(ExecutionResult::Continue)
                } else {
                    // In v5+, this becomes call_1n - should be handled by stack module
                    Err(format!(
                        "1OP:0x0F in v5+ should be handled by stack module, not math"
                    ))
                }
            }

            // ---- 2OP MATH OPERATIONS ----

            // 2OP:0x08 - or (bitwise OR)
            (0x08, crate::instruction::OperandCount::OP2) => {
                debug!("or {} {}", operands[0], operands[1]);
                if let Some(store_var) = inst.store_var {
                    self.vm
                        .write_variable(store_var, operands[0] | operands[1])?;
                }
                Ok(ExecutionResult::Continue)
            }

            // 2OP:0x09 - and (bitwise AND)
            (0x09, crate::instruction::OperandCount::OP2) => {
                debug!("and {} {}", operands[0], operands[1]);
                if let Some(store_var) = inst.store_var {
                    self.vm
                        .write_variable(store_var, operands[0] & operands[1])?;
                }
                Ok(ExecutionResult::Continue)
            }

            // 2OP:0x14 - add (signed addition)
            (0x14, crate::instruction::OperandCount::OP2) => {
                debug!("add {} {}", operands[0], operands[1]);
                if let Some(store_var) = inst.store_var {
                    let result = (operands[0] as i16).wrapping_add(operands[1] as i16) as u16;
                    self.vm.write_variable(store_var, result)?;
                }
                Ok(ExecutionResult::Continue)
            }

            // 2OP:0x15 - sub (signed subtraction)
            (0x15, crate::instruction::OperandCount::OP2) => {
                debug!("sub {} {}", operands[0], operands[1]);
                if let Some(store_var) = inst.store_var {
                    let result = (operands[0] as i16).wrapping_sub(operands[1] as i16) as u16;
                    self.vm.write_variable(store_var, result)?;
                }
                Ok(ExecutionResult::Continue)
            }

            // 2OP:0x16 - mul (signed multiplication)
            (0x16, crate::instruction::OperandCount::OP2) => {
                debug!("mul {} {}", operands[0], operands[1]);
                if let Some(store_var) = inst.store_var {
                    let result = (operands[0] as i16).wrapping_mul(operands[1] as i16) as u16;
                    self.vm.write_variable(store_var, result)?;
                }
                Ok(ExecutionResult::Continue)
            }

            // 2OP:0x17 - div (signed division)
            (0x17, crate::instruction::OperandCount::OP2) => {
                debug!("div {} {}", operands[0], operands[1]);
                if operands[1] == 0 {
                    return Err("Division by zero".to_string());
                }
                if let Some(store_var) = inst.store_var {
                    let result = (operands[0] as i16) / (operands[1] as i16);
                    self.vm.write_variable(store_var, result as u16)?;
                }
                Ok(ExecutionResult::Continue)
            }

            // 2OP:0x18 - mod (signed modulo)
            (0x18, crate::instruction::OperandCount::OP2) => {
                debug!("mod {} {}", operands[0], operands[1]);
                if operands[1] == 0 {
                    return Err("Modulo by zero".to_string());
                }
                if let Some(store_var) = inst.store_var {
                    let result = (operands[0] as i16) % (operands[1] as i16);
                    self.vm.write_variable(store_var, result as u16)?;
                }
                Ok(ExecutionResult::Continue)
            }

            // 2OP:0x1C - not (v1-v3 only, bitwise NOT)
            (0x1C, crate::instruction::OperandCount::OP2) => {
                if self.vm.game.header.version <= 3 {
                    debug!("not {} (v1-v3)", operands[0]);
                    if let Some(store_var) = inst.store_var {
                        let result = !operands[0]; // operands[1] is ignored
                        self.vm.write_variable(store_var, result)?;
                    }
                    Ok(ExecutionResult::Continue)
                } else {
                    Err("2OP:0x1C (not) is only valid in v1-v3".to_string())
                }
            }

            _ => Err(format!(
                "Unhandled math opcode: {:02x} with operand count {:?}",
                inst.opcode, inst.operand_count
            )),
        }
    }

    /// Check if an opcode is a mathematical operation
    pub fn is_math_opcode(opcode: u8, operand_count: &crate::instruction::OperandCount) -> bool {
        matches!(
            (opcode, operand_count),
            // 1OP math operations
            (0x0F, crate::instruction::OperandCount::OP1) |  // not (v1-v4)
            // 2OP math operations
            (0x08, crate::instruction::OperandCount::OP2) |  // or
            (0x09, crate::instruction::OperandCount::OP2) |  // and
            (0x14, crate::instruction::OperandCount::OP2) |  // add
            (0x15, crate::instruction::OperandCount::OP2) |  // sub
            (0x16, crate::instruction::OperandCount::OP2) |  // mul
            (0x17, crate::instruction::OperandCount::OP2) |  // div
            (0x18, crate::instruction::OperandCount::OP2) |  // mod
            (0x1C, crate::instruction::OperandCount::OP2) // not (v1-v3)
        )
    }
}
