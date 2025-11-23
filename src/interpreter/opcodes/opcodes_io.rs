/// Input/Output operations for Z-Machine interpreter
///
/// This module handles all I/O-related operations including:
/// - Text input operations (sread - line input with optional timer support)
/// - Character input operations (read_char - single character with optional timer)
/// - Stream management (input_stream - select input source, output_stream - redirect output)
///
/// These operations form the interactive layer of Z-Machine games,
/// handling all communication between the game and the player.
///
/// Key features:
/// - Version-aware input handling (V3 vs V4+ input systems)
/// - Timer interrupt support for timed input operations
/// - Stream redirection for advanced output control
/// - Proper Z-Machine specification compliance
use crate::interpreter::core::instruction::Instruction;
use crate::interpreter::{ExecutionResult, Interpreter};
use log::debug;

impl Interpreter {
    /// Handle I/O system opcodes
    pub fn execute_io_op(
        &mut self,
        inst: &Instruction,
        operands: &[u16],
    ) -> Result<ExecutionResult, String> {
        match (inst.opcode, &inst.operand_count) {
            // ---- VAR I/O OPERATIONS ----

            // VAR:0x04 - sread (text input with optional timer)
            (0x04, crate::interpreter::core::instruction::OperandCount::VAR) => {
                // For now, delegate back to the main interpreter until we can properly handle the complex input logic
                Err(
                    "I/O operations not fully implemented yet - delegating to main interpreter"
                        .to_string(),
                )
            }

            // VAR:0x13 - output_stream (when no store_var)
            (0x13, crate::interpreter::core::instruction::OperandCount::VAR) => {
                // This should only handle output_stream (when inst.store_var is None)
                // get_next_prop (when inst.store_var is Some) is handled by object module
                if inst.store_var.is_some() {
                    return Err("VAR:0x13 with store_var should be handled by object module (get_next_prop)".to_string());
                }

                // output_stream
                if !operands.is_empty() {
                    let stream_num = operands[0] as i16;
                    debug!("output_stream: stream_num={}", stream_num);

                    match stream_num {
                        1 => {
                            // Stream 1: screen output (always enabled)
                            debug!("output_stream: enabling screen output (always on)");
                        }
                        -1 => {
                            // Disable screen output (not typically implemented)
                            debug!("output_stream: disabling screen output (not implemented)");
                        }
                        3 => {
                            // Stream 3: table output (redirect to memory table)
                            if operands.len() >= 2 {
                                let table_addr = operands[1];
                                debug!(
                                    "output_stream: enabling stream 3, table at 0x{:04x}",
                                    table_addr
                                );
                                self.enable_stream3(table_addr as u32)?;
                            } else {
                                debug!("output_stream: stream 3 requested but no table address provided");
                            }
                        }
                        -3 => {
                            // Disable stream 3
                            debug!("output_stream: disabling stream 3");
                            self.disable_stream3()?;
                        }
                        _ => {
                            debug!("output_stream: unsupported stream {}", stream_num);
                        }
                    }
                }
                Ok(ExecutionResult::Continue)
            }

            // VAR:0x14 - input_stream
            (0x14, crate::interpreter::core::instruction::OperandCount::VAR) => {
                // input_stream (V3+)
                if !operands.is_empty() {
                    let stream_num = operands[0];
                    debug!("input_stream: stream_num={}", stream_num);

                    match stream_num {
                        0 => {
                            // Stream 0: keyboard input (default)
                            debug!("input_stream: selecting keyboard input (default)");
                            // This is the default - no action needed
                        }
                        1 => {
                            // Stream 1: file input (not typically implemented in basic interpreters)
                            debug!("input_stream: selecting file input (not implemented)");
                        }
                        _ => {
                            debug!("input_stream: unsupported stream {}", stream_num);
                        }
                    }
                } else {
                    debug!("input_stream: no stream number provided");
                }
                Ok(ExecutionResult::Continue)
            }

            // VAR:0x16 - read_char
            (0x16, crate::interpreter::core::instruction::OperandCount::VAR) => {
                // For now, delegate back to the main interpreter until we can properly handle the complex input logic
                Err(
                    "I/O operations not fully implemented yet - delegating to main interpreter"
                        .to_string(),
                )
            }

            _ => Err(format!(
                "Unhandled I/O opcode: {:02x} with operand count {:?}",
                inst.opcode, inst.operand_count
            )),
        }
    }

    /// Check if an opcode is an I/O operation
    pub fn is_io_opcode(
        opcode: u8,
        operand_count: &crate::interpreter::core::instruction::OperandCount,
    ) -> bool {
        matches!(
            (opcode, operand_count),
            // VAR I/O operations
            (0x04, crate::interpreter::core::instruction::OperandCount::VAR) |  // sread
            (0x14, crate::interpreter::core::instruction::OperandCount::VAR) |  // input_stream
            (0x16, crate::interpreter::core::instruction::OperandCount::VAR) // read_char
                                                                             // Note: VAR:0x13 (output_stream) is handled specially via disambiguation
        )
    }

    /// Check if a VAR:0x13 opcode should be routed to the I/O module
    /// This handles the get_next_prop vs output_stream disambiguation
    pub fn is_var_13_io_opcode(inst: &crate::interpreter::core::instruction::Instruction) -> bool {
        inst.opcode == 0x13
            && inst.operand_count == crate::interpreter::core::instruction::OperandCount::VAR
            && inst.store_var.is_none()
    }
}
