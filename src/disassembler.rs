use crate::instruction::{Instruction, InstructionForm, OperandCount, OperandType};
use std::fmt::Write;

pub struct Disassembler<'a> {
    memory: &'a [u8],
}

impl<'a> Disassembler<'a> {
    pub fn new(memory: &'a [u8]) -> Self {
        Disassembler { memory }
    }
    
    /// Check if a given address might be the start of a routine
    /// Returns (is_routine, code_start_offset) where code_start_offset is where actual code begins
    fn check_routine_header(&self, addr: usize, force: bool) -> (bool, usize) {
        if addr >= self.memory.len() {
            return (false, 0);
        }
        
        let num_locals = self.memory[addr];
        
        // Z-machine routines can have 0-15 locals
        if num_locals > 15 {
            return (false, 0);
        }
        
        // For version 1-4, each local has a 2-byte default value
        // For version 5+, locals start at 0 (no defaults stored)
        // We'll assume version 3 for now (Zork 1)
        let header_size = 1 + (num_locals as usize * 2);
        
        // Make sure we have enough bytes for the header
        if addr + header_size > self.memory.len() {
            return (false, 0);
        }
        
        // If force is true, assume it's a routine (used when explicitly disassembling a routine)
        // Otherwise, we need better heuristics to avoid false positives
        if force {
            return (true, header_size);
        }
        
        // For automatic detection, be more conservative
        // We'll only treat it as a routine if explicitly told to
        (false, 0)
    }

    /// Format a routine header for display
    fn format_routine_header(&self, addr: usize) -> String {
        let mut output = String::new();
        let num_locals = self.memory[addr];
        
        writeln!(&mut output, "\n{:#06x}: === ROUTINE START ===", addr).unwrap();
        writeln!(&mut output, "{:#06x}: {:02x}              locals: {}", 
                 addr, num_locals, num_locals).unwrap();
        
        // Display default values for locals
        let mut offset = 1;
        for i in 0..num_locals {
            if addr + offset + 1 < self.memory.len() {
                let default_val = ((self.memory[addr + offset] as u16) << 8) | 
                                  (self.memory[addr + offset + 1] as u16);
                writeln!(&mut output, "{:#06x}: {:02x} {:02x}           local[{}] = {:#06x} ({})", 
                         addr + offset, 
                         self.memory[addr + offset], 
                         self.memory[addr + offset + 1],
                         i, default_val, default_val).unwrap();
                offset += 2;
            }
        }
        
        writeln!(&mut output, "{:#06x}: === CODE START ===", addr + offset).unwrap();
        output
    }

    /// Disassemble instructions starting from a given PC address
    /// Returns a formatted string containing the disassembly
    pub fn disassemble(&self, start_pc: usize, count: Option<usize>, byte_limit: Option<usize>) -> Result<String, String> {
        let mut output = String::new();
        let mut pc = start_pc;
        let mut instructions_decoded = 0;
        let start_byte = start_pc;

        writeln!(&mut output, "Disassembly starting at {:#06x}:", start_pc).unwrap();
        writeln!(&mut output, "").unwrap();

        loop {
            // Check if we've reached our limits
            if let Some(max_count) = count {
                if instructions_decoded >= max_count {
                    break;
                }
            }
            
            if let Some(max_bytes) = byte_limit {
                if pc - start_byte >= max_bytes {
                    break;
                }
            }

            // Check bounds
            if pc >= self.memory.len() {
                writeln!(&mut output, "{:#06x}: <end of memory>", pc).unwrap();
                break;
            }

            // Check if this might be a routine header
            let (is_routine, header_size) = self.check_routine_header(pc, false);
            if is_routine && header_size > 0 {
                // Format and display the routine header
                output.push_str(&self.format_routine_header(pc));
                pc += header_size;
                continue;
            }

            // Decode instruction
            match Instruction::decode(self.memory, pc) {
                Ok(instruction) => {
                    let disasm_line = self.format_instruction(pc, &instruction);
                    writeln!(&mut output, "{}", disasm_line).unwrap();
                    
                    pc += instruction.length;
                    instructions_decoded += 1;
                }
                Err(e) => {
                    writeln!(&mut output, "{:#06x}: <decode error: {}>", pc, e).unwrap();
                    break;
                }
            }
        }

        writeln!(&mut output, "\nDisassembled {} instructions ({} bytes)", 
                 instructions_decoded, pc - start_pc).unwrap();
        
        Ok(output)
    }

