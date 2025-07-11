#![allow(unused_imports)]
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;

use crate::util::get_mem_addr;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InstructionForm {
    Long,
    Short,
    Variable,
    Extended,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperandType {
    LargeConstant,   // 00: 2-byte constant
    SmallConstant,   // 01: 1-byte constant  
    Variable,        // 10: variable reference
    Omitted,         // 11: operand omitted
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperandCount {
    Op0,  // 0OP
    Op1,  // 1OP
    Op2,  // 2OP
    Var,  // VAR
}

#[derive(Debug, Clone)]
pub struct Operand {
    pub operand_type: OperandType,
    pub value: u16,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: u8,
    pub form: InstructionForm,
    pub operand_count: OperandCount,
    pub operands: Vec<Operand>,
    pub store_variable: Option<u8>,
    pub branch_offset: Option<i16>,
    pub text: Option<String>,
    pub length: usize,  // Total instruction length in bytes
}

impl Instruction {
    pub fn decode(memory: &[u8], pc: usize) -> Result<Instruction, String> {
        if pc >= memory.len() {
            return Err("PC out of bounds".to_string());
        }

        let opcode_byte = memory[pc];
        let mut cursor = pc + 1;

        // Determine instruction form
        let form = match opcode_byte & 0xC0 {
            0xC0 => InstructionForm::Variable,  // 11xxxxxx
            0x80 => InstructionForm::Short,     // 10xxxxxx
            _ => {
                if opcode_byte == 0xBE {
                    InstructionForm::Extended
                } else {
                    InstructionForm::Long           // 0xxxxxxx or 01xxxxxx
                }
            }
        };

        let (opcode, operand_count, operand_types) = match form {
            InstructionForm::Long => {
                let opcode = opcode_byte & 0x1F;
                let operand_count = OperandCount::Op2;
                let operand_types = vec![
                    if opcode_byte & 0x40 != 0 { OperandType::Variable } else { OperandType::SmallConstant },
                    if opcode_byte & 0x20 != 0 { OperandType::Variable } else { OperandType::SmallConstant },
                ];
                (opcode, operand_count, operand_types)
            },
            InstructionForm::Short => {
                let opcode = opcode_byte & 0x0F;
                let operand_type = match (opcode_byte & 0x30) >> 4 {
                    0b00 => OperandType::LargeConstant,
                    0b01 => OperandType::SmallConstant,
                    0b10 => OperandType::Variable,
                    0b11 => OperandType::Omitted,
                    _ => unreachable!(),
                };
                let operand_count = if operand_type == OperandType::Omitted { 
                    OperandCount::Op0 
                } else { 
                    OperandCount::Op1 
                };
                (opcode, operand_count, vec![operand_type])
            },
            InstructionForm::Variable => {
                let opcode = opcode_byte & 0x1F;
                let operand_count = if opcode_byte & 0x20 != 0 { OperandCount::Var } else { OperandCount::Op2 };
                
                // Read operand types from next byte
                if cursor >= memory.len() {
                    return Err("Missing operand types byte".to_string());
                }
                let types_byte = memory[cursor];
                cursor += 1;

                let operand_types = vec![
                    Self::decode_operand_type((types_byte & 0xC0) >> 6),
                    Self::decode_operand_type((types_byte & 0x30) >> 4),
                    Self::decode_operand_type((types_byte & 0x0C) >> 2),
                    Self::decode_operand_type(types_byte & 0x03),
                ];

                (opcode, operand_count, operand_types)
            },
            InstructionForm::Extended => {
                if cursor >= memory.len() {
                    return Err("Missing extended opcode".to_string());
                }
                let opcode = memory[cursor];
                cursor += 1;

                // Read operand types from next byte
                if cursor >= memory.len() {
                    return Err("Missing operand types byte".to_string());
                }
                let types_byte = memory[cursor];
                cursor += 1;

                let operand_types = vec![
                    Self::decode_operand_type((types_byte & 0xC0) >> 6),
                    Self::decode_operand_type((types_byte & 0x30) >> 4),
                    Self::decode_operand_type((types_byte & 0x0C) >> 2),
                    Self::decode_operand_type(types_byte & 0x03),
                ];

                (opcode, OperandCount::Var, operand_types)
            }
        };

        // Read operands
        let mut operands = Vec::new();
        for &operand_type in &operand_types {
            if operand_type == OperandType::Omitted {
                break;
            }

            let operand = match operand_type {
                OperandType::LargeConstant => {
                    if cursor + 1 >= memory.len() {
                        return Err("Missing large constant operand".to_string());
                    }
                    let value = ((memory[cursor] as u16) << 8) | (memory[cursor + 1] as u16);
                    cursor += 2;
                    Operand { operand_type, value }
                },
                OperandType::SmallConstant => {
                    if cursor >= memory.len() {
                        return Err("Missing small constant operand".to_string());
                    }
                    let value = memory[cursor] as u16;
                    cursor += 1;
                    Operand { operand_type, value }
                },
                OperandType::Variable => {
                    if cursor >= memory.len() {
                        return Err("Missing variable operand".to_string());
                    }
                    let value = memory[cursor] as u16;
                    cursor += 1;
                    Operand { operand_type, value }
                },
                OperandType::Omitted => break,
            };
            operands.push(operand);
        }

        // Check if this is a store instruction and read store variable
        let mut store_variable = None;
        if Self::is_store_instruction(opcode, &operand_count) {
            if cursor >= memory.len() {
                return Err("Missing store variable".to_string());
            }
            store_variable = Some(memory[cursor]);
            cursor += 1;
        }

        // Check if this is a branch instruction and read branch offset
        let mut branch_offset = None;
        if Self::is_branch_instruction(opcode, &operand_count) {
            if cursor >= memory.len() {
                return Err("Missing branch offset".to_string());
            }
            
            let branch_byte = memory[cursor];
            cursor += 1;
            
            // Bit 7 = branch condition (0 = branch on false, 1 = branch on true)
            // Bit 6 = 0 means 2-byte offset, 1 means 1-byte offset
            let branch_on_true = (branch_byte & 0x80) != 0;
            let single_byte = (branch_byte & 0x40) != 0;
            
            let offset = if single_byte {
                // 6-bit signed offset (-64 to +63)
                let raw_offset = (branch_byte & 0x3F) as i8;
                if raw_offset > 63 {
                    (raw_offset as i16) - 128  // Convert to proper signed value
                } else {
                    raw_offset as i16
                }
            } else {
                // 14-bit signed offset 
                if cursor >= memory.len() {
                    return Err("Missing second branch offset byte".to_string());
                }
                let second_byte = memory[cursor];
                cursor += 1;
                
                let raw_offset = (((branch_byte & 0x3F) as u16) << 8) | (second_byte as u16);
                // Convert 14-bit unsigned to signed
                if raw_offset > 8191 {  // 2^13 - 1
                    (raw_offset as i16) - 16384  // 2^14
                } else {
                    raw_offset as i16
                }
            };
            
            // Store branch info (combine condition and offset)
            branch_offset = Some(if branch_on_true { offset } else { -offset - 1 });
        }

        let length = cursor - pc;

        Ok(Instruction {
            opcode,
            form,
            operand_count,
            operands,
            store_variable,
            branch_offset,
            text: None,           // Will be set by execution engine if needed
            length,
        })
    }

    fn decode_operand_type(bits: u8) -> OperandType {
        match bits {
            0b00 => OperandType::LargeConstant,
            0b01 => OperandType::SmallConstant,
            0b10 => OperandType::Variable,
            0b11 => OperandType::Omitted,
            _ => unreachable!(),
        }
    }

    fn is_branch_instruction(opcode: u8, operand_count: &OperandCount) -> bool {
        match operand_count {
            OperandCount::Op0 => {
                // 0OP branch instructions
                matches!(opcode, 0x05 | 0x06 | 0x0D | 0x0E | 0x0F)  // save, restore, verify, piracy, etc.
            },
            OperandCount::Op1 => {
                // 1OP branch instructions  
                matches!(opcode, 0x00 | 0x01 | 0x02)  // jz, get_sibling, get_child
            },
            OperandCount::Op2 => {
                // 2OP branch instructions
                matches!(opcode, 0x01 | 0x02 | 0x03 | 0x04 | 0x05 | 0x06 | 0x07 | 0x0A)  // je, jl, jg, dec_chk, inc_chk, jin, test, test_attr
            },
            OperandCount::Var => {
                // VAR branch instructions
                matches!(opcode, 0x17 | 0x1F)  // scan_table, check_arg_count
            },
        }
    }

    fn is_store_instruction(opcode: u8, operand_count: &OperandCount) -> bool {
        match operand_count {
            OperandCount::Op0 => {
                // 0OP store instructions
                matches!(opcode, 0x09)  // catch
            },
            OperandCount::Op1 => {
                // 1OP store instructions
                matches!(opcode, 0x01 | 0x02 | 0x03 | 0x04 | 0x08 | 0x0E | 0x0F)  // get_sibling, get_child, get_parent, get_prop_len, call_1s, load, not
            },
            OperandCount::Op2 => {
                // 2OP store instructions
                matches!(opcode, 0x08 | 0x09 | 0x0F | 0x10 | 0x11 | 0x12 | 0x13 | 0x14 | 0x15 | 0x16 | 0x17 | 0x18 | 0x19)  // or, and, loadw, loadb, get_prop, get_prop_addr, get_next_prop, add, sub, mul, div, mod, call_2s
            },
            OperandCount::Var => {
                // VAR store instructions
                matches!(opcode, 0x00 | 0x07 | 0x0C | 0x19 | 0x1A)  // call, random, call_vs2, call_vn2, call_vs
            },
        }
    }

    pub fn is_short(&self) -> bool {
        self.form == InstructionForm::Short
    }
    
    pub fn is_long(&self) -> bool {
        self.form == InstructionForm::Long
    }
    
    pub fn is_extended(&self) -> bool {
        self.form == InstructionForm::Extended
    }
    
    pub fn is_variable(&self) -> bool {
        self.form == InstructionForm::Variable
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Opcode: {:#04x}, Form: {:?}, Count: {:?}, Operands: {:?}", 
               self.opcode, self.form, self.operand_count, self.operands)
    }
}