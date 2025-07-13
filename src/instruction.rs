#![allow(unused_imports)]
use crate::opcode_tables;
use crate::text;
use log::debug;
use std::fmt::{Debug, Display, Error, Formatter, Write};

/// Operand types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperandType {
    /// Large constant (2 bytes)
    LargeConstant,
    /// Small constant (1 byte)
    SmallConstant,
    /// Variable number
    Variable,
    /// Omitted (not present)
    Omitted,
}

impl OperandType {
    /// Parse operand type from 2-bit value
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0x03 {
            0b00 => OperandType::LargeConstant,
            0b01 => OperandType::SmallConstant,
            0b10 => OperandType::Variable,
            0b11 => OperandType::Omitted,
            _ => unreachable!(),
        }
    }

    /// Get the size in bytes for this operand type
    pub fn size(&self) -> usize {
        match self {
            OperandType::LargeConstant => 2,
            OperandType::SmallConstant => 1,
            OperandType::Variable => 1,
            OperandType::Omitted => 0,
        }
    }
}

/// Instruction forms
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InstructionForm {
    Long,
    Short,
    Extended,
    Variable,
}

/// Operand count categories
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperandCount {
    /// 0 operands
    OP0,
    /// 1 operand
    OP1,
    /// 2 operands
    OP2,
    /// Variable number of operands (0-8)
    VAR,
}

/// Branch information
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// True if branch on true, false if branch on false
    pub on_true: bool,
    /// Branch offset (0-1 = return true/false, 2+ = jump)
    pub offset: i16,
}

/// A decoded Z-Machine instruction
#[derive(Debug, Clone)]
pub struct Instruction {
    /// The raw opcode value
    pub opcode: u8,
    /// Extended opcode for EXT instructions
    pub ext_opcode: Option<u8>,
    /// The instruction form
    pub form: InstructionForm,
    /// Operand count category
    pub operand_count: OperandCount,
    /// Operand types (up to 8)
    pub operand_types: Vec<OperandType>,
    /// Operand values (up to 8)
    pub operands: Vec<u16>,
    /// Variable to store result (if applicable)
    pub store_var: Option<u8>,
    /// Branch information (if applicable)
    pub branch: Option<BranchInfo>,
    /// String data for print opcodes
    pub text: Option<String>,
    /// Total size of instruction in bytes
    pub size: usize,
}