    /// Format a single instruction for display
    fn format_instruction(&self, pc: usize, instruction: &Instruction) -> String {
        let mut output = String::new();
        
        // Address and opcode
        write!(&mut output, "{:#06x}: ", pc).unwrap();
        
        // Opcode name and form
        let opcode_name = self.get_opcode_name(instruction);
        write!(&mut output, "{:<15} ", opcode_name).unwrap();
        
        // Operands
        if !instruction.operands.is_empty() {
            let operands_str = instruction.operands.iter()
                .map(|op| self.format_operand(op))
                .collect::<Vec<_>>()
                .join(", ");
            write!(&mut output, "{:<20} ", operands_str).unwrap();
        } else {
            write!(&mut output, "{:<20} ", "").unwrap();
        }
        
        // Store variable
        if let Some(store_var) = instruction.store_variable {
            write!(&mut output, "-> {:<6} ", self.format_variable(store_var)).unwrap();
        } else {
            write!(&mut output, "{:<10} ", "").unwrap();
        }
        
        // Branch offset
        if let Some(offset) = instruction.branch_offset {
            let branch_str = self.format_branch(pc, instruction, offset);
            write!(&mut output, "{}", branch_str).unwrap();
        }
        
        output
    }

    /// Format an operand for display
    fn format_operand(&self, operand: &crate::instruction::Operand) -> String {
        match operand.operand_type {
            OperandType::LargeConstant => format!("#{:#06x}", operand.value),
            OperandType::SmallConstant => format!("#{:#04x}", operand.value),
            OperandType::Variable => self.format_variable(operand.value as u8),
            OperandType::Omitted => String::new(),
        }
    }

    /// Format a variable reference
    fn format_variable(&self, var: u8) -> String {
        match var {
            0x00 => "(SP)".to_string(),
            0x01..=0x0f => format!("L{:02x}", var - 1),
            0x10..=0xff => format!("G{:02x}", var - 0x10),
        }
    }

    /// Format branch information
    fn format_branch(&self, pc: usize, instruction: &Instruction, stored_offset: i16) -> String {
        // The stored_offset encodes both condition and offset
        // If negative, it means branch on false, and actual offset is -(stored_offset + 1)
        // If positive or zero, it means branch on true, and actual offset is stored_offset
        let (condition, actual_offset) = if stored_offset < 0 {
            ("FALSE", -(stored_offset + 1))
        } else {
            ("TRUE", stored_offset)
        };
        
        if actual_offset == 0 {
            format!("[{}: RFALSE]", condition)
        } else if actual_offset == 1 {
            format!("[{}: RTRUE]", condition)
        } else {
            let target = (pc as i32 + instruction.length as i32 + actual_offset as i32 - 2) as usize;
            format!("[{}: {:#06x}]", condition, target)
        }
    }

