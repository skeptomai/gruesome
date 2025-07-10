use std::collections::HashMap;
use std::io::{self, Write};
use crate::game::GameFile;
use crate::instruction::{Instruction, OperandType, OperandCount};
use crate::util::get_mem_addr;

pub struct ZMachine<'a> {
    pub game: &'a GameFile<'a>,
    pub memory: Vec<u8>,
    pub pc: usize,
    pub stack: Vec<u16>,
    pub call_stack: Vec<CallFrame>,
    pub global_vars: HashMap<u8, u16>,
    pub local_vars: [u16; 15],  // Max 15 local variables
    pub running: bool,
    pub operands_buffer: Vec<u16>,  // Buffer for current instruction operands
    pub current_branch_offset: Option<i16>,  // Branch offset for current instruction
}

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub return_pc: usize,
    pub local_vars: [u16; 15],
    pub num_locals: u8,
    pub result_var: Option<u8>,
}

impl<'a> ZMachine<'a> {
    pub fn new(game: &'a GameFile<'a>) -> Self {
        let mut memory = game.bytes().to_vec();
        let pc = game.header().initial_pc;
        
        Self {
            game,
            memory,
            pc,
            stack: Vec::new(),
            call_stack: Vec::new(),
            global_vars: HashMap::new(),
            local_vars: [0; 15],
            running: true,
            operands_buffer: Vec::new(),
            current_branch_offset: None,
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        println!("Starting Z-Machine execution at PC: {:#06x}", self.pc);
        
        while self.running {
            let instruction = Instruction::decode(&self.memory, self.pc)?;
            println!("PC: {:#06x} - {} | Bytes: [{:#04x} {:#04x} {:#04x}]", 
                    self.pc, instruction, 
                    self.memory[self.pc],
                    if self.pc + 1 < self.memory.len() { self.memory[self.pc + 1] } else { 0 },
                    if self.pc + 2 < self.memory.len() { self.memory[self.pc + 2] } else { 0 });
            
            self.pc += instruction.length;
            self.execute_instruction(instruction)?;
        }
        
        Ok(())
    }

    pub fn execute_instruction(&mut self, instruction: Instruction) -> Result<(), String> {
        // Store branch offset for current instruction
        self.current_branch_offset = instruction.branch_offset;
        
        // Populate operands buffer for current instruction
        self.operands_buffer.clear();
        for operand in &instruction.operands {
            let value = self.resolve_operand(operand)?;
            self.operands_buffer.push(value);
        }
        
        match instruction.operand_count {
            OperandCount::Op0 => self.execute_0op(instruction),
            OperandCount::Op1 => self.execute_1op(instruction),
            OperandCount::Op2 => self.execute_2op(instruction),
            OperandCount::Var => self.execute_var(instruction),
        }
    }

    fn execute_0op(&mut self, instruction: Instruction) -> Result<(), String> {
        match instruction.opcode {
            0x00 => self.op_rtrue(),
            0x01 => self.op_rfalse(),
            0x02 => self.op_print(),
            0x03 => self.op_print_ret(),
            0x08 => self.op_ret_popped(),
            0x09 => self.op_catch(),
            0x0A => self.op_quit(),
            0x0B => self.op_new_line(),
            _ => Err(format!("Unknown 0OP instruction: {:#04x}", instruction.opcode)),
        }
    }

    fn execute_1op(&mut self, instruction: Instruction) -> Result<(), String> {
        if instruction.operands.is_empty() {
            return Err("1OP instruction missing operand".to_string());
        }
        
        let operand = self.resolve_operand(&instruction.operands[0])?;
        
        match instruction.opcode {
            0x00 => self.op_jz(operand),
            0x01 => self.op_get_sibling(operand),
            0x02 => self.op_get_child(operand),
            0x03 => self.op_get_parent(operand),
            0x04 => self.op_get_prop_len(operand),
            0x05 => self.op_inc(operand),
            0x06 => self.op_dec(operand),
            0x07 => self.op_print_addr(operand),
            0x08 => self.op_call_1s(operand),
            0x09 => self.op_remove_obj(operand),
            0x0A => self.op_print_obj(operand),
            0x0B => self.op_ret(operand),
            0x0C => self.op_jump(operand),
            0x0D => self.op_print_paddr(operand),
            0x0E => self.op_load(operand),
            0x0F => self.op_not(operand),
            _ => Err(format!("Unknown 1OP instruction: {:#04x}", instruction.opcode)),
        }
    }

    fn execute_2op(&mut self, instruction: Instruction) -> Result<(), String> {
        if instruction.operands.len() < 2 {
            return Err("2OP instruction missing operands".to_string());
        }
        
        let operand1 = self.resolve_operand(&instruction.operands[0])?;
        let operand2 = self.resolve_operand(&instruction.operands[1])?;
        
        match instruction.opcode {
            0x00 => {
                println!("WARNING: 2OP opcode 0 is reserved/unused - treating as NOP");
                Ok(())
            },
            0x01 => self.op_je(operand1, operand2),
            0x02 => self.op_jl(operand1, operand2),
            0x03 => self.op_jg(operand1, operand2),
            0x04 => self.op_dec_chk(operand1, operand2),
            0x05 => self.op_inc_chk(operand1, operand2),
            0x06 => self.op_jin(operand1, operand2),
            0x07 => self.op_test(operand1, operand2),
            0x08 => self.op_or(operand1, operand2),
            0x09 => self.op_and(operand1, operand2),
            0x0A => self.op_test_attr(operand1, operand2),
            0x0B => self.op_set_attr(operand1, operand2),
            0x0C => self.op_clear_attr(operand1, operand2),
            0x0D => self.op_store(operand1, operand2),
            0x0E => self.op_insert_obj(operand1, operand2),
            0x0F => self.op_loadw(operand1, operand2),
            0x10 => self.op_loadb(operand1, operand2),
            0x11 => self.op_get_prop(operand1, operand2),
            0x12 => self.op_get_prop_addr(operand1, operand2),
            0x13 => self.op_get_next_prop(operand1, operand2),
            0x14 => self.op_add(operand1, operand2),
            0x15 => self.op_sub(operand1, operand2),
            0x16 => self.op_mul(operand1, operand2),
            0x17 => self.op_div(operand1, operand2),
            0x18 => self.op_mod(operand1, operand2),
            0x19 => self.op_call_2s(operand1, operand2),
            0x1A => self.op_call_2n(operand1, operand2),
            0x1B => self.op_set_colour(operand1, operand2),
            0x1C => self.op_throw(operand1, operand2),
            _ => Err(format!("Unknown 2OP instruction: {:#04x}", instruction.opcode)),
        }
    }

    fn execute_var(&mut self, instruction: Instruction) -> Result<(), String> {
        match instruction.opcode {
            0x00 => self.op_call(),
            0x01 => self.op_storew(),
            0x02 => self.op_storeb(),
            0x03 => self.op_put_prop(),
            0x04 => self.op_sread(),
            0x05 => self.op_print_char(),
            0x06 => self.op_print_num(),
            0x07 => self.op_random(),
            0x08 => self.op_push(),
            0x09 => self.op_pull(),
            0x0A => self.op_split_window(),
            0x0B => self.op_set_window(),
            0x0C => self.op_call_vs2(),
            0x0D => self.op_erase_window(),
            0x0E => self.op_erase_line(),
            0x0F => self.op_set_cursor(),
            0x10 => self.op_get_cursor(),
            0x11 => self.op_set_text_style(),
            0x12 => self.op_buffer_mode(),
            0x13 => self.op_output_stream(),
            0x14 => self.op_input_stream(),
            0x15 => self.op_sound_effect(),
            0x16 => self.op_read_char(),
            0x17 => self.op_scan_table(),
            0x18 => self.op_not_v4(),
            0x19 => self.op_call_vn(),
            0x1A => self.op_call_vn2(),
            0x1B => self.op_tokenise(),
            0x1C => self.op_encode_text(),
            0x1D => self.op_copy_table(),
            0x1E => self.op_print_table(),
            0x1F => self.op_check_arg_count(),
            _ => Err(format!("Unknown VAR instruction: {:#04x}", instruction.opcode)),
        }
    }

    fn resolve_operand(&self, operand: &crate::instruction::Operand) -> Result<u16, String> {
        match operand.operand_type {
            OperandType::LargeConstant | OperandType::SmallConstant => Ok(operand.value),
            OperandType::Variable => {
                if operand.value == 0 {
                    // Stack
                    if self.stack.is_empty() {
                        Err("Attempted to read from empty stack".to_string())
                    } else {
                        Ok(self.stack[self.stack.len() - 1])
                    }
                } else if operand.value <= 15 {
                    // Local variable
                    Ok(self.local_vars[(operand.value - 1) as usize])
                } else {
                    // Global variable
                    Ok(self.global_vars.get(&(operand.value as u8)).copied().unwrap_or(0))
                }
            },
            OperandType::Omitted => Err("Cannot resolve omitted operand".to_string()),
        }
    }

    fn store_variable(&mut self, var: u8, value: u16) -> Result<(), String> {
        if var == 0 {
            // Stack
            self.stack.push(value);
        } else if var <= 15 {
            // Local variable
            self.local_vars[(var - 1) as usize] = value;
        } else {
            // Global variable
            self.global_vars.insert(var, value);
        }
        Ok(())
    }

    // 0OP Instructions
    fn op_rtrue(&mut self) -> Result<(), String> {
        self.return_from_routine(1)
    }

    fn op_rfalse(&mut self) -> Result<(), String> {
        self.return_from_routine(0)
    }

    fn op_print(&mut self) -> Result<(), String> {
        // Print immediate string following instruction
        println!("PRINT: [immediate string not implemented]");
        Ok(())
    }

    fn op_print_ret(&mut self) -> Result<(), String> {
        self.op_print()?;
        println!();
        self.return_from_routine(1)
    }

    fn op_ret_popped(&mut self) -> Result<(), String> {
        if self.stack.is_empty() {
            return Err("Stack underflow in ret_popped".to_string());
        }
        let value = self.stack.pop().unwrap();
        self.return_from_routine(value)
    }

    fn op_catch(&mut self) -> Result<(), String> {
        // Store current call stack frame count
        let frame_count = self.call_stack.len() as u16;
        self.store_variable(0, frame_count)?; // Store on stack
        Ok(())
    }

    fn op_quit(&mut self) -> Result<(), String> {
        println!("Game quit.");
        self.running = false;
        Ok(())
    }

    fn op_new_line(&mut self) -> Result<(), String> {
        println!();
        Ok(())
    }

    // 1OP Instructions
    pub fn op_jz(&mut self, operand: u16) -> Result<(), String> {
        let condition = operand == 0;
        self.handle_branch(condition)
    }

    pub fn op_get_sibling(&mut self, operand: u16) -> Result<(), String> {
        let obj_num = operand;
        let sibling = self.get_object_sibling(obj_num)?;
        
        // Store the sibling object number
        self.store_variable(0, sibling)?;
        
        // Branch if sibling is not 0 (object has a sibling)
        let condition = sibling != 0;
        self.handle_branch(condition)
    }

    pub fn op_get_child(&mut self, operand: u16) -> Result<(), String> {
        let obj_num = operand;
        let child = self.get_object_child(obj_num)?;
        
        // Store the child object number
        self.store_variable(0, child)?;
        
        // Branch if child is not 0 (object has a child)
        let condition = child != 0;
        self.handle_branch(condition)
    }

    fn op_get_parent(&mut self, operand: u16) -> Result<(), String> {
        println!("GET_PARENT: object {} (not implemented)", operand);
        Ok(())
    }

    fn op_get_prop_len(&mut self, operand: u16) -> Result<(), String> {
        println!("GET_PROP_LEN: property {} (not implemented)", operand);
        Ok(())
    }

    fn op_inc(&mut self, operand: u16) -> Result<(), String> {
        println!("INC: variable {} (not implemented)", operand);
        Ok(())
    }

    fn op_dec(&mut self, operand: u16) -> Result<(), String> {
        println!("DEC: variable {} (not implemented)", operand);
        Ok(())
    }

    fn op_print_addr(&mut self, operand: u16) -> Result<(), String> {
        println!("PRINT_ADDR: {} (not implemented)", operand);
        Ok(())
    }

    fn op_call_1s(&mut self, operand: u16) -> Result<(), String> {
        println!("CALL_1S: routine {} (not implemented)", operand);
        Ok(())
    }

    fn op_remove_obj(&mut self, operand: u16) -> Result<(), String> {
        println!("REMOVE_OBJ: object {} (not implemented)", operand);
        Ok(())
    }

    fn op_print_obj(&mut self, operand: u16) -> Result<(), String> {
        println!("PRINT_OBJ: object {} (not implemented)", operand);
        Ok(())
    }

    fn op_ret(&mut self, operand: u16) -> Result<(), String> {
        self.return_from_routine(operand)
    }

    fn op_jump(&mut self, operand: u16) -> Result<(), String> {
        // Signed jump offset
        let offset = operand as i16;
        self.pc = ((self.pc as i32) + (offset as i32) - 2) as usize;
        Ok(())
    }

    fn op_print_paddr(&mut self, operand: u16) -> Result<(), String> {
        println!("PRINT_PADDR: {} (not implemented)", operand);
        Ok(())
    }

    fn op_load(&mut self, operand: u16) -> Result<(), String> {
        println!("LOAD: variable {} (not implemented)", operand);
        Ok(())
    }

    fn op_not(&mut self, operand: u16) -> Result<(), String> {
        let result = !operand;
        self.store_variable(0, result)?; // Store result on stack
        Ok(())
    }

    // 2OP Instructions
    pub fn op_je(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let condition = operand1 == operand2;
        self.handle_branch(condition)
    }

    pub fn op_jl(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let condition = (operand1 as i16) < (operand2 as i16);
        self.handle_branch(condition)
    }

    pub fn op_jg(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let condition = (operand1 as i16) > (operand2 as i16);
        self.handle_branch(condition)
    }

    pub fn op_dec_chk(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let var_num = operand1 as u8;
        let threshold = operand2 as i16;
        
        // Get current value of variable
        let current_value = if var_num == 0 {
            if self.stack.is_empty() {
                return Err("Stack underflow in dec_chk".to_string());
            }
            self.stack.pop().unwrap()
        } else if var_num <= 15 {
            self.local_vars[(var_num - 1) as usize]
        } else {
            self.global_vars.get(&var_num).copied().unwrap_or(0)
        };
        
        // Decrement the value
        let new_value = (current_value as i16).wrapping_sub(1) as u16;
        
        // Store the decremented value back
        self.store_variable(var_num, new_value)?;
        
        // Branch if new value < threshold
        let condition = (new_value as i16) < threshold;
        self.handle_branch(condition)
    }

    pub fn op_inc_chk(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let var_num = operand1 as u8;
        let threshold = operand2 as i16;
        
        // Get current value of variable
        let current_value = if var_num == 0 {
            if self.stack.is_empty() {
                return Err("Stack underflow in inc_chk".to_string());
            }
            self.stack.pop().unwrap()
        } else if var_num <= 15 {
            self.local_vars[(var_num - 1) as usize]
        } else {
            self.global_vars.get(&var_num).copied().unwrap_or(0)
        };
        
        // Increment the value
        let new_value = (current_value as i16).wrapping_add(1) as u16;
        
        // Store the incremented value back
        self.store_variable(var_num, new_value)?;
        
        // Branch if new value > threshold
        let condition = (new_value as i16) > threshold;
        self.handle_branch(condition)
    }

    pub fn op_jin(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let child_obj = operand1;
        let parent_obj = operand2;
        
        // Get the parent of child_obj and check if it matches parent_obj
        let child_parent = self.get_object_parent(child_obj)?;
        let condition = child_parent == parent_obj;
        
        self.handle_branch(condition)
    }

    pub fn op_test(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let condition = (operand1 & operand2) == operand2;
        self.handle_branch(condition)
    }

    fn op_or(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let result = operand1 | operand2;
        self.store_variable(0, result)?;
        Ok(())
    }

    fn op_and(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let result = operand1 & operand2;
        self.store_variable(0, result)?;
        Ok(())
    }

    pub fn op_test_attr(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let obj_num = operand1;
        let attr_num = operand2;
        
        // Test if the object has the specified attribute
        let condition = self.test_object_attribute(obj_num, attr_num)?;
        self.handle_branch(condition)
    }

    fn op_set_attr(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        println!("SET_ATTR: obj {} attr {} (not implemented)", operand1, operand2);
        Ok(())
    }

    fn op_clear_attr(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        println!("CLEAR_ATTR: obj {} attr {} (not implemented)", operand1, operand2);
        Ok(())
    }

    fn op_store(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        self.store_variable(operand1 as u8, operand2)
    }

    fn op_insert_obj(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        println!("INSERT_OBJ: obj {} into obj {} (not implemented)", operand1, operand2);
        Ok(())
    }

    fn op_loadw(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let addr = operand1 as usize + (operand2 as usize * 2);
        if addr + 1 < self.memory.len() {
            let value = ((self.memory[addr] as u16) << 8) | (self.memory[addr + 1] as u16);
            self.store_variable(0, value)?;
        }
        Ok(())
    }

    fn op_loadb(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let addr = operand1 as usize + operand2 as usize;
        if addr < self.memory.len() {
            let value = self.memory[addr] as u16;
            self.store_variable(0, value)?;
        }
        Ok(())
    }

    fn op_get_prop(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        println!("GET_PROP: obj {} prop {} (not implemented)", operand1, operand2);
        Ok(())
    }

    fn op_get_prop_addr(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        println!("GET_PROP_ADDR: obj {} prop {} (not implemented)", operand1, operand2);
        Ok(())
    }

    fn op_get_next_prop(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        println!("GET_NEXT_PROP: obj {} prop {} (not implemented)", operand1, operand2);
        Ok(())
    }

    fn op_add(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let result = operand1.wrapping_add(operand2);
        self.store_variable(0, result)?;
        Ok(())
    }

    fn op_sub(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let result = operand1.wrapping_sub(operand2);
        self.store_variable(0, result)?;
        Ok(())
    }

    fn op_mul(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let result = operand1.wrapping_mul(operand2);
        self.store_variable(0, result)?;
        Ok(())
    }

    fn op_div(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        if operand2 == 0 {
            return Err("Division by zero".to_string());
        }
        let result = (operand1 as i16) / (operand2 as i16);
        self.store_variable(0, result as u16)?;
        Ok(())
    }

    fn op_mod(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        if operand2 == 0 {
            return Err("Modulo by zero".to_string());
        }
        let result = (operand1 as i16) % (operand2 as i16);
        self.store_variable(0, result as u16)?;
        Ok(())
    }

    // VAR Instructions
    pub fn op_call(&mut self) -> Result<(), String> {
        // CALL takes a variable number of operands:
        // call routine arg1 arg2 ... -> (result)
        // If routine address is 0, return 0 immediately
        
        // We need at least 1 operand (the routine address)
        if self.operands_buffer.is_empty() {
            return Err("CALL instruction missing routine address".to_string());
        }
        
        let routine_addr = self.operands_buffer[0];
        
        if routine_addr == 0 {
            // Call to routine 0 returns 0
            self.store_variable(0, 0)?; // Store 0 on stack
            return Ok(());
        }
        
        // Convert packed address to byte address
        let version = if self.memory.len() > 0 { self.memory[0] } else { 3 };
        let byte_addr = match version {
            1 | 2 | 3 => (routine_addr as usize) * 2,
            4 | 5 => (routine_addr as usize) * 4,
            6 | 7 | 8 => {
                let base_high = if self.memory.len() > 5 { 
                    ((self.memory[4] as usize) << 8) | (self.memory[5] as usize)
                } else { 0 };
                (routine_addr as usize) * 4 + base_high
            },
            _ => return Err("Unsupported Z-Machine version".to_string()),
        };
        
        if byte_addr >= self.memory.len() {
            return Err(format!("Routine address out of bounds: {:#06x}", byte_addr));
        }
        
        // Read routine header
        let num_locals = self.memory[byte_addr];
        if num_locals > 15 {
            return Err(format!("Too many local variables in routine: {}", num_locals));
        }
        
        let mut routine_pc = byte_addr + 1;
        let mut local_defaults = [0u16; 15];
        
        // In versions 1-4, read default values for locals
        if version <= 4 {
            for i in 0..(num_locals as usize) {
                if routine_pc + 1 >= self.memory.len() {
                    return Err("Routine header truncated".to_string());
                }
                local_defaults[i] = ((self.memory[routine_pc] as u16) << 8) | (self.memory[routine_pc + 1] as u16);
                routine_pc += 2;
            }
        }
        
        // Save current call frame
        let call_frame = CallFrame {
            return_pc: self.pc,
            local_vars: self.local_vars,
            num_locals: num_locals,
            result_var: Some(0), // Store result on stack by default
        };
        self.call_stack.push(call_frame);
        
        // Set up new locals with defaults
        self.local_vars = local_defaults;
        
        // Pass arguments to local variables
        let args = &self.operands_buffer[1..]; // Skip routine address
        for (i, &arg_value) in args.iter().enumerate() {
            if i < num_locals as usize {
                self.local_vars[i] = arg_value;
            }
        }
        
        // Jump to routine code
        self.pc = routine_pc;
        
        println!("CALL: Calling routine at {:#06x} with {} locals, {} args", 
                byte_addr, num_locals, args.len());
        
        Ok(())
    }

    fn op_storew(&mut self) -> Result<(), String> {
        println!("STOREW: (not implemented)");
        Ok(())
    }

    fn op_storeb(&mut self) -> Result<(), String> {
        println!("STOREB: (not implemented)");
        Ok(())
    }

    fn op_put_prop(&mut self) -> Result<(), String> {
        println!("PUT_PROP: (not implemented)");
        Ok(())
    }

    fn op_sread(&mut self) -> Result<(), String> {
        println!("SREAD: (not implemented)");
        Ok(())
    }

    fn op_print_char(&mut self) -> Result<(), String> {
        println!("PRINT_CHAR: (not implemented)");
        Ok(())
    }

    fn op_print_num(&mut self) -> Result<(), String> {
        println!("PRINT_NUM: (not implemented)");
        Ok(())
    }

    fn op_random(&mut self) -> Result<(), String> {
        println!("RANDOM: (not implemented)");
        Ok(())
    }

    fn op_push(&mut self) -> Result<(), String> {
        println!("PUSH: (not implemented)");
        Ok(())
    }

    fn op_pull(&mut self) -> Result<(), String> {
        println!("PULL: (not implemented)");
        Ok(())
    }

    fn op_split_window(&mut self) -> Result<(), String> {
        println!("SPLIT_WINDOW: (not implemented)");
        Ok(())
    }

    fn op_set_window(&mut self) -> Result<(), String> {
        println!("SET_WINDOW: (not implemented)");
        Ok(())
    }

    fn op_call_vs2(&mut self) -> Result<(), String> {
        println!("CALL_VS2: (not implemented)");
        Ok(())
    }

    fn op_erase_window(&mut self) -> Result<(), String> {
        println!("ERASE_WINDOW: (not implemented)");
        Ok(())
    }

    fn op_erase_line(&mut self) -> Result<(), String> {
        println!("ERASE_LINE: (not implemented)");
        Ok(())
    }

    fn op_set_cursor(&mut self) -> Result<(), String> {
        println!("SET_CURSOR: (not implemented)");
        Ok(())
    }

    fn op_get_cursor(&mut self) -> Result<(), String> {
        println!("GET_CURSOR: (not implemented)");
        Ok(())
    }

    fn op_set_text_style(&mut self) -> Result<(), String> {
        println!("SET_TEXT_STYLE: (not implemented)");
        Ok(())
    }

    fn op_buffer_mode(&mut self) -> Result<(), String> {
        println!("BUFFER_MODE: (not implemented)");
        Ok(())
    }

    fn op_output_stream(&mut self) -> Result<(), String> {
        println!("OUTPUT_STREAM: (not implemented)");
        Ok(())
    }

    fn op_input_stream(&mut self) -> Result<(), String> {
        println!("INPUT_STREAM: (not implemented)");
        Ok(())
    }

    fn op_sound_effect(&mut self) -> Result<(), String> {
        println!("SOUND_EFFECT: (not implemented)");
        Ok(())
    }

    fn op_read_char(&mut self) -> Result<(), String> {
        println!("READ_CHAR: (not implemented)");
        Ok(())
    }

    fn op_scan_table(&mut self) -> Result<(), String> {
        println!("SCAN_TABLE: (not implemented)");
        Ok(())
    }

    fn op_not_v4(&mut self) -> Result<(), String> {
        println!("NOT_V4: (not implemented)");
        Ok(())
    }

    fn op_call_vn(&mut self) -> Result<(), String> {
        println!("CALL_VN: (not implemented)");
        Ok(())
    }

    fn op_call_vn2(&mut self) -> Result<(), String> {
        println!("CALL_VN2: (not implemented)");
        Ok(())
    }

    fn op_tokenise(&mut self) -> Result<(), String> {
        println!("TOKENISE: (not implemented)");
        Ok(())
    }

    fn op_encode_text(&mut self) -> Result<(), String> {
        println!("ENCODE_TEXT: (not implemented)");
        Ok(())
    }

    fn op_copy_table(&mut self) -> Result<(), String> {
        println!("COPY_TABLE: (not implemented)");
        Ok(())
    }

    fn op_print_table(&mut self) -> Result<(), String> {
        println!("PRINT_TABLE: (not implemented)");
        Ok(())
    }

    fn op_check_arg_count(&mut self) -> Result<(), String> {
        println!("CHECK_ARG_COUNT: (not implemented)");
        Ok(())
    }

    fn op_call_2s(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        println!("CALL_2S: routine {} arg {} (not implemented)", operand1, operand2);
        Ok(())
    }

    fn op_call_2n(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        println!("CALL_2N: routine {} arg {} (not implemented)", operand1, operand2);
        Ok(())
    }

    fn op_set_colour(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        println!("SET_COLOUR: fg {} bg {} (not implemented)", operand1, operand2);
        Ok(())
    }

    fn op_throw(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        println!("THROW: value {} frame {} (not implemented)", operand1, operand2);
        Ok(())
    }

    pub fn return_from_routine(&mut self, value: u16) -> Result<(), String> {
        if let Some(frame) = self.call_stack.pop() {
            self.pc = frame.return_pc;
            self.local_vars = frame.local_vars;
            
            if let Some(result_var) = frame.result_var {
                self.store_variable(result_var, value)?;
            }
            
            println!("RETURN: Returned from routine with value {}, PC now {:#06x}", value, self.pc);
        } else {
            // Return from main routine - quit
            println!("RETURN: Returned from main routine, quitting");
            self.running = false;
        }
        Ok(())
    }

    fn handle_branch(&mut self, condition: bool) -> Result<(), String> {
        if let Some(branch_offset) = self.current_branch_offset {
            // Decode branch condition from offset sign (see instruction decoder)
            let (should_branch, actual_offset) = if branch_offset >= 0 {
                (condition, branch_offset)  // Branch on true
            } else {
                (!condition, -branch_offset - 1)  // Branch on false  
            };
            
            if should_branch {
                match actual_offset {
                    0 => {
                        // Branch offset 0 means RFALSE
                        println!("BRANCH: rfalse");
                        self.return_from_routine(0)?;
                    },
                    1 => {
                        // Branch offset 1 means RTRUE  
                        println!("BRANCH: rtrue");
                        self.return_from_routine(1)?;
                    },
                    _ => {
                        // Normal branch: offset is relative to PC after instruction
                        let new_pc = (self.pc as i32) + (actual_offset as i32) - 2;
                        if new_pc < 0 || new_pc >= self.memory.len() as i32 {
                            return Err(format!("Branch target out of bounds: {:#06x}", new_pc));
                        }
                        self.pc = new_pc as usize;
                        println!("BRANCH: jumping to {:#06x} (offset {})", self.pc, actual_offset);
                    }
                }
            } else {
                println!("BRANCH: condition false, not branching");
            }
        } else {
            return Err("handle_branch called on non-branch instruction".to_string());
        }
        Ok(())
    }

    // Helper methods for object table access
    fn get_object_table_addr(&self) -> usize {
        // Object table starts after property defaults table
        // Property defaults are at byte 10-11 of header (word address)
        let prop_defaults_addr = ((self.memory[10] as u16) << 8) | (self.memory[11] as u16);
        
        // Property defaults table is 31 words (62 bytes) for version 1-3
        (prop_defaults_addr + 62) as usize
    }
    
    fn get_object_addr(&self, obj_num: u16) -> Result<usize, String> {
        if obj_num == 0 {
            return Err("Object number 0 is invalid".to_string());
        }
        
        let obj_table_addr = self.get_object_table_addr();
        let obj_size = 9; // 9 bytes per object in version 1-3
        
        Ok(obj_table_addr + ((obj_num - 1) as usize * obj_size))
    }
    
    pub fn get_object_parent(&self, obj_num: u16) -> Result<u16, String> {
        if obj_num == 0 {
            return Ok(0);
        }
        
        let obj_addr = self.get_object_addr(obj_num)?;
        if obj_addr + 4 >= self.memory.len() {
            return Err("Object table access out of bounds".to_string());
        }
        
        Ok(self.memory[obj_addr + 4] as u16)
    }
    
    pub fn get_object_sibling(&self, obj_num: u16) -> Result<u16, String> {
        if obj_num == 0 {
            return Ok(0);
        }
        
        let obj_addr = self.get_object_addr(obj_num)?;
        if obj_addr + 5 >= self.memory.len() {
            return Err("Object table access out of bounds".to_string());
        }
        
        Ok(self.memory[obj_addr + 5] as u16)
    }
    
    pub fn get_object_child(&self, obj_num: u16) -> Result<u16, String> {
        if obj_num == 0 {
            return Ok(0);
        }
        
        let obj_addr = self.get_object_addr(obj_num)?;
        if obj_addr + 6 >= self.memory.len() {
            return Err("Object table access out of bounds".to_string());
        }
        
        Ok(self.memory[obj_addr + 6] as u16)
    }
    
    pub fn test_object_attribute(&self, obj_num: u16, attr_num: u16) -> Result<bool, String> {
        if obj_num == 0 {
            return Ok(false);
        }
        
        if attr_num >= 32 {
            return Err("Attribute number must be 0-31".to_string());
        }
        
        let obj_addr = self.get_object_addr(obj_num)?;
        if obj_addr + 3 >= self.memory.len() {
            return Err("Object table access out of bounds".to_string());
        }
        
        // Attributes are stored in first 4 bytes of object
        let byte_index = (attr_num / 8) as usize;
        let bit_index = 7 - (attr_num % 8);
        let byte_value = self.memory[obj_addr + byte_index];
        
        Ok((byte_value & (1 << bit_index)) != 0)
    }
}