impl Instruction {
    /// Decode an instruction from memory at the given address
    pub fn decode(memory: &[u8], addr: usize, version: u8) -> Result<Self, String> {
        if addr >= memory.len() {
            return Err(format!("Instruction address {} out of bounds", addr));
        }

        let mut offset = addr;
        let opcode_byte = memory[offset];
        offset += 1;

        // Debug output for problematic instructions
        if addr == 0x06f91
            || addr == 0x06f8c
            || addr == 0x06f8f
            || addr == 0x08cb0
            || addr == 0x08cb4
            || addr == 0x08cbc
            || (opcode_byte >> 6 < 2 && (opcode_byte & 0x1F) == 0)
        {
            debug!("=== INSTRUCTION DEBUG at {:05x} ===", addr);
            debug!(
                "  Opcode byte: {:02x} (binary: {:08b})",
                opcode_byte, opcode_byte
            );
            debug!("  Top 2 bits: {:02b}", opcode_byte >> 6);
            debug!("  Bottom 5 bits: {:02x}", opcode_byte & 0x1F);
            // Show next few bytes for context
            if addr + 4 < memory.len() {
                debug!(
                    "  Next bytes: {:02x} {:02x} {:02x} {:02x}",
                    memory[addr + 1],
                    memory[addr + 2],
                    memory[addr + 3],
                    memory[addr + 4]
                );
            }
        }

        // Determine instruction form based on top 2 bits
        let form = match opcode_byte >> 6 {
            0b11 => InstructionForm::Variable,
            0b10 => InstructionForm::Short,
            _ => {
                // Check for extended form (0xBE in V5+)
                if opcode_byte == 0xBE && version >= 5 {
                    InstructionForm::Extended
                } else {
                    InstructionForm::Long
                }
            }
        };

        // Get the actual opcode and operand count
        let (opcode, ext_opcode, operand_count) = match form {
            InstructionForm::Long => {
                // Long form: 2OP, opcode in bottom 5 bits
                (opcode_byte & 0x1F, None, OperandCount::OP2)
            }
            InstructionForm::Short => {
                // Short form: opcode in bottom 4 bits
                let op_count = if (opcode_byte >> 4) & 0x03 == 0x03 {
                    OperandCount::OP0
                } else {
                    OperandCount::OP1
                };
                (opcode_byte & 0x0F, None, op_count)
            }
            InstructionForm::Variable => {
                // Variable form: opcode in bottom 5 bits
                let op_count = if opcode_byte & 0x20 == 0 {
                    OperandCount::OP2
                } else {
                    OperandCount::VAR
                };
                (opcode_byte & 0x1F, None, op_count)
            }
            InstructionForm::Extended => {
                // Extended form: next byte is the actual opcode
                if offset >= memory.len() {
                    return Err("Extended opcode out of bounds".to_string());
                }
                let ext_op = memory[offset];
                offset += 1;
                (opcode_byte, Some(ext_op), OperandCount::VAR)
            }
        };

        // Decode operand types
        let mut operand_types = Vec::new();

        match form {
            InstructionForm::Long => {
                // Long form: 2 operands, types in bits 6 and 5
                let type1 = if opcode_byte & 0x40 != 0 {
                    OperandType::Variable
                } else {
                    OperandType::SmallConstant
                };
                let type2 = if opcode_byte & 0x20 != 0 {
                    OperandType::Variable
                } else {
                    OperandType::SmallConstant
                };
                
                
                operand_types.push(type1);
                operand_types.push(type2);
            }
            InstructionForm::Short => {
                // Short form: 0 or 1 operand, type in bits 5-4
                if operand_count != OperandCount::OP0 {
                    let op_type = OperandType::from_bits((opcode_byte >> 4) & 0x03);
                    if op_type != OperandType::Omitted {
                        operand_types.push(op_type);
                    }
                }
            }
            InstructionForm::Variable | InstructionForm::Extended => {
                // Variable/Extended form: operand types follow
                if offset >= memory.len() {
                    return Err("Operand types out of bounds".to_string());
                }

                // Read operand type bytes
                let mut type_bytes = vec![memory[offset]];
                offset += 1;

                // Check if we need more type bytes (for >4 operands)
                if operand_count == OperandCount::VAR && type_bytes[0] == 0xFF {
                    if offset >= memory.len() {
                        return Err("Extended operand types out of bounds".to_string());
                    }
                    type_bytes.push(memory[offset]);
                    offset += 1;
                }

                // Parse operand types from type bytes
                for type_byte in type_bytes {
                    for i in 0..4 {
                        let op_type = OperandType::from_bits(type_byte >> (6 - i * 2));
                        if op_type == OperandType::Omitted {
                            break;
                        }
                        operand_types.push(op_type);
                    }
                }
            }
        };

        // Check if we need to limit the number of operands for this instruction
        let expected_count = crate::opcode_tables::get_expected_operand_count(
            opcode,
            ext_opcode,
            form,
            operand_count,
            version,
        );

        let operand_limit = if let Some(count) = expected_count {
            operand_types.len().min(count)
        } else {
            operand_types.len()
        };

        // Read operand values
        let mut operands = Vec::new();
        for (i, op_type) in operand_types.iter().enumerate() {
            // Stop if we've reached the expected operand count
            if i >= operand_limit {
                break;
            }

            match op_type {
                OperandType::LargeConstant => {
                    if offset + 1 >= memory.len() {
                        return Err("Large constant out of bounds".to_string());
                    }
                    let value = ((memory[offset] as u16) << 8) | (memory[offset + 1] as u16);
                    operands.push(value);
                    offset += 2;
                }
                OperandType::SmallConstant | OperandType::Variable => {
                    if offset >= memory.len() {
                        return Err("Small constant/variable out of bounds".to_string());
                    }
                    operands.push(memory[offset] as u16);
                    offset += 1;
                }
                OperandType::Omitted => break,
            }
        }

        // Check if instruction stores a result
        let store_var = if Self::stores_result(opcode, ext_opcode, form, operand_count, version) {
            if offset >= memory.len() {
                return Err("Store variable out of bounds".to_string());
            }
            let var = memory[offset];
            offset += 1;
            Some(var)
        } else {
            None
        };

        // Check if instruction has a branch
        let branch = if Self::has_branch(opcode, ext_opcode, form, operand_count, version) {
            if offset >= memory.len() {
                return Err("Branch offset out of bounds".to_string());
            }

            let first_byte = memory[offset];
            offset += 1;

            let on_true = (first_byte & 0x80) != 0;
            let offset_val = if (first_byte & 0x40) != 0 {
                // Short form: 6-bit signed offset  
                let val = (first_byte & 0x3F) as i16;
                // For 6-bit signed: bit 5 is sign bit, range is -32 to +31
                if val > 31 {
                    // Convert from 6-bit two's complement to 16-bit
                    val - 64
                } else {
                    val
                }
            } else {
                // Long form: 14-bit signed offset
                if offset >= memory.len() {
                    return Err("Branch offset second byte out of bounds".to_string());
                }
                let second_byte = memory[offset];
                offset += 1;

                let val = (((first_byte & 0x3F) as i16) << 8) | (second_byte as i16);
                if val & 0x2000 != 0 {
                    // Sign extend
                    val | (0xC000u16 as i16)
                } else {
                    val
                }
            };

            Some(BranchInfo {
                on_true,
                offset: offset_val,
            })
        } else {
            None
        };

        // Check for text (print opcodes)
        let (text, _text_len) = if Self::has_text(opcode, ext_opcode, form, operand_count, version)
        {
            // Decode the inline text string
            // Extract abbreviation table address from header at offset 0x18
            let abbrev_addr = if memory.len() >= 0x1a {
                ((memory[0x18] as u16) << 8 | memory[0x19] as u16) as usize
            } else {
                0x40 // Fallback
            };
            match text::decode_string(memory, offset, abbrev_addr) {
                Ok((string, len)) => {
                    offset += len;
                    (Some(string), len)
                }
                Err(e) => {
                    return Err(format!("Failed to decode inline text: {}", e));
                }
            }
        } else {
            (None, 0)
        };

        let size = offset - addr;

        // Truncate operand_types to match the actual operands read
        let mut actual_operand_types = operand_types;
        actual_operand_types.truncate(operands.len());

        Ok(Instruction {
            opcode,
            ext_opcode,
            form,
            operand_count,
            operand_types: actual_operand_types,
            operands,
            store_var,
            branch,
            text,
            size,
        })
    }