    /// Get the name of an opcode
    fn get_opcode_name(&self, instruction: &Instruction) -> String {
        match (&instruction.operand_count, instruction.opcode) {
            // 0OP instructions
            (OperandCount::Op0, 0x00) => "RTRUE",
            (OperandCount::Op0, 0x01) => "RFALSE",
            (OperandCount::Op0, 0x02) => "PRINT",
            (OperandCount::Op0, 0x03) => "PRINT_RET",
            (OperandCount::Op0, 0x04) => "NOP",
            (OperandCount::Op0, 0x05) => "SAVE",
            (OperandCount::Op0, 0x06) => "RESTORE",
            (OperandCount::Op0, 0x07) => "RESTART",
            (OperandCount::Op0, 0x08) => "RET_POPPED",
            (OperandCount::Op0, 0x09) => "CATCH",
            (OperandCount::Op0, 0x0a) => "QUIT",
            (OperandCount::Op0, 0x0b) => "NEW_LINE",
            (OperandCount::Op0, 0x0c) => "SHOW_STATUS",
            (OperandCount::Op0, 0x0d) => "VERIFY",
            (OperandCount::Op0, 0x0f) => "PIRACY",

            // 1OP instructions
            (OperandCount::Op1, 0x00) => "JZ",
            (OperandCount::Op1, 0x01) => "GET_SIBLING",
            (OperandCount::Op1, 0x02) => "GET_CHILD",
            (OperandCount::Op1, 0x03) => "GET_PARENT",
            (OperandCount::Op1, 0x04) => "GET_PROP_LEN",
            (OperandCount::Op1, 0x05) => "INC",
            (OperandCount::Op1, 0x06) => "DEC",
            (OperandCount::Op1, 0x07) => "PRINT_ADDR",
            (OperandCount::Op1, 0x08) => "CALL_1S",
            (OperandCount::Op1, 0x09) => "REMOVE_OBJ",
            (OperandCount::Op1, 0x0a) => "PRINT_OBJ",
            (OperandCount::Op1, 0x0b) => "RET",
            (OperandCount::Op1, 0x0c) => "JUMP",
            (OperandCount::Op1, 0x0d) => "PRINT_PADDR",
            (OperandCount::Op1, 0x0e) => "LOAD",
            (OperandCount::Op1, 0x0f) => "NOT",

            // 2OP instructions
            (OperandCount::Op2, 0x01) => "JE",
            (OperandCount::Op2, 0x02) => "JL",
            (OperandCount::Op2, 0x03) => "JG",
            (OperandCount::Op2, 0x04) => "DEC_CHK",
            (OperandCount::Op2, 0x05) => "INC_CHK",
            (OperandCount::Op2, 0x06) => "JIN",
            (OperandCount::Op2, 0x07) => "TEST",
            (OperandCount::Op2, 0x08) => "OR",
            (OperandCount::Op2, 0x09) => "AND",
            (OperandCount::Op2, 0x0a) => "TEST_ATTR",
            (OperandCount::Op2, 0x0b) => "SET_ATTR",
            (OperandCount::Op2, 0x0c) => "CLEAR_ATTR",
            (OperandCount::Op2, 0x0d) => "STORE",
            (OperandCount::Op2, 0x0e) => "INSERT_OBJ",
            (OperandCount::Op2, 0x0f) => "LOADW",
            (OperandCount::Op2, 0x10) => "LOADB",
            (OperandCount::Op2, 0x11) => "GET_PROP",
            (OperandCount::Op2, 0x12) => "GET_PROP_ADDR",
            (OperandCount::Op2, 0x13) => "GET_NEXT_PROP",
            (OperandCount::Op2, 0x14) => "ADD",
            (OperandCount::Op2, 0x15) => "SUB",
            (OperandCount::Op2, 0x16) => "MUL",
            (OperandCount::Op2, 0x17) => "DIV",
            (OperandCount::Op2, 0x18) => "MOD",
            (OperandCount::Op2, 0x19) => "CALL_2S",

            // VAR instructions
            (OperandCount::Var, 0x00) => "CALL",
            (OperandCount::Var, 0x01) => "STOREW",
            (OperandCount::Var, 0x02) => "STOREB",
            (OperandCount::Var, 0x03) => "PUT_PROP",
            (OperandCount::Var, 0x04) => "SREAD",
            (OperandCount::Var, 0x05) => "PRINT_CHAR",
            (OperandCount::Var, 0x06) => "PRINT_NUM",
            (OperandCount::Var, 0x07) => "RANDOM",
            (OperandCount::Var, 0x08) => "PUSH",
            (OperandCount::Var, 0x09) => "PULL",
            (OperandCount::Var, 0x0a) => "SPLIT_WINDOW",
            (OperandCount::Var, 0x0b) => "SET_WINDOW",
            (OperandCount::Var, 0x0c) => "CALL_VS2",
            (OperandCount::Var, 0x0d) => "ERASE_WINDOW",
            (OperandCount::Var, 0x0e) => "ERASE_LINE",
            (OperandCount::Var, 0x0f) => "SET_CURSOR",
            (OperandCount::Var, 0x10) => "GET_CURSOR",
            (OperandCount::Var, 0x11) => "SET_TEXT_STYLE",
            (OperandCount::Var, 0x12) => "BUFFER_MODE",
            (OperandCount::Var, 0x13) => "OUTPUT_STREAM",
            (OperandCount::Var, 0x14) => "INPUT_STREAM",
            (OperandCount::Var, 0x15) => "SOUND_EFFECT",
            (OperandCount::Var, 0x16) => "READ_CHAR",
            (OperandCount::Var, 0x17) => "SCAN_TABLE",
            (OperandCount::Var, 0x18) => "NOT",
            (OperandCount::Var, 0x19) => "CALL_VN",
            (OperandCount::Var, 0x1a) => "CALL_VN2",
            (OperandCount::Var, 0x1b) => "TOKENISE",
            (OperandCount::Var, 0x1c) => "ENCODE_TEXT",
            (OperandCount::Var, 0x1d) => "COPY_TABLE",
            (OperandCount::Var, 0x1e) => "PRINT_TABLE",
            (OperandCount::Var, 0x1f) => "CHECK_ARG_COUNT",

            _ => {
                if instruction.form == InstructionForm::Extended {
                    return format!("EXT_{:#04x}", instruction.opcode);
                } else {
                    return format!("UNK_{:?}_{:#04x}", instruction.operand_count, instruction.opcode);
                }
            }
        }.to_string()
    }
}

