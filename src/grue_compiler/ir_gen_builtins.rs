// IR Generator - Builtin Function Handling
//
// Extracted from ir_generator.rs as part of modularization effort.
// Handles recognition and code generation for builtin functions.

use crate::grue_compiler::error::CompilerError;

use super::{IrBlock, IrGenerator, IrId, IrInstruction};

#[cfg(debug_assertions)]
use super::IrValue;

impl IrGenerator {
    /// Check if a function name is a builtin function
    ///
    /// Builtins include:
    /// - Output: print, print_num, println, print_ret, print_message, new_line
    /// - Object system: move, get_location, get_child, get_sibling, get_prop, test_attr, set_attr, clear_attr
    /// - Scoring: add_score, subtract_score
    /// - Utilities: word_to_number, to_string, random, quit
    /// - String utilities: indexOf, slice, substring, toLowerCase, toUpperCase, trim, charAt, split, replace, startsWith, endsWith
    /// - Math utilities: abs, min, max, round, floor, ceil
    /// - Type checking: is_string, is_int, is_bool, is_array, is_object, typeof
    /// - Debug: debug_break (debug builds only)
    pub(super) fn is_builtin_function(&self, name: &str) -> bool {
        #[cfg(debug_assertions)]
        {
            matches!(
                name,
                "print"
                    | "print_num"
                    | "println"
                    | "print_ret"
                    | "print_message"
                    | "new_line"
                    | "move"
                    | "add_score"
                    | "subtract_score"
                    | "word_to_number"
                    | "get_location"
                    | "get_child"
                    | "get_sibling"
                    | "get_prop"
                    | "test_attr"
                    | "set_attr"
                    | "clear_attr"
                    | "to_string"
                    | "random"
                    // String utility functions
                    | "indexOf"
                    | "slice"
                    | "substring"
                    | "toLowerCase"
                    | "toUpperCase"
                    | "trim"
                    | "charAt"
                    | "split"
                    | "replace"
                    | "startsWith"
                    | "endsWith"
                    // Math utility functions
                    | "abs"
                    | "min"
                    | "max"
                    | "round"
                    | "floor"
                    | "ceil"
                    // Type checking functions
                    | "is_string"
                    | "is_int"
                    | "is_bool"
                    | "is_array"
                    | "is_object"
                    | "typeof"
                    // Game control
                    | "quit"
                    // Debug breakpoints (debug builds only)
                    | "debug_break"
            )
        }
        #[cfg(not(debug_assertions))]
        {
            matches!(
                name,
                "print"
                    | "print_num"
                    | "println"
                    | "print_ret"
                    | "print_message"
                    | "new_line"
                    | "move"
                    | "add_score"
                    | "subtract_score"
                    | "word_to_number"
                    | "get_location"
                    | "get_child"
                    | "get_sibling"
                    | "get_prop"
                    | "test_attr"
                    | "set_attr"
                    | "clear_attr"
                    | "to_string"
                    | "random"
                    // String utility functions
                    | "indexOf"
                    | "slice"
                    | "substring"
                    | "toLowerCase"
                    | "toUpperCase"
                    | "trim"
                    | "charAt"
                    | "split"
                    | "replace"
                    | "startsWith"
                    | "endsWith"
                    // Math utility functions
                    | "abs"
                    | "min"
                    | "max"
                    | "round"
                    | "floor"
                    | "ceil"
                    // Type checking functions
                    | "is_string"
                    | "is_int"
                    | "is_bool"
                    | "is_array"
                    | "is_object"
                    | "typeof"
                    // Game control
                    | "quit"
            )
        }
    }

