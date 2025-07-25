/// Memory operations for Z-Machine interpreter
/// 
/// This module handles all memory access operations including:
/// - Variable operations (load, store)
/// - Word operations (loadw, storew) - 16-bit values at word boundaries
/// - Byte operations (loadb, storeb) - 8-bit values at byte addresses
/// 
/// These operations form the foundation of Z-Machine memory access,
/// enabling everything from variable manipulation to dynamic memory access.

use crate::instruction::Instruction;
use crate::interpreter::{ExecutionResult, Interpreter};
use log::debug;

impl Interpreter {
    /// Handle memory access opcodes
    pub fn execute_memory_op(&mut self, inst: &Instruction, operands: &[u16]) -> Result<ExecutionResult, String> {
        match (inst.opcode, &inst.operand_count) {
            // ---- 1OP MEMORY OPERATIONS ----
            
            // 1OP:0x0E - load (load from variable)
            (0x0E, crate::instruction::OperandCount::OP1) => {
                // load - operand can be any type, value specifies which variable to load
                let var_num = operands[0] as u8;
                let value = self.vm.read_variable(var_num)?;
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, value)?;
                }
                Ok(ExecutionResult::Continue)
            }

            // ---- 2OP MEMORY OPERATIONS ----

            // 2OP:0x01 - je (already handled in main interpreter - not memory)
            // Note: This slot is used by je (jump if equal) which is not a memory operation

            // 2OP:0x0D - store (store to variable)
            (0x0D, crate::instruction::OperandCount::OP2) => {
                // store
                // Use raw operand for variable number (destination)
                let var_num = inst.operands[0] as u8;
                let value = operands[1];
                let current_pc = self.vm.pc - inst.size as u32;

                if var_num == 0x10 {
                    debug!(
                        "Setting location (global 0) to object {} at PC {:05x}",
                        value, current_pc
                    );
                    if value == 180 {
                        debug!("*** LOCATION SET TO 180 (MAZE 9) ***");
                        // This is important for Zork I navigation
                    }
                }

                debug!("store: var_num={:02x}, value={}", var_num, value);
                self.vm.write_variable(var_num, value)?;
                Ok(ExecutionResult::Continue)
            }

            // 2OP:0x0F - loadw (load word from memory)
            (0x0F, crate::instruction::OperandCount::OP2) => {
                // loadw
                let addr = operands[0] as u32 + (operands[1] as u32 * 2);
                let value = self.vm.read_word(addr);
                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, value)?;
                }
                Ok(ExecutionResult::Continue)
            }

            // 2OP:0x10 - loadb (load byte from memory)
            (0x10, crate::instruction::OperandCount::OP2) => {
                // loadb
                let addr = operands[0] as u32 + operands[1] as u32;
                let value = self.vm.read_byte(addr) as u16;

                // Debug the leaves issue
                let pc = self.vm.pc - inst.size as u32;
                if pc == 0x6345 || pc == 0x6349 {
                    debug!(
                        "loadb at 0x{:04x}: base=0x{:04x}, offset={}, addr=0x{:04x}, value={}",
                        pc, operands[0], operands[1], addr, value
                    );
                    // Also show what V01 points to
                    if operands[0] == 1 {
                        // If using V01
                        if let Ok(v01) = self.vm.read_variable(1) {
                            debug!("  V01 = 0x{:04x}", v01);
                            // Show parse buffer entry
                            for i in 0..4 {
                                let byte = self.vm.read_byte(v01 as u32 + i);
                                debug!("    V01[{}] = {}", i, byte);
                            }
                        }
                    }
                }

                if let Some(store_var) = inst.store_var {
                    self.vm.write_variable(store_var, value)?;
                }
                Ok(ExecutionResult::Continue)
            }

            // ---- VAR MEMORY OPERATIONS ----

            // VAR:0x01 - storew (store word to memory)
            (0x01, crate::instruction::OperandCount::VAR) => {
                // storew
                if operands.len() < 3 {
                    // For Variable form with OP2, this might be 2OP:21 (storew) not VAR:01
                    if inst.form == crate::instruction::InstructionForm::Variable
                        && inst.operand_count == crate::instruction::OperandCount::OP2
                    {
                        // This is actually 2OP:21 (storew) in Variable form
                        debug!("Note: Variable form storew with OP2 at PC {:05x} - this is 2OP:21 in Variable form", 
                               self.vm.pc - inst.size as u32);
                    }
                    return Err(format!(
                        "storew at PC {:05x} requires 3 operands, got {} (operands: {:?}) - instruction form: {:?}, opcode: {:02x}, operand_count: {:?}",
                        self.vm.pc - inst.size as u32, operands.len(), operands, inst.form, inst.opcode, inst.operand_count
                    ));
                }
                let addr = operands[0] as u32 + (operands[1] as u32 * 2);
                self.vm.write_word(addr, operands[2])?;
                Ok(ExecutionResult::Continue)
            }

            // VAR:0x02 - storeb (store byte to memory)
            (0x02, crate::instruction::OperandCount::VAR) => {
                // storeb
                if operands.len() < 3 {
                    return Err("storeb requires 3 operands".to_string());
                }
                let addr = operands[0] as u32 + operands[1] as u32;
                self.vm.write_byte(addr, operands[2] as u8)?;
                Ok(ExecutionResult::Continue)
            }

            _ => Err(format!("Unhandled memory opcode: {:02x} with operand count {:?}", 
                           inst.opcode, inst.operand_count))
        }
    }

    /// Check if an opcode is a memory operation
    pub fn is_memory_opcode(opcode: u8, operand_count: &crate::instruction::OperandCount) -> bool {
        matches!(
            (opcode, operand_count),
            // 1OP memory operations
            (0x0E, crate::instruction::OperandCount::OP1) |  // load
            // 2OP memory operations  
            (0x0D, crate::instruction::OperandCount::OP2) |  // store
            (0x0F, crate::instruction::OperandCount::OP2) |  // loadw
            (0x10, crate::instruction::OperandCount::OP2) |  // loadb
            // VAR memory operations
            (0x01, crate::instruction::OperandCount::VAR) |  // storew
            (0x02, crate::instruction::OperandCount::VAR)    // storeb
        )
    }
}