/// Convenience function to disassemble a range of memory
pub fn disassemble_range(memory: &[u8], start_pc: usize, end_pc: usize) -> Result<String, String> {
    let disassembler = Disassembler::new(memory);
    let byte_limit = end_pc.saturating_sub(start_pc);
    disassembler.disassemble(start_pc, None, Some(byte_limit))
}

/// Convenience function to disassemble a specific number of instructions
pub fn disassemble_instructions(memory: &[u8], start_pc: usize, count: usize) -> Result<String, String> {
    let disassembler = Disassembler::new(memory);
    disassembler.disassemble(start_pc, Some(count), None)
}

/// Disassemble a routine at a given packed address
pub fn disassemble_routine(memory: &[u8], packed_addr: u16, version: u8) -> Result<String, String> {
    let disassembler = Disassembler::new(memory);
    
    // Convert packed address to byte address
    let byte_addr = match version {
        1 | 2 | 3 => (packed_addr as usize) * 2,
        4 | 5 => (packed_addr as usize) * 4,
        6 | 7 | 8 => {
            // Version 6+ would need routine base address from header
            // For now, just use the simple calculation
            (packed_addr as usize) * 4
        }
        _ => (packed_addr as usize) * 2,
    };
    
    let mut output = String::new();
    writeln!(&mut output, "Routine at packed address {:#06x} (byte address {:#06x}):", 
             packed_addr, byte_addr).unwrap();
    writeln!(&mut output, "").unwrap();
    
    // First, format the routine header
    if byte_addr < memory.len() {
        output.push_str(&disassembler.format_routine_header(byte_addr));
        
        // Then disassemble the code
        let num_locals = memory[byte_addr];
        let code_start = byte_addr + 1 + (num_locals as usize * 2);
        
        // Disassemble up to 100 instructions or until we hit what looks like another routine
        match disassembler.disassemble(code_start, Some(100), None) {
            Ok(disasm) => output.push_str(&disasm),
            Err(e) => writeln!(&mut output, "Error disassembling routine: {}", e).unwrap(),
        }
    } else {
        writeln!(&mut output, "Error: Address {:#06x} is out of bounds", byte_addr).unwrap();
    }
    
    Ok(output)
}