    /// Check if an instruction stores a result
    fn stores_result(
        opcode: u8,
        ext_opcode: Option<u8>,
        form: InstructionForm,
        operand_count: OperandCount,
        version: u8,
    ) -> bool {
        crate::opcode_tables::stores_result(opcode, ext_opcode, form, operand_count, version)
    }

    /// Check if an instruction has a branch
    fn has_branch(
        opcode: u8,
        ext_opcode: Option<u8>,
        form: InstructionForm,
        operand_count: OperandCount,
        version: u8,
    ) -> bool {
        crate::opcode_tables::has_branch(opcode, ext_opcode, form, operand_count, version)
    }

    /// Check if an instruction has inline text
    fn has_text(
        opcode: u8,
        ext_opcode: Option<u8>,
        form: InstructionForm,
        operand_count: OperandCount,
        version: u8,
    ) -> bool {
        crate::opcode_tables::has_text(opcode, ext_opcode, form, operand_count, version)
    }

    /// Get a human-readable name for the instruction
    pub fn name(&self, version: u8) -> &'static str {
        crate::opcode_tables::get_instruction_name(
            self.opcode,
            self.ext_opcode,
            self.form,
            self.operand_count,
            version,
        )
    }

    /// Format the instruction with proper version information
    pub fn format_with_version(&self, version: u8) -> String {
        let mut result = String::from(self.name(version));

        // Print operands
        for (i, op) in self.operands.iter().enumerate() {
            if i == 0 {
                result.push(' ');
            } else {
                result.push_str(", ");
            }

            match self.operand_types[i] {
                OperandType::Variable => write!(result, "V{:02x}", op).unwrap(),
                _ => write!(result, "#{:04x}", op).unwrap(),
            }
        }

        // Print store variable
        if let Some(var) = self.store_var {
            write!(result, " -> V{:02x}", var).unwrap();
        }

        // Print branch info
        if let Some(ref branch) = self.branch {
            write!(
                result,
                " [{}{}]",
                if branch.on_true { "TRUE" } else { "FALSE" },
                match branch.offset {
                    0 => " RFALSE".to_string(),
                    1 => " RTRUE".to_string(),
                    n => format!(" {:+}", n),
                }
            )
            .unwrap();
        }

        result
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        // We can't know the version in Display, so we'll use a default
        // For proper display, use a method that takes version as parameter
        write!(f, "{}", self.name(3))?;

        // Print operands
        for (i, op) in self.operands.iter().enumerate() {
            if i == 0 {
                write!(f, " ")?;
            } else {
                write!(f, ", ")?;
            }

            match self.operand_types[i] {
                OperandType::Variable => write!(f, "V{:02x}", op)?,
                _ => write!(f, "#{:04x}", op)?,
            }
        }

        // Print store variable
        if let Some(var) = self.store_var {
            write!(f, " -> V{:02x}", var)?;
        }

        // Print branch info
        if let Some(ref branch) = self.branch {
            write!(
                f,
                " [{}{}]",
                if branch.on_true { "TRUE" } else { "FALSE" },
                match branch.offset {
                    0 => " RFALSE".to_string(),
                    1 => " RTRUE".to_string(),
                    n => format!(" {:+}", n),
                }
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operand_type_from_bits() {
        assert_eq!(OperandType::from_bits(0b00), OperandType::LargeConstant);
        assert_eq!(OperandType::from_bits(0b01), OperandType::SmallConstant);
        assert_eq!(OperandType::from_bits(0b10), OperandType::Variable);
        assert_eq!(OperandType::from_bits(0b11), OperandType::Omitted);
    }

    #[test]
    fn test_decode_long_form() {
        // Test a simple "je" instruction (opcode 0x01) in long form
        // je #1234 #5678
        let memory = vec![
            0x41, // Long form, 2OP, opcode 1 (je), both small constants
            0x34, // First operand (small constant)
            0x78, // Second operand (small constant)
            0x80, // Branch: on true, offset 0 (return false)
            0x00, 0x00, // Padding
        ];

        let inst = Instruction::decode(&memory, 0, 3).unwrap();
        assert_eq!(inst.form, InstructionForm::Long);
        assert_eq!(inst.opcode, 0x01);
        assert_eq!(inst.operands.len(), 2);
        assert_eq!(inst.operands[0], 0x34);
        assert_eq!(inst.operands[1], 0x78);
        assert!(inst.branch.is_some());
    }

    #[test]
    fn test_decode_short_form() {
        // Test "jump" instruction (opcode 0x0C) in short form
        // jump #34
        let memory = vec![
            0x9C, // Short form, 1OP, opcode 12 (jump), small constant (bits 5-4 = 01)
            0x34, // Operand
            0x00, 0x00, // Padding
        ];

        let inst = Instruction::decode(&memory, 0, 3).unwrap();
        assert_eq!(inst.form, InstructionForm::Short);
        assert_eq!(inst.opcode, 0x0C);
        assert_eq!(inst.operands.len(), 1);
        assert_eq!(inst.operands[0], 0x34);
    }

    #[test]
    fn test_decode_variable_form() {
        // Test "call" instruction in variable form
        let memory = vec![
            0xE0, // Variable form, VAR opcode 0 (call)
            0x2A, // Operand types: large, variable, variable, variable
            0x12, 0x34, // First operand (large constant)
            0x01, // Second operand (variable)
            0x02, // Third operand (variable)
            0x03, // Fourth operand (variable)
            0x00, // Store variable
            0x00, 0x00, // Padding to ensure we don't run out of memory
        ];

        let inst = Instruction::decode(&memory, 0, 3).unwrap();
        assert_eq!(inst.form, InstructionForm::Variable);
        assert_eq!(inst.operands.len(), 4);
        assert_eq!(inst.operands[0], 0x1234);
        assert_eq!(inst.operands[1], 0x01);
        assert_eq!(inst.operands[2], 0x02);
        assert_eq!(inst.operands[3], 0x03);
        assert_eq!(inst.store_var, Some(0x00));
    }
}