    pub(super) fn generate_builtin_function_call(
        &mut self,
        name: &str,
        arg_temps: &[IrId],
        block: &mut IrBlock,
    ) -> Result<IrId, CompilerError> {
        let temp_id = self.next_id();

        match name {
            // String utility functions
            "indexOf" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "indexOf expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringIndexOf {
                    target: temp_id,
                    string: arg_temps[0],
                    substring: arg_temps[1],
                });
            }
            "slice" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "slice expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringSlice {
                    target: temp_id,
                    string: arg_temps[0],
                    start: arg_temps[1],
                });
            }
            "substring" => {
                if arg_temps.len() != 3 {
                    return Err(CompilerError::CodeGenError(
                        "substring expects 3 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringSubstring {
                    target: temp_id,
                    string: arg_temps[0],
                    start: arg_temps[1],
                    end: arg_temps[2],
                });
            }
            "toLowerCase" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "toLowerCase expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringToLowerCase {
                    target: temp_id,
                    string: arg_temps[0],
                });
            }
            "toUpperCase" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "toUpperCase expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringToUpperCase {
                    target: temp_id,
                    string: arg_temps[0],
                });
            }
            "trim" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "trim expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringTrim {
                    target: temp_id,
                    string: arg_temps[0],
                });
            }
            "charAt" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "charAt expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringCharAt {
                    target: temp_id,
                    string: arg_temps[0],
                    index: arg_temps[1],
                });
            }
            "split" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "split expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringSplit {
                    target: temp_id,
                    string: arg_temps[0],
                    delimiter: arg_temps[1],
                });
            }
            "replace" => {
                if arg_temps.len() != 3 {
                    return Err(CompilerError::CodeGenError(
                        "replace expects 3 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringReplace {
                    target: temp_id,
                    string: arg_temps[0],
                    search: arg_temps[1],
                    replacement: arg_temps[2],
                });
            }
            "startsWith" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "startsWith expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringStartsWith {
                    target: temp_id,
                    string: arg_temps[0],
                    prefix: arg_temps[1],
                });
            }
            "endsWith" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "endsWith expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::StringEndsWith {
                    target: temp_id,
                    string: arg_temps[0],
                    suffix: arg_temps[1],
                });
            }
            // Math utility functions
            "abs" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "abs expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathAbs {
                    target: temp_id,
                    value: arg_temps[0],
                });
            }
            "min" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "min expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathMin {
                    target: temp_id,
                    a: arg_temps[0],
                    b: arg_temps[1],
                });
            }
            "max" => {
                if arg_temps.len() != 2 {
                    return Err(CompilerError::CodeGenError(
                        "max expects 2 arguments".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathMax {
                    target: temp_id,
                    a: arg_temps[0],
                    b: arg_temps[1],
                });
            }
            "round" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "round expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathRound {
                    target: temp_id,
                    value: arg_temps[0],
                });
            }
            "floor" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "floor expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathFloor {
                    target: temp_id,
                    value: arg_temps[0],
                });
            }
            "ceil" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "ceil expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::MathCeil {
                    target: temp_id,
                    value: arg_temps[0],
                });
            }
            // Type checking functions
            "is_string" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "is_string expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeCheck {
                    target: temp_id,
                    value: arg_temps[0],
                    type_name: "string".to_string(),
                });
            }
            "is_int" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "is_int expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeCheck {
                    target: temp_id,
                    value: arg_temps[0],
                    type_name: "int".to_string(),
                });
            }
            "is_bool" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "is_bool expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeCheck {
                    target: temp_id,
                    value: arg_temps[0],
                    type_name: "bool".to_string(),
                });
            }
            "is_array" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "is_array expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeCheck {
                    target: temp_id,
                    value: arg_temps[0],
                    type_name: "array".to_string(),
                });
            }
            "is_object" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "is_object expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeCheck {
                    target: temp_id,
                    value: arg_temps[0],
                    type_name: "object".to_string(),
                });
            }
            "typeof" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "typeof expects 1 argument".to_string(),
                    ));
                }
                block.add_instruction(IrInstruction::TypeOf {
                    target: temp_id,
                    value: arg_temps[0],
                });
            }
            // Debug breakpoint (debug builds only)
            #[cfg(debug_assertions)]
            "debug_break" => {
                if arg_temps.len() != 1 {
                    return Err(CompilerError::CodeGenError(
                        "debug_break expects 1 argument (label string)".to_string(),
                    ));
                }
                // Extract label from the LoadImmediate instruction
                // We need to look back at the IR to find the string value
                // For now, use a placeholder - we'll need to track this properly
                block.add_instruction(IrInstruction::DebugBreak {
                    label: format!("breakpoint_{}", temp_id),
                });
                // Return a dummy value (0) since debug_break doesn't produce a useful result
                block.add_instruction(IrInstruction::LoadImmediate {
                    target: temp_id,
                    value: IrValue::Integer(0),
                });
            }
            // Score management functions - Now implemented as real Z-Machine functions
            // ARCHITECTURE FIX (Nov 4, 2025): Converted from inline generation to real builtin functions
            // per CLAUDE.md directive: "ALL builtin functions MUST be implemented as real Z-Machine functions"
            "add_score" | "subtract_score" | "word_to_number" => {
                // Use standard builtin function call mechanism like get_exit
                // This eliminates opcode 0x15 errors by using proper function calls
                // instead of inline IR generation that violates Z-Machine V3 constraints

                // Look up function ID (or create placeholder)
                let func_id = if let Some(&id) = self.symbol_ids.get(name) {
                    id
                } else {
                    let placeholder_id = self.next_id();
                    self.symbol_ids.insert(name.to_string(), placeholder_id);
                    self.builtin_functions
                        .insert(placeholder_id, name.to_string());
                    placeholder_id
                };

                // Generate function call instruction
                block.add_instruction(IrInstruction::Call {
                    target: Some(temp_id),
                    function: func_id,
                    args: arg_temps.to_vec(),
                });
            }
            // For other builtin functions, use standard call mechanism
            _ => {
                // Look up function ID (or create placeholder)
                let func_id = if let Some(&id) = self.symbol_ids.get(name) {
                    id
                } else {
                    let placeholder_id = self.next_id();
                    self.symbol_ids.insert(name.to_string(), placeholder_id);
                    self.builtin_functions
                        .insert(placeholder_id, name.to_string());
                    placeholder_id
                };

                block.add_instruction(IrInstruction::Call {
                    target: Some(temp_id),
                    function: func_id,
                    args: arg_temps.to_vec(),
                });
            }
        }

        Ok(temp_id)
    }
}
