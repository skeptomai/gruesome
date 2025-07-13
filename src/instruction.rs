#![allow(unused_imports)]
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;
use log::info;

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

#[derive(Clone)]
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
    fn get_opcode_name(opcode: u8, operand_count: &OperandCount) -> &'static str {
        match operand_count {
            OperandCount::Op0 => match opcode {
                0x00 => "rtrue",
                0x01 => "rfalse",
                0x02 => "print",
                0x03 => "print_ret",
                0x04 => "nop",
                0x05 => "save",
                0x06 => "restore",
                0x07 => "restart",
                0x08 => "ret_popped",
                0x09 => "catch",
                0x0A => "quit",
                0x0B => "new_line",
                0x0C => "show_status",
                0x0D => "verify",
                0x0E => "extended",
                0x0F => "piracy",
                _ => "unknown_0op",
            },
            OperandCount::Op1 => match opcode {
                0x00 => "jz",
                0x01 => "get_sibling",
                0x02 => "get_child",
                0x03 => "get_parent",
                0x04 => "get_prop_len",
                0x05 => "inc",
                0x06 => "dec",
                0x07 => "print_addr",
                0x08 => "call_1s",
                0x09 => "remove_obj",
                0x0A => "print_obj",
                0x0B => "ret",
                0x0C => "jump",
                0x0D => "print_paddr",
                0x0E => "load",
                0x0F => "not",
                _ => "unknown_1op",
            },
            OperandCount::Op2 => match opcode {
                0x01 => "je",
                0x02 => "jl",
                0x03 => "jg",
                0x04 => "dec_chk",
                0x05 => "inc_chk",
                0x06 => "jin",
                0x07 => "test",
                0x08 => "or",
                0x09 => "and",
                0x0A => "test_attr",
                0x0B => "set_attr",
                0x0C => "clear_attr",
                0x0D => "store",
                0x0E => "insert_obj",
                0x0F => "loadw",
                0x10 => "loadb",
                0x11 => "get_prop",
                0x12 => "get_prop_addr",
                0x13 => "get_next_prop",
                0x14 => "add",
                0x15 => "sub",
                0x16 => "mul",
                0x17 => "div",
                0x18 => "mod",
                0x19 => "call_2s",
                0x1A => "call_2n",
                0x1B => "set_colour",
                0x1C => "throw",
                _ => "unknown_2op",
            },
            OperandCount::Var => match opcode {
                0x00 => "call",
                0x01 => "storew",
                0x02 => "storeb",
                0x03 => "put_prop",
                0x04 => "sread",
                0x05 => "print_char",
                0x06 => "print_num",
                0x07 => "random",
                0x08 => "push",
                0x09 => "pull",
                0x0A => "split_window",
                0x0B => "set_window",
                0x0C => "call_vs2",
                0x0D => "erase_window",
                0x0E => "erase_line",
                0x0F => "set_cursor",
                0x10 => "get_cursor",
                0x11 => "set_text_style",
                0x12 => "buffer_mode",
                0x13 => "output_stream",
                0x14 => "input_stream",
                0x15 => "sound_effect",
                0x16 => "read_char",
                0x17 => "scan_table",
                0x18 => "not_v4",
                0x19 => "call_vn",
                0x1A => "call_vn2",
                0x1B => "tokenise",
                0x1C => "encode_text",
                0x1D => "copy_table",
                0x1E => "print_table",
                0x1F => "check_arg_count",
                _ => "unknown_var",
            },
        }
    }

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
                // 6-bit signed offset 
                let raw_offset = branch_byte & 0x3F;
                // Sign extend from 6 bits
                if raw_offset & 0x20 != 0 {
                    (raw_offset as i16) | (-64i16)  // Sign extend
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

        let instruction = Instruction {
            opcode,
            form,
            operand_count,
            operands,
            store_variable,
            branch_offset,
            text: None,           // Will be set by execution engine if needed
            length,
        };


        Ok(instruction)
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

impl Debug for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let opcode_name = Self::get_opcode_name(self.opcode, &self.operand_count);
        write!(
            f,
            "name: {} (opcode={:#04x}, form={:?}, operand_count={:?}, operands={:?}, store_var={:?}, branch={:?})",
            opcode_name,
            self.opcode,
            self.form,
            self.operand_count,
            self.operands,
            self.store_variable,
            self.branch_offset
        )
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let opcode_name = Self::get_opcode_name(self.opcode, &self.operand_count);
        write!(f, "{}", opcode_name)?;
        
        // Show operands in a more readable format
        if !self.operands.is_empty() {
            write!(f, " ")?;
            for (i, operand) in self.operands.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                match operand.operand_type {
                    OperandType::Variable => write!(f, "var{}", operand.value)?,
                    OperandType::SmallConstant | OperandType::LargeConstant => write!(f, "#{}", operand.value)?,
                    OperandType::Omitted => write!(f, "_")?,
                }
            }
        }
        
        // Show store variable if present
        if let Some(var) = self.store_variable {
            write!(f, " -> var{}", var)?;
        }
        
        // Show branch offset if present
        if let Some(offset) = self.branch_offset {
            write!(f, " ?{}", offset)?;
        }
        
        Ok(())
    }
}
