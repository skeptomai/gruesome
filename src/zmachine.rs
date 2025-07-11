use std::collections::HashMap;
use std::io::{self, Write, Read, stdin, stdout};
use std::fs::File;
use std::path::Path;
use crate::game::GameFile;
use crate::instruction::{Instruction, OperandType, OperandCount};
use crate::util::ZTextReader;

// Helper structure for parsed words
#[derive(Debug, Clone)]
struct ParsedWord {
    text: String,
    position: usize, // Position in original text
}

// Window management structures
#[derive(Debug, Clone)]
pub struct Window {
    pub id: u16,
    pub top: u16,      // Top row (1-based)
    pub left: u16,     // Left column (1-based)
    pub height: u16,   // Height in rows
    pub width: u16,    // Width in columns
    pub cursor_row: u16, // Current cursor row (1-based)
    pub cursor_col: u16, // Current cursor column (1-based)
    pub text_style: u16, // Current text style flags
    pub wrap: bool,      // Text wrapping enabled
    pub scrolling: bool, // Scrolling enabled
}

pub struct ZMachine<'a> {
    pub game: &'a GameFile<'a>,
    pub memory: Vec<u8>,
    pub pc: usize,
    pub stack: Vec<u16>,
    pub call_stack: Vec<CallFrame>,
    pub global_vars: HashMap<u8, u16>,
    pub local_vars: [u16; 15],  // Max 15 local variables
    pub running: bool,
    pub loop_detection_counter: u32,  // To detect infinite loops
    pub operands_buffer: Vec<u16>,  // Buffer for current instruction operands
    pub current_branch_offset: Option<i16>,  // Branch offset for current instruction
    pub current_store_variable: u8,  // Store variable for current instruction
    pub random_seed: u32,  // Simple random number generator seed
    
    // I/O Stream management
    pub output_streams: Vec<u16>,  // Active output streams (1=screen, 2=transcript, 3=memory, 4=command)
    pub input_stream: u16,         // Active input stream (0=keyboard, 1=file)
    pub transcript_file: Option<File>,  // Transcript file for stream 2
    pub memory_stream_addr: Option<u16>,  // Memory address for stream 3
    pub memory_stream_data: Vec<u8>,      // Buffer for memory stream data
    
    // Window management
    pub windows: Vec<Window>,      // All available windows
    pub current_window: u16,       // Currently active window ID
    pub screen_height: u16,        // Screen height in rows
    pub screen_width: u16,         // Screen width in columns
}

// Save file format structure
#[derive(Debug)]
struct SaveFileHeader {
    version: u32,           // Save file format version
    game_version: u8,       // Z-machine version from game file
    pc: u32,               // Program counter
    stack_size: u32,       // Size of stack
    call_stack_size: u32,  // Size of call stack
    global_vars_size: u32, // Number of global variables
    dynamic_mem_size: u32, // Size of dynamic memory
    random_seed: u32,      // Random seed
}

const SAVE_FILE_VERSION: u32 = 1;
const SAVE_FILE_MAGIC: &[u8] = b"ZSAV";

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub return_pc: usize,
    pub local_vars: [u16; 15],
    pub num_locals: u8,
    pub result_var: Option<u8>,
}

impl<'a> ZMachine<'a> {
    pub fn new(game: &'a GameFile<'a>) -> Self {
        let memory = game.bytes().to_vec();
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
            loop_detection_counter: 0,
            operands_buffer: Vec::new(),
            current_branch_offset: None,
            current_store_variable: 0,
            random_seed: 12345, // Default seed
            
            // Initialize I/O streams
            output_streams: vec![1], // Screen output enabled by default
            input_stream: 0,         // Keyboard input by default
            transcript_file: None,
            memory_stream_addr: None,
            memory_stream_data: Vec::new(),
            
            // Initialize window system
            windows: Self::create_default_windows(),
            current_window: 0,       // Start with lower window (main window)
            screen_height: 24,       // Default screen size
            screen_width: 80,
        }
    }

    fn create_default_windows() -> Vec<Window> {
        vec![
            // Window 0: Lower window (main text window)
            Window {
                id: 0,
                top: 1,
                left: 1,
                height: 24,  // Full screen initially
                width: 80,
                cursor_row: 1,
                cursor_col: 1,
                text_style: 0,  // Normal text
                wrap: true,     // Wrapping enabled
                scrolling: true, // Scrolling enabled
            },
            // Window 1: Upper window (status line)
            Window {
                id: 1,
                top: 1,
                left: 1,
                height: 0,   // Initially no height (not split)
                width: 80,
                cursor_row: 1,
                cursor_col: 1,
                text_style: 0,
                wrap: false,  // No wrapping in status line
                scrolling: false, // No scrolling in status line
            },
        ]
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
    
    pub fn run_interactive(&mut self) -> Result<(), String> {
        while self.running {
            // Simple loop detection - if we've executed too many instructions without user input,
            // something is probably wrong
            self.loop_detection_counter += 1;
            if self.loop_detection_counter > 100000 {
                // Reset counter to allow continued execution but warn about the issue
                self.loop_detection_counter = 0;
                eprintln!("Warning: Detected possible infinite loop - resetting counter");
                eprintln!("PC: {:#x}, Stack size: {}, Call stack size: {}", self.pc, self.stack.len(), self.call_stack.len());
                return Err("Infinite loop detected - stopping execution".to_string());
            }
            
            let instruction = Instruction::decode(&self.memory, self.pc)?;
            
            // Debug instructions around SREAD
            if self.loop_detection_counter < 20 || (self.loop_detection_counter > 50000 && self.loop_detection_counter < 50010) {
                eprintln!("DEBUG: Executing instruction #{} at PC {:#x}: opcode={:#x}, form={:?}", 
                         self.loop_detection_counter, self.pc, instruction.opcode, instruction.form);
            }
            
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
        
        // Store the store_variable field for instructions that need it
        self.current_store_variable = instruction.store_variable.unwrap_or(0);
        
        match instruction.operand_count {
            OperandCount::Op0 => self.execute_0op(instruction),
            OperandCount::Op1 => self.execute_1op(instruction),
            OperandCount::Op2 => self.execute_2op(instruction),
            OperandCount::Var => {
                // Extended instructions use Var operand count but need special handling
                if instruction.form == crate::instruction::InstructionForm::Extended {
                    self.execute_extended(instruction)
                } else {
                    self.execute_var(instruction)
                }
            }
        }
    }

    fn execute_0op(&mut self, instruction: Instruction) -> Result<(), String> {
        match instruction.opcode {
            0x00 => self.op_rtrue(),
            0x01 => self.op_rfalse(),
            0x02 => self.op_print(),
            0x03 => self.op_print_ret(),
            0x04 => self.op_show_status(),
            0x05 => self.op_save(),
            0x06 => self.op_restore(),
            0x07 => self.op_restart(),
            0x08 => self.op_ret_popped(),
            0x09 => self.op_catch(),
            0x0A => self.op_quit(),
            0x0B => self.op_new_line(),
            0x0D => self.op_verify(),
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
                // 2OP opcode 0 is reserved/unused - treat as NOP
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
            _ => Err(format!("Unknown 2OP instruction: {:#04x}", instruction.opcode)),
        }
    }

    fn execute_extended(&mut self, instruction: Instruction) -> Result<(), String> {
        // Extended instructions (0xBE prefix)
        match instruction.opcode {
            0x00 => self.op_save_undo(),
            0x01 => self.op_restore_undo(),
            0x02 => self.op_log_shift(),
            0x03 => self.op_art_shift(),
            0x04 => self.op_set_font(),
            0x05 => self.op_draw_picture(),
            0x06 => self.op_picture_data(),
            0x07 => self.op_erase_picture(),
            0x08 => self.op_set_margins(),
            0x09 => self.op_save_undo(),  // SAVE_UNDO (duplicate of 0x00)
            0x0A => self.op_restore_undo(), // RESTORE_UNDO (duplicate of 0x01)
            0x0B => self.op_print_unicode(),
            0x0C => self.op_check_unicode(),
            0x0D => self.op_set_true_colour(),
            0x10 => self.op_move_window(),
            0x11 => self.op_window_size(),
            0x12 => self.op_window_style(),
            0x13 => self.op_get_wind_prop(),
            0x14 => self.op_scroll_window(),
            0x15 => self.op_pop_stack(),
            0x16 => self.op_read_mouse(),
            0x17 => self.op_mouse_window(),
            0x18 => self.op_push_stack(),
            0x19 => self.op_put_wind_prop(),
            0x1A => self.op_print_form(),
            0x1B => self.op_make_menu(),
            0x1C => self.op_picture_table(),
            0x1D => self.op_buffer_screen(),
            _ => Err(format!("Unknown extended instruction: {:#04x}", instruction.opcode)),
        }
    }

    fn execute_var(&mut self, instruction: Instruction) -> Result<(), String> {
        let result = match instruction.opcode {
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
        };
        result
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
        if var == 125 || var == 126 {
        }
        if var == 0 {
            // Stack
            if self.stack.len() > 100000 {
                eprintln!("Stack overflow at PC {:#x}, call stack size: {}", self.pc, self.call_stack.len());
                return Err("Stack overflow - too many items on stack".to_string());
            }
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

    pub fn op_save(&mut self) -> Result<(), String> {
        // SAVE instruction - saves game state to file
        let save_path = "game.sav";
        
        match self.save_to_file(save_path) {
            Ok(_) => {
                println!("SAVE: Game state saved to {}", save_path);
                
                // In Z-machine, SAVE branches on success/failure
                if let Some(_) = self.current_branch_offset {
                    self.handle_branch(true)?; // Branch on success
                }
                Ok(())
            }
            Err(e) => {
                println!("SAVE: Failed to save game state: {}", e);
                
                if let Some(_) = self.current_branch_offset {
                    self.handle_branch(false)?; // Branch on failure
                }
                Ok(())
            }
        }
    }
    
    fn save_to_file(&self, path: &str) -> Result<(), String> {
        let mut file = File::create(path).map_err(|e| format!("Failed to create save file: {}", e))?;
        
        // Write magic bytes
        file.write_all(SAVE_FILE_MAGIC).map_err(|e| format!("Failed to write magic: {}", e))?;
        
        // Prepare header
        let dynamic_mem_size = self.game.header().base_static_mem;
        let header = SaveFileHeader {
            version: SAVE_FILE_VERSION,
            game_version: self.game.header().version,
            pc: self.pc as u32,
            stack_size: self.stack.len() as u32,
            call_stack_size: self.call_stack.len() as u32,
            global_vars_size: self.global_vars.len() as u32,
            dynamic_mem_size: dynamic_mem_size as u32,
            random_seed: self.random_seed,
        };
        
        // Write header
        self.write_header(&mut file, &header)?;
        
        // Write dynamic memory (only the part that can change)
        let dynamic_memory = &self.memory[0..dynamic_mem_size.min(self.memory.len())];
        file.write_all(dynamic_memory).map_err(|e| format!("Failed to write dynamic memory: {}", e))?;
        
        // Write stack
        for &value in &self.stack {
            file.write_all(&value.to_le_bytes()).map_err(|e| format!("Failed to write stack: {}", e))?;
        }
        
        // Write call stack
        for frame in &self.call_stack {
            file.write_all(&(frame.return_pc as u32).to_le_bytes()).map_err(|e| format!("Failed to write call frame: {}", e))?;
            file.write_all(&frame.num_locals.to_le_bytes()).map_err(|e| format!("Failed to write call frame: {}", e))?;
            file.write_all(&frame.result_var.unwrap_or(0).to_le_bytes()).map_err(|e| format!("Failed to write call frame: {}", e))?;
            
            // Write local variables
            for &var in &frame.local_vars {
                file.write_all(&var.to_le_bytes()).map_err(|e| format!("Failed to write local vars: {}", e))?;
            }
        }
        
        // Write global variables
        for (&key, &value) in &self.global_vars {
            file.write_all(&key.to_le_bytes()).map_err(|e| format!("Failed to write global vars: {}", e))?;
            file.write_all(&value.to_le_bytes()).map_err(|e| format!("Failed to write global vars: {}", e))?;
        }
        
        // Write current local variables
        for &var in &self.local_vars {
            file.write_all(&var.to_le_bytes()).map_err(|e| format!("Failed to write local vars: {}", e))?;
        }
        
        file.flush().map_err(|e| format!("Failed to flush save file: {}", e))?;
        Ok(())
    }
    
    fn write_header(&self, file: &mut File, header: &SaveFileHeader) -> Result<(), String> {
        file.write_all(&header.version.to_le_bytes()).map_err(|e| format!("Failed to write header: {}", e))?;
        file.write_all(&header.game_version.to_le_bytes()).map_err(|e| format!("Failed to write header: {}", e))?;
        file.write_all(&header.pc.to_le_bytes()).map_err(|e| format!("Failed to write header: {}", e))?;
        file.write_all(&header.stack_size.to_le_bytes()).map_err(|e| format!("Failed to write header: {}", e))?;
        file.write_all(&header.call_stack_size.to_le_bytes()).map_err(|e| format!("Failed to write header: {}", e))?;
        file.write_all(&header.global_vars_size.to_le_bytes()).map_err(|e| format!("Failed to write header: {}", e))?;
        file.write_all(&header.dynamic_mem_size.to_le_bytes()).map_err(|e| format!("Failed to write header: {}", e))?;
        file.write_all(&header.random_seed.to_le_bytes()).map_err(|e| format!("Failed to write header: {}", e))?;
        Ok(())
    }
    
    pub fn op_restore(&mut self) -> Result<(), String> {
        // RESTORE instruction - restores game state from file
        let save_path = "game.sav";
        
        match self.restore_from_file(save_path) {
            Ok(_) => {
                println!("RESTORE: Game state restored from {}", save_path);
                
                // In Z-machine, RESTORE branches on success/failure
                if let Some(_) = self.current_branch_offset {
                    self.handle_branch(true)?; // Branch on success
                }
                Ok(())
            }
            Err(e) => {
                println!("RESTORE: Failed to restore game state: {}", e);
                
                if let Some(_) = self.current_branch_offset {
                    self.handle_branch(false)?; // Branch on failure
                }
                Ok(())
            }
        }
    }
    
    fn restore_from_file(&mut self, path: &str) -> Result<(), String> {
        if !Path::new(path).exists() {
            return Err("Save file not found".to_string());
        }
        
        let mut file = File::open(path).map_err(|e| format!("Failed to open save file: {}", e))?;
        
        // Read and verify magic bytes
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic).map_err(|e| format!("Failed to read magic: {}", e))?;
        if magic != SAVE_FILE_MAGIC {
            return Err("Invalid save file format".to_string());
        }
        
        // Read header
        let header = self.read_header(&mut file)?;
        
        // Verify compatibility
        if header.version != SAVE_FILE_VERSION {
            return Err(format!("Unsupported save file version: {}", header.version));
        }
        
        if header.game_version != self.game.header().version {
            return Err("Save file is for a different game version".to_string());
        }
        
        // Restore dynamic memory
        let dynamic_mem_size = header.dynamic_mem_size as usize;
        if dynamic_mem_size > self.memory.len() {
            return Err("Save file dynamic memory size exceeds current memory".to_string());
        }
        
        file.read_exact(&mut self.memory[0..dynamic_mem_size])
            .map_err(|e| format!("Failed to read dynamic memory: {}", e))?;
        
        // Restore stack
        self.stack.clear();
        for _ in 0..header.stack_size {
            let mut bytes = [0u8; 2];
            file.read_exact(&mut bytes).map_err(|e| format!("Failed to read stack: {}", e))?;
            self.stack.push(u16::from_le_bytes(bytes));
        }
        
        // Restore call stack
        self.call_stack.clear();
        for _ in 0..header.call_stack_size {
            let mut return_pc_bytes = [0u8; 4];
            file.read_exact(&mut return_pc_bytes).map_err(|e| format!("Failed to read call frame: {}", e))?;
            let return_pc = u32::from_le_bytes(return_pc_bytes) as usize;
            
            let mut num_locals_bytes = [0u8; 1];
            file.read_exact(&mut num_locals_bytes).map_err(|e| format!("Failed to read call frame: {}", e))?;
            let num_locals = num_locals_bytes[0];
            
            let mut result_var_bytes = [0u8; 1];
            file.read_exact(&mut result_var_bytes).map_err(|e| format!("Failed to read call frame: {}", e))?;
            let result_var = if result_var_bytes[0] == 0 { None } else { Some(result_var_bytes[0]) };
            
            let mut local_vars = [0u16; 15];
            for i in 0..15 {
                let mut bytes = [0u8; 2];
                file.read_exact(&mut bytes).map_err(|e| format!("Failed to read local vars: {}", e))?;
                local_vars[i] = u16::from_le_bytes(bytes);
            }
            
            self.call_stack.push(CallFrame {
                return_pc,
                local_vars,
                num_locals,
                result_var,
            });
        }
        
        // Restore global variables
        self.global_vars.clear();
        for _ in 0..header.global_vars_size {
            let mut key_bytes = [0u8; 1];
            file.read_exact(&mut key_bytes).map_err(|e| format!("Failed to read global vars: {}", e))?;
            let key = key_bytes[0];
            
            let mut value_bytes = [0u8; 2];
            file.read_exact(&mut value_bytes).map_err(|e| format!("Failed to read global vars: {}", e))?;
            let value = u16::from_le_bytes(value_bytes);
            
            self.global_vars.insert(key, value);
        }
        
        // Restore current local variables
        for i in 0..15 {
            let mut bytes = [0u8; 2];
            file.read_exact(&mut bytes).map_err(|e| format!("Failed to read local vars: {}", e))?;
            self.local_vars[i] = u16::from_le_bytes(bytes);
        }
        
        // Restore other state
        self.pc = header.pc as usize;
        self.random_seed = header.random_seed;
        
        Ok(())
    }
    
    fn read_header(&self, file: &mut File) -> Result<SaveFileHeader, String> {
        let mut version_bytes = [0u8; 4];
        file.read_exact(&mut version_bytes).map_err(|e| format!("Failed to read header: {}", e))?;
        let version = u32::from_le_bytes(version_bytes);
        
        let mut game_version_bytes = [0u8; 1];
        file.read_exact(&mut game_version_bytes).map_err(|e| format!("Failed to read header: {}", e))?;
        let game_version = game_version_bytes[0];
        
        let mut pc_bytes = [0u8; 4];
        file.read_exact(&mut pc_bytes).map_err(|e| format!("Failed to read header: {}", e))?;
        let pc = u32::from_le_bytes(pc_bytes);
        
        let mut stack_size_bytes = [0u8; 4];
        file.read_exact(&mut stack_size_bytes).map_err(|e| format!("Failed to read header: {}", e))?;
        let stack_size = u32::from_le_bytes(stack_size_bytes);
        
        let mut call_stack_size_bytes = [0u8; 4];
        file.read_exact(&mut call_stack_size_bytes).map_err(|e| format!("Failed to read header: {}", e))?;
        let call_stack_size = u32::from_le_bytes(call_stack_size_bytes);
        
        let mut global_vars_size_bytes = [0u8; 4];
        file.read_exact(&mut global_vars_size_bytes).map_err(|e| format!("Failed to read header: {}", e))?;
        let global_vars_size = u32::from_le_bytes(global_vars_size_bytes);
        
        let mut dynamic_mem_size_bytes = [0u8; 4];
        file.read_exact(&mut dynamic_mem_size_bytes).map_err(|e| format!("Failed to read header: {}", e))?;
        let dynamic_mem_size = u32::from_le_bytes(dynamic_mem_size_bytes);
        
        let mut random_seed_bytes = [0u8; 4];
        file.read_exact(&mut random_seed_bytes).map_err(|e| format!("Failed to read header: {}", e))?;
        let random_seed = u32::from_le_bytes(random_seed_bytes);
        
        Ok(SaveFileHeader {
            version,
            game_version,
            pc,
            stack_size,
            call_stack_size,
            global_vars_size,
            dynamic_mem_size,
            random_seed,
        })
    }
    
    pub fn op_restart(&mut self) -> Result<(), String> {
        // RESTART instruction - restarts the game
        eprintln!("DEBUG: RESTART called at PC {:#x}, stack size: {}", self.pc, self.stack.len());
        println!("RESTART: Restarting game");
        
        // Reset Z-machine state to initial conditions
        self.pc = self.game.header().initial_pc;
        self.stack.clear();
        self.call_stack.clear();
        self.global_vars.clear();
        self.local_vars = [0; 15];
        self.random_seed = 12345;
        
        // Reset dynamic memory to initial state
        let initial_memory = self.game.bytes().to_vec();
        let dynamic_end = self.game.header().base_static_mem;
        
        for i in 0..dynamic_end.min(self.memory.len()) {
            self.memory[i] = initial_memory[i];
        }
        
        Ok(())
    }
    
    pub fn op_verify(&mut self) -> Result<(), String> {
        // VERIFY instruction - checks game file integrity
        match self.verify_game_file() {
            Ok(true) => {
                println!("VERIFY: Game file verified successfully");
                
                // In Z-machine, VERIFY branches on success/failure
                if let Some(_) = self.current_branch_offset {
                    self.handle_branch(true)?; // Branch on success
                }
                Ok(())
            }
            Ok(false) => {
                println!("VERIFY: Game file verification failed");
                
                if let Some(_) = self.current_branch_offset {
                    self.handle_branch(false)?; // Branch on failure
                }
                Ok(())
            }
            Err(e) => {
                println!("VERIFY: Error during verification: {}", e);
                
                if let Some(_) = self.current_branch_offset {
                    self.handle_branch(false)?; // Branch on failure
                }
                Ok(())
            }
        }
    }
    
    fn verify_game_file(&self) -> Result<bool, String> {
        // Get the expected checksum from the header
        let expected_checksum = self.game.header().checksum_file;
        
        // Calculate actual checksum of the file
        let actual_checksum = self.calculate_checksum()?;
        
        // Compare checksums
        Ok(expected_checksum == actual_checksum)
    }
    
    fn calculate_checksum(&self) -> Result<usize, String> {
        // Z-machine checksum is calculated over the entire file except the checksum field itself
        // The checksum field is at offset 0x1C in the header
        
        let file_bytes = self.game.bytes();
        let mut checksum: u32 = 0;
        
        // Sum all bytes except the checksum field (bytes 0x1C and 0x1D)
        for (i, &byte) in file_bytes.iter().enumerate() {
            if i != 0x1C && i != 0x1D {
                checksum = checksum.wrapping_add(byte as u32);
            }
        }
        
        // The checksum is the low 16 bits
        Ok((checksum & 0xFFFF) as usize)
    }

    fn op_print(&mut self) -> Result<(), String> {
        // Print immediate string following instruction
        let (text, _length) = self.read_zstring_inline()?;
        self.write_output(&text)?;
        Ok(())
    }

    fn op_print_ret(&mut self) -> Result<(), String> {
        self.op_print()?;
        self.write_output("\n")?;
        self.return_from_routine(1)
    }

    fn op_show_status(&mut self) -> Result<(), String> {
        // SHOW_STATUS displays the status line (location, score, moves, etc.)
        // This is critical for Zork I and other games
        
        // In Zork I, global variables contain:
        // - Location (typically global 0 or 1)
        // - Score (typically global 1 or 2)  
        // - Moves (typically global 2 or 3)
        
        // For now, let's read from standard global variables
        let location_obj = self.read_global_variable(0).unwrap_or(0);
        let score = self.read_global_variable(1).unwrap_or(0) as i16;
        let moves = self.read_global_variable(2).unwrap_or(0);
        
        // Get location name from object
        let location_name = if location_obj > 0 {
            match self.get_object_name(location_obj) {
                Ok(name) => name,
                Err(_) => format!("Location {}", location_obj),
            }
        } else {
            "Unknown".to_string()
        };
        
        // Format status line
        let status_line = format!("{}    Score: {}  Moves: {}", location_name, score, moves);
        
        // Switch to upper window (status line)
        let current_window = self.current_window;
        self.current_window = 1;
        
        // Make sure upper window is available
        let upper_window_needs_split = if let Some(upper_window) = self.windows.get(1) {
            upper_window.height == 0
        } else {
            false
        };
        
        if upper_window_needs_split {
            // Split window to make room for status line
            if let Some(upper_window) = self.windows.get_mut(1) {
                upper_window.height = 1;
                upper_window.width = 80;
                upper_window.cursor_row = 1;
                upper_window.cursor_col = 1;
            }
            
            // Adjust lower window
            if let Some(lower_window) = self.windows.get_mut(0) {
                lower_window.top = 2;
                lower_window.height = self.screen_height - 1;
            }
        }
        
        // Clear status line and print
        if let Some(upper_window) = self.windows.get_mut(1) {
            upper_window.cursor_row = 1;
            upper_window.cursor_col = 1;
        }
        
        // Output to status line
        self.write_output(&format!("\x1b[1;1H\x1b[K{}", status_line))?;
        
        // Switch back to original window
        self.current_window = current_window;
        
        Ok(())
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
        self.store_variable(self.current_store_variable, frame_count)?;
        Ok(())
    }

    fn op_quit(&mut self) -> Result<(), String> {
        println!("Game quit.");
        self.running = false;
        Ok(())
    }

    fn op_new_line(&mut self) -> Result<(), String> {
        self.write_output("\n")?;
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
        self.store_variable(self.current_store_variable, sibling)?;
        
        // Branch if sibling is not 0 (object has a sibling)
        let condition = sibling != 0;
        self.handle_branch(condition)
    }

    pub fn op_get_child(&mut self, operand: u16) -> Result<(), String> {
        let obj_num = operand;
        let child = self.get_object_child(obj_num)?;
        
        // Store the child object number
        self.store_variable(self.current_store_variable, child)?;
        
        // Branch if child is not 0 (object has a child)
        let condition = child != 0;
        self.handle_branch(condition)
    }

    pub fn op_get_parent(&mut self, operand: u16) -> Result<(), String> {
        let obj_num = operand;
        let parent = self.get_object_parent(obj_num)?;
        
        // Store the parent object number (LOAD-style instruction)
        self.store_variable(self.current_store_variable, parent)
    }

    pub fn op_get_prop_len(&mut self, operand: u16) -> Result<(), String> {
        let prop_addr = operand as usize;
        
        if prop_addr == 0 {
            // GET_PROP_LEN 0 returns 0
            self.store_variable(self.current_store_variable, 0)?;
            return Ok(());
        }
        
        if prop_addr >= self.memory.len() {
            return Err("Property address out of bounds".to_string());
        }
        
        // The property address points to the property data
        // The size byte is immediately before the data
        if prop_addr == 0 {
            return Err("Invalid property address".to_string());
        }
        
        let size_byte = self.memory[prop_addr - 1];
        
        if size_byte == 0 {
            return Err("Invalid property size byte".to_string());
        }
        
        // Extract size from upper 3 bits (v1-3 format)
        let property_size = (size_byte >> 5) + 1;
        
        self.store_variable(self.current_store_variable, property_size as u16)
    }

    pub fn op_inc(&mut self, operand: u16) -> Result<(), String> {
        let var_num = operand as u8;
        
        // Get current value of variable
        let current_value = if var_num == 0 {
            if self.stack.is_empty() {
                return Err("Stack underflow in inc".to_string());
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
        self.store_variable(var_num, new_value)
    }

    pub fn op_dec(&mut self, operand: u16) -> Result<(), String> {
        let var_num = operand as u8;
        
        // Get current value of variable
        let current_value = if var_num == 0 {
            if self.stack.is_empty() {
                return Err("Stack underflow in dec".to_string());
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
        self.store_variable(var_num, new_value)
    }

    pub fn op_print_addr(&mut self, operand: u16) -> Result<(), String> {
        // Print string at given byte address
        let (text, _) = self.read_zstring_at_address(operand as usize)?;
        self.write_output(&text)?;
        Ok(())
    }

    pub fn op_call_1s(&mut self, operand: u16) -> Result<(), String> {
        // CALL_1S: Call routine with 1 argument, store result
        let routine_addr = operand;
        
        // Set up operands buffer for the generic call implementation
        self.operands_buffer = vec![routine_addr];
        
        // Call the generic call implementation
        self.op_call()
    }

    pub fn op_remove_obj(&mut self, operand: u16) -> Result<(), String> {
        let obj_num = operand;
        
        if obj_num == 0 {
            return Ok(()); // Removing object 0 is a no-op
        }
        
        // Get current parent of the object
        let current_parent = self.get_object_parent(obj_num)?;
        
        if current_parent == 0 {
            return Ok(()); // Object has no parent, nothing to do
        }
        
        // Remove object from parent's child list
        // Need to update sibling chain
        let obj_sibling = self.get_object_sibling(obj_num)?;
        let parent_child = self.get_object_child(current_parent)?;
        
        if parent_child == obj_num {
            // Object is the first child - set parent's child to object's sibling
            self.set_object_child(current_parent, obj_sibling)?;
        } else {
            // Object is not the first child - find previous sibling and update its sibling pointer
            let mut current = parent_child;
            
            while current != 0 {
                let next_sibling = self.get_object_sibling(current)?;
                if next_sibling == obj_num {
                    // Found the previous sibling - update its sibling pointer
                    self.set_object_sibling(current, obj_sibling)?;
                    break;
                }
                current = next_sibling;
            }
        }
        
        // Clear the object's parent and sibling
        self.set_object_parent(obj_num, 0)?;
        self.set_object_sibling(obj_num, 0)?;
        
        Ok(())
    }

    pub fn op_print_obj(&mut self, operand: u16) -> Result<(), String> {
        // Print object name (stored in property table)
        if operand == 0 {
            return Err("Cannot print name of object 0".to_string());
        }
        
        match self.get_object_name(operand) {
            Ok(name) => {
                self.write_output(&name)?;
                Ok(())
            }
            Err(e) => Err(e),
        }
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

    pub fn op_print_paddr(&mut self, operand: u16) -> Result<(), String> {
        // Print string at given packed address
        let byte_addr = self.convert_packed_address(operand);
        let (text, _) = self.read_zstring_at_address(byte_addr)?;
        self.write_output(&text)?;
        Ok(())
    }

    pub fn op_load(&mut self, operand: u16) -> Result<(), String> {
        let var_num = operand as u8;
        
        // Get value of variable
        let value = if var_num == 0 {
            if self.stack.is_empty() {
                return Err("Stack underflow in load".to_string());
            }
            self.stack[self.stack.len() - 1]  // Peek at top without popping
        } else if var_num <= 15 {
            self.local_vars[(var_num - 1) as usize]
        } else {
            self.global_vars.get(&var_num).copied().unwrap_or(0)
        };
        
        // Store the loaded value (result of LOAD instruction)
        self.store_variable(self.current_store_variable, value)
    }

    fn op_not(&mut self, operand: u16) -> Result<(), String> {
        let result = !operand;
        self.store_variable(self.current_store_variable, result)?;
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
        self.store_variable(self.current_store_variable, result)?;
        Ok(())
    }

    fn op_and(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let result = operand1 & operand2;
        self.store_variable(self.current_store_variable, result)?;
        Ok(())
    }

    pub fn op_test_attr(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let obj_num = operand1;
        let attr_num = operand2;
        
        // Test if the object has the specified attribute
        let condition = self.test_object_attribute(obj_num, attr_num)?;
        self.handle_branch(condition)
    }

    pub fn op_set_attr(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let obj_num = operand1;
        let attr_num = operand2;
        
        if obj_num == 0 {
            return Ok(()); // Setting attributes on object 0 is a no-op
        }
        
        if attr_num >= 32 {
            return Err(format!("Attribute number must be 0-31, got {}", attr_num));
        }
        
        let obj_addr = self.get_object_addr(obj_num)?;
        if obj_addr + 3 >= self.memory.len() {
            return Err("Object table access out of bounds".to_string());
        }
        
        // Attributes are stored in first 4 bytes of object
        let byte_index = (attr_num / 8) as usize;
        let bit_index = 7 - (attr_num % 8);
        
        // Set the bit
        self.memory[obj_addr + byte_index] |= 1 << bit_index;
        
        Ok(())
    }

    pub fn op_clear_attr(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let obj_num = operand1;
        let attr_num = operand2;
        
        if obj_num == 0 {
            return Ok(()); // Clearing attributes on object 0 is a no-op
        }
        
        if attr_num >= 32 {
            return Err(format!("Attribute number must be 0-31, got {}", attr_num));
        }
        
        let obj_addr = self.get_object_addr(obj_num)?;
        if obj_addr + 3 >= self.memory.len() {
            return Err("Object table access out of bounds".to_string());
        }
        
        // Attributes are stored in first 4 bytes of object
        let byte_index = (attr_num / 8) as usize;
        let bit_index = 7 - (attr_num % 8);
        
        // Clear the bit
        self.memory[obj_addr + byte_index] &= !(1 << bit_index);
        
        Ok(())
    }

    fn op_store(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        self.store_variable(operand1 as u8, operand2)
    }

    pub fn op_insert_obj(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let obj_num = operand1;
        let dest_parent = operand2;
        
        if obj_num == 0 {
            return Ok(()); // Inserting object 0 is a no-op
        }
        
        if dest_parent == 0 {
            return Ok(()); // Inserting into object 0 is a no-op
        }
        
        // First, remove the object from its current parent
        self.op_remove_obj(obj_num)?;
        
        // Get the current first child of the destination parent
        let current_first_child = self.get_object_child(dest_parent)?;
        
        // Set the object as the new first child of the destination parent
        self.set_object_child(dest_parent, obj_num)?;
        
        // Set the object's parent to the destination parent
        self.set_object_parent(obj_num, dest_parent)?;
        
        // Set the object's sibling to the previous first child
        self.set_object_sibling(obj_num, current_first_child)?;
        
        Ok(())
    }

    pub fn op_loadw(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let addr = operand1 as usize + (operand2 as usize * 2);
        if addr + 1 < self.memory.len() {
            let value = ((self.memory[addr] as u16) << 8) | (self.memory[addr + 1] as u16);
            self.store_variable(self.current_store_variable, value)?;
        }
        Ok(())
    }

    pub fn op_loadb(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let addr = operand1 as usize + operand2 as usize;
        if addr < self.memory.len() {
            let value = self.memory[addr] as u16;
            
            
            self.store_variable(self.current_store_variable, value)?;
        }
        Ok(())
    }

    pub fn op_get_prop(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let obj_num = operand1;
        let prop_num = operand2 as u8;
        
        if obj_num == 0 {
            return Err("Cannot get property of object 0".to_string());
        }
        
        if prop_num == 0 || prop_num > 31 {
            return Err("Property number must be 1-31".to_string());
        }
        
        // Try to find the property on the object
        match self.find_property(obj_num, prop_num)? {
            Some((prop_data_addr, prop_size)) => {
                // Property found - read its value
                let value = match prop_size {
                    1 => {
                        // 1-byte property
                        if prop_data_addr >= self.memory.len() {
                            return Err("Property data out of bounds".to_string());
                        }
                        self.memory[prop_data_addr] as u16
                    },
                    2 => {
                        // 2-byte property (big-endian)
                        if prop_data_addr + 1 >= self.memory.len() {
                            return Err("Property data out of bounds".to_string());
                        }
                        ((self.memory[prop_data_addr] as u16) << 8) | (self.memory[prop_data_addr + 1] as u16)
                    },
                    _ => {
                        return Err(format!("GET_PROP can only read 1 or 2 byte properties, found {} bytes", prop_size));
                    }
                };
                
                self.store_variable(self.current_store_variable, value)
            },
            None => {
                // Property not found - return default value
                let default_value = self.get_property_default(prop_num)?;
                self.store_variable(self.current_store_variable, default_value)
            }
        }
    }

    pub fn op_get_prop_addr(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let obj_num = operand1;
        let prop_num = operand2 as u8;
        
        if obj_num == 0 {
            // GET_PROP_ADDR of object 0 returns 0
            self.store_variable(self.current_store_variable, 0)?;
            return Ok(());
        }
        
        if prop_num == 0 || prop_num > 31 {
            return Err("Property number must be 1-31".to_string());
        }
        
        // Try to find the property on the object
        match self.find_property(obj_num, prop_num)? {
            Some((prop_data_addr, _prop_size)) => {
                // Property found - return its data address
                self.store_variable(self.current_store_variable, prop_data_addr as u16)
            },
            None => {
                // Property not found - return 0
                self.store_variable(self.current_store_variable, 0)
            }
        }
    }

    pub fn op_get_next_prop(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let obj_num = operand1;
        let prop_num = operand2 as u8;
        
        if obj_num == 0 {
            return Err("Cannot get next property of object 0".to_string());
        }
        
        let props_addr = self.get_object_properties_addr(obj_num)?;
        
        if props_addr >= self.memory.len() {
            return Err("Property table address out of bounds".to_string());
        }
        
        // Skip object description - first byte is description length in words
        let desc_len = self.memory[props_addr] as usize;
        let mut cursor = props_addr + 1 + (desc_len * 2);
        
        if prop_num == 0 {
            // Return first property number
            if cursor < self.memory.len() {
                let size_byte = self.memory[cursor];
                if size_byte != 0 {
                    let first_prop_num = size_byte & 0x1F;
                    self.store_variable(self.current_store_variable, first_prop_num as u16)?;
                    return Ok(());
                }
            }
            // No properties
            self.store_variable(self.current_store_variable, 0)?;
            return Ok(());
        }
        
        // Search for the specified property, then return the next one
        let mut found_target = false;
        
        while cursor < self.memory.len() {
            let size_byte = self.memory[cursor];
            
            if size_byte == 0 {
                // End of property list
                break;
            }
            
            let property_num = size_byte & 0x1F;
            let property_size = (size_byte >> 5) + 1;
            
            if found_target {
                // Return this property number (the one after the target)
                self.store_variable(self.current_store_variable, property_num as u16)?;
                return Ok(());
            }
            
            if property_num == prop_num {
                // Found the target property
                found_target = true;
            }
            
            // Move to next property
            cursor += 1 + property_size as usize;
        }
        
        // Target property was last or not found - return 0
        self.store_variable(self.current_store_variable, 0)
    }

    fn op_add(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let result = operand1.wrapping_add(operand2);
        self.store_variable(self.current_store_variable, result)?;
        Ok(())
    }

    fn op_sub(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let result = operand1.wrapping_sub(operand2);
        self.store_variable(self.current_store_variable, result)?;
        Ok(())
    }

    fn op_mul(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let result = operand1.wrapping_mul(operand2);
        self.store_variable(self.current_store_variable, result)?;
        Ok(())
    }

    fn op_div(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        if operand2 == 0 {
            return Err("Division by zero".to_string());
        }
        let result = (operand1 as i16) / (operand2 as i16);
        self.store_variable(self.current_store_variable, result as u16)?;
        Ok(())
    }

    fn op_mod(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        if operand2 == 0 {
            return Err("Modulo by zero".to_string());
        }
        let result = (operand1 as i16) % (operand2 as i16);
        self.store_variable(self.current_store_variable, result as u16)?;
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
            self.store_variable(self.current_store_variable, 0)?; // Store 0 in the specified variable
            return Ok(());
        }
        
        
        // Convert packed address to byte address
        let version = self.game.version();
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
            return Err(format!("Too many local variables in routine: {} at address {:#06x} (routine_addr={:#06x})", num_locals, byte_addr, routine_addr));
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
            result_var: Some(self.current_store_variable), // Store result in specified variable
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
        
        // println!("CALL: Calling routine at {:#06x} with {} locals, {} args", 
        //         byte_addr, num_locals, args.len());
        
        Ok(())
    }

    pub fn op_storew(&mut self) -> Result<(), String> {
        // STOREW takes 3 operands: array-address, word-index, value
        if self.operands_buffer.len() < 3 {
            return Err("STOREW instruction missing operands".to_string());
        }
        
        let array_addr = self.operands_buffer[0] as usize;
        let word_index = self.operands_buffer[1] as usize;
        let value = self.operands_buffer[2];
        
        let byte_addr = array_addr + (word_index * 2);
        
        if byte_addr + 1 >= self.memory.len() {
            return Err("STOREW address out of bounds".to_string());
        }
        
        // Store word in big-endian format
        self.memory[byte_addr] = (value >> 8) as u8;
        self.memory[byte_addr + 1] = (value & 0xFF) as u8;
        
        Ok(())
    }

    pub fn op_storeb(&mut self) -> Result<(), String> {
        // STOREB takes 3 operands: array-address, byte-index, value
        if self.operands_buffer.len() < 3 {
            return Err("STOREB instruction missing operands".to_string());
        }
        
        let array_addr = self.operands_buffer[0] as usize;
        let byte_index = self.operands_buffer[1] as usize;
        let value = self.operands_buffer[2];
        
        let byte_addr = array_addr + byte_index;
        
        if byte_addr >= self.memory.len() {
            return Err("STOREB address out of bounds".to_string());
        }
        
        // Store byte (only low 8 bits of value)
        self.memory[byte_addr] = (value & 0xFF) as u8;
        
        Ok(())
    }

    pub fn op_put_prop(&mut self) -> Result<(), String> {
        // PUT_PROP takes 3 operands: object, property, value
        if self.operands_buffer.len() < 3 {
            return Err("PUT_PROP instruction missing operands".to_string());
        }
        
        let obj_num = self.operands_buffer[0];
        let prop_num = self.operands_buffer[1] as u8;
        let value = self.operands_buffer[2];
        
        if obj_num == 0 {
            return Err("Cannot set property of object 0".to_string());
        }
        
        if prop_num == 0 || prop_num > 31 {
            return Err("Property number must be 1-31".to_string());
        }
        
        // Find the property on the object
        match self.find_property(obj_num, prop_num)? {
            Some((prop_data_addr, prop_size)) => {
                // Property found - write the new value
                match prop_size {
                    1 => {
                        // 1-byte property - store low byte only
                        if prop_data_addr >= self.memory.len() {
                            return Err("Property data out of bounds".to_string());
                        }
                        self.memory[prop_data_addr] = (value & 0xFF) as u8;
                    },
                    2 => {
                        // 2-byte property (big-endian)
                        if prop_data_addr + 1 >= self.memory.len() {
                            return Err("Property data out of bounds".to_string());
                        }
                        self.memory[prop_data_addr] = (value >> 8) as u8;     // High byte
                        self.memory[prop_data_addr + 1] = (value & 0xFF) as u8; // Low byte
                    },
                    _ => {
                        return Err(format!("PUT_PROP can only write 1 or 2 byte properties, found {} bytes", prop_size));
                    }
                }
                Ok(())
            },
            None => {
                // Property not found - in some games, this should silently succeed
                // This is a common pattern in Z-machine games
                Ok(())
            }
        }
    }

    fn op_sread(&mut self) -> Result<(), String> {
        // SREAD takes 2 operands: text-buffer address, parse-buffer address
        // Reset loop detection counter since we're about to get user input
        self.loop_detection_counter = 0;
        
        eprintln!("DEBUG: SREAD called at PC {:#x}, stack size: {}", self.pc, self.stack.len());
        
        if self.operands_buffer.len() < 2 {
            return Err("SREAD instruction missing operands".to_string());
        }
        
        let text_buffer = self.operands_buffer[0] as usize;
        let parse_buffer = self.operands_buffer[1] as usize;
        
        
        if text_buffer >= self.memory.len() || parse_buffer >= self.memory.len() {
            return Err("SREAD buffer address out of bounds".to_string());
        }
        
        if text_buffer == 0 || parse_buffer == 0 {
            // Classic Z-machine games often don't initialize buffer addresses
            // Provide reasonable defaults for compatibility
            
            // Calculate buffer locations based on the game's memory layout
            let global_vars_addr = self.game.header().global_variables;
            let default_text_buffer = ((global_vars_addr + 0x200) & !0xFF) as usize; // Round up to next 256-byte boundary
            let default_parse_buffer = default_text_buffer + 0x100; // 256 bytes after text buffer
            
            // Update global variables so the game can find the buffers
            self.global_vars.insert(125, default_text_buffer as u16);
            self.global_vars.insert(126, default_parse_buffer as u16);
            
            return self.op_sread_with_defaults(default_text_buffer, default_parse_buffer);
        }
        
        // Read max input length from first byte of text buffer
        let max_len = self.memory[text_buffer] as usize;
        
        // Read input from user
        self.write_output("> ")?;
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                // Remove trailing newline
                input = input.trim_end().to_string();
                
                // Convert to lowercase (Z-machine convention)
                input = input.to_lowercase();
                
                // Truncate to max length
                if input.len() > max_len {
                    input.truncate(max_len);
                }
                
                // Store input length in second byte
                self.memory[text_buffer + 1] = input.len() as u8;
                
                // Store input characters starting at third byte
                for (i, ch) in input.chars().enumerate() {
                    if text_buffer + 2 + i < self.memory.len() {
                        self.memory[text_buffer + 2 + i] = ch as u8;
                    }
                }
                
                // Parse input into words using TOKENISE functionality
                // Set up operands for tokenise operation
                let old_operands = self.operands_buffer.clone();
                self.operands_buffer = vec![text_buffer as u16, parse_buffer as u16];
                
                // Call tokenise to parse the input
                match self.op_tokenise() {
                    Ok(_) => {
                        // Restore original operands
                        self.operands_buffer = old_operands;
                        eprintln!("DEBUG: SREAD completed at PC {:#x}, stack size: {}, about to return control to game", self.pc, self.stack.len());
                    }
                    Err(e) => {
                        // Restore original operands and propagate error
                        self.operands_buffer = old_operands;
                        return Err(format!("Failed to parse input: {}", e));
                    }
                }
                
                Ok(())
            }
            Err(_) => Err("Failed to read input".to_string()),
        }
    }

    fn op_sread_with_defaults(&mut self, text_buffer: usize, parse_buffer: usize) -> Result<(), String> {
        // Reset loop detection counter since we're about to get user input
        self.loop_detection_counter = 0;
        
        // Initialize buffers with reasonable defaults for classic games
        if text_buffer < self.memory.len() {
            self.memory[text_buffer] = 80; // Max 80 characters
        }
        if parse_buffer < self.memory.len() {
            self.memory[parse_buffer] = 10; // Max 10 words
        }
        
        // Read max input length from first byte of text buffer
        let max_len = self.memory[text_buffer] as usize;
        
        // Read input from user
        self.write_output("> ")?;
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                // Remove trailing newline
                input = input.trim_end().to_string();
                
                // Convert to lowercase (Z-machine convention)
                input = input.to_lowercase();
                
                // Truncate to max length
                if input.len() > max_len {
                    input.truncate(max_len);
                }
                
                // Store input length in second byte
                self.memory[text_buffer + 1] = input.len() as u8;
                
                // Store input characters starting at third byte
                for (i, ch) in input.chars().enumerate() {
                    if text_buffer + 2 + i < self.memory.len() {
                        self.memory[text_buffer + 2 + i] = ch as u8;
                    }
                }
                
                // Parse input into words using TOKENISE functionality
                let old_operands = self.operands_buffer.clone();
                self.operands_buffer = vec![text_buffer as u16, parse_buffer as u16];
                
                // Call tokenise to parse the input
                match self.op_tokenise() {
                    Ok(_) => {
                        // Restore original operands
                        self.operands_buffer = old_operands;
                    }
                    Err(e) => {
                        // Restore original operands and propagate error
                        self.operands_buffer = old_operands;
                        return Err(format!("Failed to parse input: {}", e));
                    }
                }
                
                Ok(())
            }
            Err(_) => Err("Failed to read input".to_string()),
        }
    }

    pub fn op_print_char(&mut self) -> Result<(), String> {
        // PRINT_CHAR takes one operand (the character code to print)
        if self.operands_buffer.is_empty() {
            return Err("PRINT_CHAR instruction missing operand".to_string());
        }
        
        let char_code = self.operands_buffer[0];
        
        // Convert to char and print (basic ASCII for now)
        if char_code <= 255 {
            let ch = char_code as u8 as char;
            self.write_output(&ch.to_string())?;
        }
        
        Ok(())
    }

    pub fn op_print_num(&mut self) -> Result<(), String> {
        // PRINT_NUM takes one operand (the signed number to print)
        if self.operands_buffer.is_empty() {
            return Err("PRINT_NUM instruction missing operand".to_string());
        }
        
        let number = self.operands_buffer[0] as i16;  // Treat as signed
        self.write_output(&number.to_string())?;
        
        Ok(())
    }

    pub fn op_random(&mut self) -> Result<(), String> {
        // RANDOM takes one operand (range)
        if self.operands_buffer.is_empty() {
            return Err("RANDOM instruction missing operand".to_string());
        }
        
        let range = self.operands_buffer[0] as i16;
        
        let result = if range > 0 {
            // Positive range: return random number from 1 to range
            let random_value = self.generate_random();
            1 + (random_value % (range as u32)) as u16
        } else if range < 0 {
            // Negative range: seed the generator with abs(range)
            self.random_seed = (-range) as u32;
            0
        } else {
            // Range 0: seed with unpredictable value
            self.random_seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32;
            0
        };
        
        self.store_variable(self.current_store_variable, result)
    }
    
    fn generate_random(&mut self) -> u32 {
        // Simple linear congruential generator
        // Using constants from Numerical Recipes
        self.random_seed = (self.random_seed.wrapping_mul(1664525).wrapping_add(1013904223)) & 0x7FFFFFFF;
        self.random_seed
    }

    pub fn op_push(&mut self) -> Result<(), String> {
        // PUSH takes one operand (the value to push)
        // In some cases, PUSH might be called without operands (stack manipulation)
        let value = if self.operands_buffer.is_empty() {
            // If no operands provided, this might be a stack manipulation instruction
            // that pushes a default value or performs some other operation
            0  // Default value
        } else {
            self.operands_buffer[0]
        };
        
        self.stack.push(value);
        Ok(())
    }

    pub fn op_pull(&mut self) -> Result<(), String> {
        // PULL takes one operand (the variable to store the popped value)
        if self.operands_buffer.is_empty() {
            return Err("PULL instruction missing operand".to_string());
        }
        
        if self.stack.is_empty() {
            return Err("Stack underflow in pull".to_string());
        }
        
        let value = self.stack.pop().unwrap();
        let var_num = self.operands_buffer[0] as u8;
        
        self.store_variable(var_num, value)
    }

    fn op_split_window(&mut self) -> Result<(), String> {
        // SPLIT_WINDOW splits the screen into upper and lower windows
        // Operand 1: lines - number of lines for upper window (0 = unsplit)
        
        if self.operands_buffer.is_empty() {
            return Err("SPLIT_WINDOW instruction missing operand".to_string());
        }
        
        let lines = self.operands_buffer[0];
        
        if lines == 0 {
            // Unsplit screen - upper window has no height
            if let Some(upper_window) = self.windows.get_mut(1) {
                upper_window.height = 0;
            }
            
            // Lower window takes full screen
            if let Some(lower_window) = self.windows.get_mut(0) {
                lower_window.top = 1;
                lower_window.height = self.screen_height;
            }
            
            // Switch to lower window
            self.current_window = 0;
        } else {
            // Split screen
            let upper_lines = std::cmp::min(lines, self.screen_height - 1);
            let lower_lines = self.screen_height - upper_lines;
            
            // Upper window (status line)
            if let Some(upper_window) = self.windows.get_mut(1) {
                upper_window.top = 1;
                upper_window.height = upper_lines;
                upper_window.width = self.screen_width;
                upper_window.cursor_row = 1;
                upper_window.cursor_col = 1;
            }
            
            // Lower window (main text)
            if let Some(lower_window) = self.windows.get_mut(0) {
                lower_window.top = upper_lines + 1;
                lower_window.height = lower_lines;
                lower_window.width = self.screen_width;
                // Don't reset cursor position in lower window
            }
            
            // Clear upper window when splitting
            self.clear_window(1)?;
        }
        
        Ok(())
    }

    fn op_set_window(&mut self) -> Result<(), String> {
        // SET_WINDOW selects which window to use for output
        // Operand 1: window number (0 = lower, 1 = upper)
        
        if self.operands_buffer.is_empty() {
            return Err("SET_WINDOW instruction missing operand".to_string());
        }
        
        let window_num = self.operands_buffer[0];
        
        match window_num {
            0 => {
                // Lower window (main text window)
                self.current_window = 0;
            }
            1 => {
                // Upper window (status line)
                // Check if upper window has been split
                if let Some(upper_window) = self.windows.get(1) {
                    if upper_window.height > 0 {
                        self.current_window = 1;
                    } else {
                        // Upper window not split, stay in lower window
                        self.current_window = 0;
                    }
                }
            }
            _ => {
                return Err(format!("SET_WINDOW: invalid window number {}", window_num));
            }
        }
        
        Ok(())
    }

    fn op_call_vs2(&mut self) -> Result<(), String> {
        // CALL_VS2 is similar to CALL but with different operand handling
        // It takes up to 7 operands (routine address + up to 6 arguments)
        
        if self.operands_buffer.is_empty() {
            return Err("CALL_VS2 instruction missing routine address".to_string());
        }
        
        let routine_addr = self.operands_buffer[0];
        
        if routine_addr == 0 {
            // Call to routine 0 returns 0
            self.stack.push(0);
            return Ok(());
        }
        
        // Convert packed address to byte address
        let byte_addr = self.convert_packed_address(routine_addr);
        
        if byte_addr >= self.memory.len() {
            return Err(format!("CALL_VS2: routine address {:#x} out of bounds", byte_addr));
        }
        
        // Read number of local variables
        let num_locals = self.memory[byte_addr];
        
        if num_locals > 15 {
            return Err("CALL_VS2: Too many local variables".to_string());
        }
        
        // Create call frame
        let call_frame = CallFrame {
            return_pc: self.pc,
            local_vars: self.local_vars,
            num_locals,
            result_var: Some(0), // Store result on stack
        };
        
        self.call_stack.push(call_frame);
        
        // Set up new local variables
        let mut new_locals = [0u16; 15];
        
        // Read default values and set arguments
        let mut addr = byte_addr + 1;
        for i in 0..(num_locals as usize) {
            if addr + 1 < self.memory.len() {
                let default_value = ((self.memory[addr] as u16) << 8) | (self.memory[addr + 1] as u16);
                new_locals[i] = default_value;
                addr += 2;
            }
        }
        
        // Override with provided arguments (skip first operand which is routine address)
        for (i, &arg) in self.operands_buffer.iter().skip(1).enumerate() {
            if i < num_locals as usize {
                new_locals[i] = arg;
            }
        }
        
        self.local_vars = new_locals;
        
        // Set PC to start of routine code
        self.pc = addr;
        
        Ok(())
    }

    fn op_erase_window(&mut self) -> Result<(), String> {
        // ERASE_WINDOW clears a window
        // Operand 1: window number (0 = lower, 1 = upper, -1 = entire screen, -2 = entire screen and unsplit)
        
        if self.operands_buffer.is_empty() {
            return Err("ERASE_WINDOW instruction missing operand".to_string());
        }
        
        let window_spec = self.operands_buffer[0] as i16;
        
        match window_spec {
            0 => {
                // Clear lower window
                self.clear_window(0)?;
            }
            1 => {
                // Clear upper window
                self.clear_window(1)?;
            }
            -1 => {
                // Clear entire screen
                self.clear_screen()?;
            }
            -2 => {
                // Clear entire screen and unsplit
                self.clear_screen()?;
                self.op_split_window_internal(0)?; // Unsplit
            }
            _ => {
                return Err(format!("ERASE_WINDOW: invalid window specification {}", window_spec));
            }
        }
        
        Ok(())
    }

    fn op_erase_line(&mut self) -> Result<(), String> {
        // ERASE_LINE clears the current line from cursor position to end of line
        // Operand 1: value (1 = clear from cursor to end of line, other values reserved)
        
        let value = if !self.operands_buffer.is_empty() {
            self.operands_buffer[0]
        } else {
            1 // Default value
        };
        
        match value {
            1 => {
                // Clear from cursor to end of line in current window
                self.clear_line_from_cursor()?;
            }
            _ => {
                // Other values are reserved/implementation-specific
                // For now, just treat as clear to end of line
                self.clear_line_from_cursor()?;
            }
        }
        
        Ok(())
    }

    fn op_set_cursor(&mut self) -> Result<(), String> {
        // SET_CURSOR positions cursor in current window
        // Operand 1: row (1-based)
        // Operand 2: column (1-based)
        
        if self.operands_buffer.len() < 2 {
            return Err("SET_CURSOR instruction missing operands".to_string());
        }
        
        let row = self.operands_buffer[0];
        let col = self.operands_buffer[1];
        
        // Validate coordinates
        if row == 0 || col == 0 {
            return Err("SET_CURSOR: invalid cursor position (1-based)".to_string());
        }
        
        // Set cursor position in current window
        if let Some(window) = self.windows.get_mut(self.current_window as usize) {
            // Clamp to window boundaries
            let max_row = window.height;
            let max_col = window.width;
            
            window.cursor_row = std::cmp::min(row, max_row);
            window.cursor_col = std::cmp::min(col, max_col);
            
            // Store values to avoid borrowing issues
            let cursor_row = window.cursor_row;
            let cursor_col = window.cursor_col;
            
            // Send cursor positioning escape sequence for terminal
            let _ = window; // End the borrow
            self.position_cursor(cursor_row, cursor_col)?;
        } else {
            return Err("SET_CURSOR: invalid current window".to_string());
        }
        
        Ok(())
    }

    fn op_get_cursor(&mut self) -> Result<(), String> {
        // GET_CURSOR stores cursor position in a table
        // Operand 1: table address (2 words: row, column)
        
        if self.operands_buffer.is_empty() {
            return Err("GET_CURSOR instruction missing operand".to_string());
        }
        
        let table_addr = self.operands_buffer[0] as usize;
        
        if table_addr + 3 >= self.memory.len() {
            return Err("GET_CURSOR: table address out of bounds".to_string());
        }
        
        // Get cursor position from current window
        if let Some(window) = self.windows.get(self.current_window as usize) {
            let row = window.cursor_row;
            let col = window.cursor_col;
            
            // Store as 2-byte words in memory
            self.memory[table_addr] = (row >> 8) as u8;
            self.memory[table_addr + 1] = row as u8;
            self.memory[table_addr + 2] = (col >> 8) as u8;
            self.memory[table_addr + 3] = col as u8;
        } else {
            return Err("GET_CURSOR: invalid current window".to_string());
        }
        
        Ok(())
    }

    fn op_set_text_style(&mut self) -> Result<(), String> {
        // SET_TEXT_STYLE sets text formatting style
        // Operand 1: style flags (bit 0=reverse, bit 1=bold, bit 2=italic, bit 3=fixed-width)
        
        if self.operands_buffer.is_empty() {
            return Err("SET_TEXT_STYLE instruction missing operand".to_string());
        }
        
        let style = self.operands_buffer[0];
        
        // Update current window's text style
        if let Some(window) = self.windows.get_mut(self.current_window as usize) {
            window.text_style = style;
            
            // Apply terminal formatting escape sequences
            self.apply_text_style(style)?;
        } else {
            return Err("SET_TEXT_STYLE: invalid current window".to_string());
        }
        
        Ok(())
    }

    fn op_buffer_mode(&mut self) -> Result<(), String> {
        // BUFFER_MODE controls output buffering
        // Operand 1: flag (0=disable buffering, 1=enable buffering)
        
        if self.operands_buffer.is_empty() {
            return Err("BUFFER_MODE instruction missing operand".to_string());
        }
        
        let flag = self.operands_buffer[0];
        
        match flag {
            0 => {
                // Disable buffering - flush output immediately
                stdout().flush().map_err(|e| format!("BUFFER_MODE flush error: {}", e))?;
                // Note: In a full implementation, this would also set a flag to flush after each output
            }
            1 => {
                // Enable buffering - output is buffered until flushed
                // This is typically the default behavior
            }
            _ => {
                return Err(format!("BUFFER_MODE: invalid flag {}", flag));
            }
        }
        
        Ok(())
    }

    fn op_output_stream(&mut self) -> Result<(), String> {
        // OUTPUT_STREAM manages output streams
        // Operand 1: stream number (positive to enable, negative to disable)
        // Operand 2: table address (for stream 3 only), optional
        
        if self.operands_buffer.is_empty() {
            return Err("OUTPUT_STREAM instruction missing operand".to_string());
        }
        
        let stream_spec = self.operands_buffer[0] as i16;
        let stream_num = stream_spec.abs() as u16;
        let enable = stream_spec > 0;
        
        match stream_num {
            1 => {
                // Screen output
                if enable {
                    if !self.output_streams.contains(&1) {
                        self.output_streams.push(1);
                    }
                } else {
                    self.output_streams.retain(|&x| x != 1);
                }
            }
            2 => {
                // Transcript output
                if enable {
                    if !self.output_streams.contains(&2) {
                        self.output_streams.push(2);
                        // TODO: Open transcript file
                    }
                } else {
                    self.output_streams.retain(|&x| x != 2);
                    // TODO: Close transcript file
                }
            }
            3 => {
                // Memory output
                if enable {
                    if self.operands_buffer.len() < 2 {
                        return Err("OUTPUT_STREAM 3 requires table address".to_string());
                    }
                    let table_addr = self.operands_buffer[1];
                    if !self.output_streams.contains(&3) {
                        self.output_streams.push(3);
                        self.memory_stream_addr = Some(table_addr);
                        self.memory_stream_data.clear();
                    }
                } else {
                    if self.output_streams.contains(&3) {
                        self.output_streams.retain(|&x| x != 3);
                        // Write collected data to memory
                        self.flush_memory_stream()?;
                    }
                }
            }
            4 => {
                // Command output (not commonly used)
                if enable {
                    if !self.output_streams.contains(&4) {
                        self.output_streams.push(4);
                    }
                } else {
                    self.output_streams.retain(|&x| x != 4);
                }
            }
            _ => {
                return Err(format!("OUTPUT_STREAM: invalid stream number {}", stream_num));
            }
        }
        
        Ok(())
    }
    
    fn flush_memory_stream(&mut self) -> Result<(), String> {
        // Write memory stream data to the specified address
        if let Some(addr) = self.memory_stream_addr {
            let addr = addr as usize;
            if addr + 2 + self.memory_stream_data.len() > self.memory.len() {
                return Err("OUTPUT_STREAM: memory stream overflow".to_string());
            }
            
            // Write length as 2-byte word
            let length = self.memory_stream_data.len() as u16;
            self.memory[addr] = (length >> 8) as u8;
            self.memory[addr + 1] = length as u8;
            
            // Write data
            for (i, &byte) in self.memory_stream_data.iter().enumerate() {
                self.memory[addr + 2 + i] = byte;
            }
            
            self.memory_stream_addr = None;
            self.memory_stream_data.clear();
        }
        Ok(())
    }

    fn op_input_stream(&mut self) -> Result<(), String> {
        // INPUT_STREAM selects input stream
        // Operand 1: stream number (0=keyboard, 1=file)
        
        if self.operands_buffer.is_empty() {
            return Err("INPUT_STREAM instruction missing operand".to_string());
        }
        
        let stream_num = self.operands_buffer[0];
        
        match stream_num {
            0 => {
                // Keyboard input
                self.input_stream = 0;
            }
            1 => {
                // File input (not implemented yet)
                return Err("INPUT_STREAM from file not implemented".to_string());
            }
            _ => {
                return Err(format!("INPUT_STREAM: invalid stream number {}", stream_num));
            }
        }
        
        Ok(())
    }

    fn op_sound_effect(&mut self) -> Result<(), String> {
        println!("SOUND_EFFECT: (not implemented)");
        Ok(())
    }

    fn op_read_char(&mut self) -> Result<(), String> {
        // READ_CHAR reads a single character from input
        // Operand 1: input device (1=keyboard, 2=file), default 1
        // Operand 2: time (timeout in tenths of seconds), optional
        // Operand 3: routine (timeout routine), optional
        
        let device = if !self.operands_buffer.is_empty() {
            self.operands_buffer[0]
        } else {
            1 // Default to keyboard
        };
        
        let timeout = if self.operands_buffer.len() >= 2 {
            self.operands_buffer[1]
        } else {
            0 // No timeout
        };
        
        let _timeout_routine = if self.operands_buffer.len() >= 3 {
            self.operands_buffer[2]
        } else {
            0 // No timeout routine
        };
        
        let char_code = match device {
            1 => {
                // Keyboard input
                if timeout > 0 {
                    // TODO: Implement timed input
                    // For now, just do regular input
                    self.read_char_from_keyboard()?
                } else {
                    self.read_char_from_keyboard()?
                }
            }
            2 => {
                // File input (not implemented yet)
                return Err("READ_CHAR from file not implemented".to_string());
            }
            _ => {
                return Err("READ_CHAR: invalid input device".to_string());
            }
        };
        
        // Store the character code in the specified variable
        self.stack.push(char_code);
        Ok(())
    }
    
    fn read_char_from_keyboard(&mut self) -> Result<u16, String> {
        // Read a single character from stdin
        let mut buffer = String::new();
        match stdin().read_line(&mut buffer) {
            Ok(_) => {
                // Get the first character or return 0 if empty
                let char_code = buffer.chars().next().unwrap_or('\0') as u16;
                Ok(char_code)
            }
            Err(e) => Err(format!("READ_CHAR input error: {}", e))
        }
    }
    
    // Window management helper methods
    fn clear_window(&mut self, window_id: u16) -> Result<(), String> {
        let (height, top, left) = if let Some(window) = self.windows.get_mut(window_id as usize) {
            // Reset cursor to top-left of window
            window.cursor_row = 1;
            window.cursor_col = 1;
            
            // Store values to avoid borrowing issues
            (window.height, window.top, window.left)
        } else {
            return Ok(()); // Window doesn't exist, silently succeed
        };
        
        // Send clear commands to terminal
        if height > 0 {
            for row in 0..height {
                // Position cursor at start of each line in window
                self.position_cursor(top + row, left)?;
                // Clear to end of line
                print!("\x1b[K");
            }
            
            // Position cursor back at window top-left
            self.position_cursor(top, left)?;
        }
        
        Ok(())
    }
    
    fn clear_screen(&mut self) -> Result<(), String> {
        // Clear entire screen
        print!("\x1b[2J\x1b[H");
        stdout().flush().map_err(|e| format!("Clear screen error: {}", e))?;
        
        // Reset all window cursors
        for window in &mut self.windows {
            if window.height > 0 {
                window.cursor_row = 1;
                window.cursor_col = 1;
            }
        }
        
        Ok(())
    }
    
    fn clear_line_from_cursor(&mut self) -> Result<(), String> {
        // Clear from cursor to end of line
        print!("\x1b[K");
        stdout().flush().map_err(|e| format!("Clear line error: {}", e))?;
        Ok(())
    }
    
    fn position_cursor(&mut self, row: u16, col: u16) -> Result<(), String> {
        // Send ANSI escape sequence to position cursor
        print!("\x1b[{};{}H", row, col);
        stdout().flush().map_err(|e| format!("Position cursor error: {}", e))?;
        Ok(())
    }
    
    fn apply_text_style(&mut self, style: u16) -> Result<(), String> {
        // Apply ANSI text formatting
        let mut escape_seq = String::from("\x1b[0m"); // Reset first
        
        if style & 1 != 0 {
            escape_seq.push_str("\x1b[7m"); // Reverse video
        }
        if style & 2 != 0 {
            escape_seq.push_str("\x1b[1m"); // Bold
        }
        if style & 4 != 0 {
            escape_seq.push_str("\x1b[3m"); // Italic
        }
        if style & 8 != 0 {
            // Fixed-width font (no specific ANSI code, just note it)
        }
        
        print!("{}", escape_seq);
        stdout().flush().map_err(|e| format!("Text style error: {}", e))?;
        Ok(())
    }
    
    fn op_split_window_internal(&mut self, lines: u16) -> Result<(), String> {
        // Internal version of split_window for use by other operations
        let old_operands = self.operands_buffer.clone();
        self.operands_buffer = vec![lines];
        let result = self.op_split_window();
        self.operands_buffer = old_operands;
        result
    }
    
    fn write_output(&mut self, text: &str) -> Result<(), String> {
        // Convert carets to newlines for proper text formatting
        let formatted_text = text.replace('^', "\n");
        
        // Write text to all active output streams
        for &stream in &self.output_streams.clone() {
            match stream {
                1 => {
                    // Screen output
                    print!("{}", formatted_text);
                    stdout().flush().map_err(|e| format!("Screen output error: {}", e))?;
                }
                2 => {
                    // Transcript output
                    // TODO: Write to transcript file
                }
                3 => {
                    // Memory output
                    for ch in formatted_text.chars() {
                        self.memory_stream_data.push(ch as u8);
                    }
                }
                4 => {
                    // Command output
                    // TODO: Write to command file
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn op_scan_table(&mut self) -> Result<(), String> {
        // SCAN_TABLE takes 4 operands and branches on result:
        // 1. x - value to search for
        // 2. table - table address  
        // 3. len - table length (number of entries)
        // 4. form - entry form (optional, default 0x82)
        //   - Bits 0-6: entry length in bytes
        //   - Bit 7: if set, compare only low byte of value
        
        if self.operands_buffer.len() < 3 {
            return Err("SCAN_TABLE instruction missing operands".to_string());
        }
        
        let search_value = self.operands_buffer[0];
        let table_addr = self.operands_buffer[1] as usize;
        let table_len = self.operands_buffer[2] as usize;
        let form = if self.operands_buffer.len() >= 4 {
            self.operands_buffer[3] as u8
        } else {
            0x82 // Default: 2-byte entries, compare full word
        };
        
        if table_addr >= self.memory.len() {
            return Err("SCAN_TABLE table address out of bounds".to_string());
        }
        
        let entry_length = (form & 0x7F) as usize; // Bits 0-6
        let compare_byte_only = (form & 0x80) != 0; // Bit 7
        
        if entry_length == 0 {
            return Err("SCAN_TABLE entry length cannot be zero".to_string());
        }
        
        // Search through the table
        for i in 0..table_len {
            let entry_addr = table_addr + (i * entry_length);
            
            if entry_addr + entry_length > self.memory.len() {
                break; // Stop if entry would exceed memory bounds
            }
            
            let found = if compare_byte_only {
                // Compare only low byte
                let entry_value = self.memory[entry_addr] as u16;
                let search_low_byte = search_value & 0xFF;
                entry_value == search_low_byte
            } else {
                // Compare full word (assuming big-endian storage)
                let entry_value = if entry_length >= 2 {
                    ((self.memory[entry_addr] as u16) << 8) | (self.memory[entry_addr + 1] as u16)
                } else {
                    self.memory[entry_addr] as u16
                };
                entry_value == search_value
            };
            
            if found {
                // Found the value - store the address and branch true
                self.store_variable(self.current_store_variable, entry_addr as u16)?;
                
                if let Some(_) = self.current_branch_offset {
                    self.handle_branch(true)?;
                }
                return Ok(());
            }
        }
        
        // Not found - store 0 and branch false
        self.store_variable(self.current_store_variable, 0)?;
        
        if let Some(_) = self.current_branch_offset {
            self.handle_branch(false)?;
        }
        
        Ok(())
    }

    fn op_not_v4(&mut self) -> Result<(), String> {
        // NOT_V4 performs bitwise NOT operation
        // Same as NOT instruction but with different operand handling
        
        if self.operands_buffer.is_empty() {
            return Err("NOT_V4 instruction missing operand".to_string());
        }
        
        let value = self.operands_buffer[0];
        let result = !value; // Bitwise NOT
        
        self.stack.push(result);
        Ok(())
    }

    fn op_call_vn(&mut self) -> Result<(), String> {
        // CALL_VN calls a routine but discards the return value
        // VN = Variable operands, No result
        
        if self.operands_buffer.is_empty() {
            return Err("CALL_VN instruction missing routine address".to_string());
        }
        
        let routine_addr = self.operands_buffer[0];
        
        if routine_addr == 0 {
            // Call to routine 0 does nothing
            return Ok(());
        }
        
        // Convert packed address to byte address
        let byte_addr = self.convert_packed_address(routine_addr);
        
        if byte_addr >= self.memory.len() {
            return Err(format!("CALL_VN: routine address {:#x} out of bounds", byte_addr));
        }
        
        // Read number of local variables
        let num_locals = self.memory[byte_addr];
        
        if num_locals > 15 {
            return Err("CALL_VN: Too many local variables".to_string());
        }
        
        // Create call frame (no result variable since we discard return value)
        let call_frame = CallFrame {
            return_pc: self.pc,
            local_vars: self.local_vars,
            num_locals,
            result_var: None, // No result stored
        };
        
        self.call_stack.push(call_frame);
        
        // Set up new local variables
        let mut new_locals = [0u16; 15];
        
        // Read default values and set arguments
        let mut addr = byte_addr + 1;
        for i in 0..(num_locals as usize) {
            if addr + 1 < self.memory.len() {
                let default_value = ((self.memory[addr] as u16) << 8) | (self.memory[addr + 1] as u16);
                new_locals[i] = default_value;
                addr += 2;
            }
        }
        
        // Override with provided arguments (skip first operand which is routine address)
        for (i, &arg) in self.operands_buffer.iter().skip(1).enumerate() {
            if i < num_locals as usize {
                new_locals[i] = arg;
            }
        }
        
        self.local_vars = new_locals;
        
        // Set PC to start of routine code
        self.pc = addr;
        
        Ok(())
    }

    fn op_call_vn2(&mut self) -> Result<(), String> {
        // CALL_VN2 is similar to CALL_VN but with different operand handling
        // It takes up to 7 operands (routine address + up to 6 arguments)
        // VN2 = Variable operands (extended), No result
        
        if self.operands_buffer.is_empty() {
            return Err("CALL_VN2 instruction missing routine address".to_string());
        }
        
        let routine_addr = self.operands_buffer[0];
        
        if routine_addr == 0 {
            // Call to routine 0 does nothing
            return Ok(());
        }
        
        // Convert packed address to byte address
        let byte_addr = self.convert_packed_address(routine_addr);
        
        if byte_addr >= self.memory.len() {
            return Err(format!("CALL_VN2: routine address {:#x} out of bounds", byte_addr));
        }
        
        // Read number of local variables
        let num_locals = self.memory[byte_addr];
        
        if num_locals > 15 {
            return Err("CALL_VN2: Too many local variables".to_string());
        }
        
        // Create call frame (no result variable since we discard return value)
        let call_frame = CallFrame {
            return_pc: self.pc,
            local_vars: self.local_vars,
            num_locals,
            result_var: None, // No result stored
        };
        
        self.call_stack.push(call_frame);
        
        // Set up new local variables
        let mut new_locals = [0u16; 15];
        
        // Read default values and set arguments
        let mut addr = byte_addr + 1;
        for i in 0..(num_locals as usize) {
            if addr + 1 < self.memory.len() {
                let default_value = ((self.memory[addr] as u16) << 8) | (self.memory[addr + 1] as u16);
                new_locals[i] = default_value;
                addr += 2;
            }
        }
        
        // Override with provided arguments (skip first operand which is routine address)
        for (i, &arg) in self.operands_buffer.iter().skip(1).enumerate() {
            if i < num_locals as usize {
                new_locals[i] = arg;
            }
        }
        
        self.local_vars = new_locals;
        
        // Set PC to start of routine code
        self.pc = addr;
        
        Ok(())
    }

    fn op_tokenise(&mut self) -> Result<(), String> {
        // TOKENISE takes 2-4 operands:
        // 1. text-buffer address (required)
        // 2. parse-buffer address (required) 
        // 3. dictionary address (optional, default to game dictionary)
        // 4. flag (optional, default to false)
        
        if self.operands_buffer.len() < 2 {
            return Err("TOKENISE instruction missing operands".to_string());
        }
        
        let text_buffer = self.operands_buffer[0] as usize;
        let parse_buffer = self.operands_buffer[1] as usize;
        
        if text_buffer >= self.memory.len() || parse_buffer >= self.memory.len() {
            return Err("TOKENISE buffer address out of bounds".to_string());
        }
        
        // Get the input text from the text buffer
        let input_length = self.memory[text_buffer + 1] as usize;
        let mut input_text = String::new();
        
        for i in 0..input_length {
            if text_buffer + 2 + i < self.memory.len() {
                input_text.push(self.memory[text_buffer + 2 + i] as char);
            }
        }
        
        // Parse the input text into words
        let words = self.parse_input_text(&input_text)?;
        
        // Get maximum number of words that can be stored in parse buffer
        let max_words = self.memory[parse_buffer] as usize;
        
        // Store the number of words found (up to max_words)
        let words_to_store = std::cmp::min(words.len(), max_words);
        self.memory[parse_buffer + 1] = words_to_store as u8;
        
        // Store each word in the parse buffer
        // Format: [word_length][word_start_offset][dictionary_address_high][dictionary_address_low]
        for (i, word) in words.iter().take(words_to_store).enumerate() {
            let entry_offset = parse_buffer + 2 + (i * 4);
            
            if entry_offset + 3 < self.memory.len() {
                // Store word length
                self.memory[entry_offset] = word.text.len() as u8;
                
                // Store word start position in text buffer (relative to text start)
                self.memory[entry_offset + 1] = (word.position + 2) as u8; // +2 for buffer header
                
                // Look up word in dictionary
                let dict_addr = self.lookup_word_in_dictionary(&word.text)?;
                
                // Store dictionary address (big-endian)
                self.memory[entry_offset + 2] = (dict_addr >> 8) as u8;
                self.memory[entry_offset + 3] = (dict_addr & 0xFF) as u8;
            }
        }
        
        Ok(())
    }

    fn op_encode_text(&mut self) -> Result<(), String> {
        // ENCODE_TEXT takes 4 operands:
        // 1. text-buffer address (zscii text to encode)
        // 2. length of text to encode 
        // 3. start position within text buffer
        // 4. coded-text address (where to store encoded result)
        
        if self.operands_buffer.len() < 4 {
            return Err("ENCODE_TEXT instruction missing operands".to_string());
        }
        
        let text_buffer = self.operands_buffer[0] as usize;
        let length = self.operands_buffer[1] as usize;
        let start_pos = self.operands_buffer[2] as usize;
        let coded_text_addr = self.operands_buffer[3] as usize;
        
        if text_buffer >= self.memory.len() || coded_text_addr >= self.memory.len() {
            return Err("ENCODE_TEXT buffer address out of bounds".to_string());
        }
        
        // Extract the text to encode
        let mut text_to_encode = String::new();
        for i in 0..length {
            let addr = text_buffer + start_pos + i;
            if addr < self.memory.len() {
                text_to_encode.push(self.memory[addr] as char);
            } else {
                break;
            }
        }
        
        // Encode the text to Z-characters
        let encoded_words = self.encode_text_to_zchars(&text_to_encode)?;
        
        // Store the encoded words at the specified address
        for (i, word) in encoded_words.iter().enumerate() {
            let addr = coded_text_addr + (i * 2);
            if addr + 1 < self.memory.len() {
                self.memory[addr] = (word >> 8) as u8;     // High byte
                self.memory[addr + 1] = (word & 0xFF) as u8; // Low byte
            }
        }
        
        Ok(())
    }
    
    fn parse_input_text(&self, input: &str) -> Result<Vec<ParsedWord>, String> {
        let mut words = Vec::new();
        let mut current_word = String::new();
        let mut word_start = 0;
        let mut in_word = false;
        
        for (i, ch) in input.chars().enumerate() {
            if ch.is_alphabetic() || ch == '\'' {
                if !in_word {
                    word_start = i;
                    in_word = true;
                    current_word.clear();
                }
                current_word.push(ch);
            } else {
                if in_word {
                    // End of word
                    if !current_word.is_empty() {
                        words.push(ParsedWord {
                            text: current_word.to_lowercase(),
                            position: word_start,
                        });
                    }
                    current_word.clear();
                    in_word = false;
                }
            }
        }
        
        // Handle word at end of input
        if in_word && !current_word.is_empty() {
            words.push(ParsedWord {
                text: current_word.to_lowercase(),
                position: word_start,
            });
        }
        
        Ok(words)
    }
    
    fn lookup_word_in_dictionary(&self, word: &str) -> Result<u16, String> {
        // Get dictionary address from header
        let dict_addr = self.game.header().dictionary;
        
        // Read dictionary header
        let bytes = self.game.bytes();
        let mut pos = dict_addr;
        
        // Skip separator characters
        let n_separators = bytes[pos] as usize;
        pos += 1 + n_separators;
        
        // Read entry length
        let entry_length = bytes[pos] as usize;
        pos += 1;
        
        // Read number of entries
        let num_entries = ((bytes[pos] as u16) << 8) | (bytes[pos + 1] as u16);
        pos += 2;
        
        // Normalize input word (convert to lowercase, truncate to 6 chars for v1-3)
        let normalized_word = word.to_lowercase();
        let truncated_word = if normalized_word.len() > 6 {
            &normalized_word[..6]
        } else {
            &normalized_word
        };
        
        // Search through dictionary entries
        for i in 0..num_entries {
            let entry_addr = pos + (i as usize * entry_length);
            
            // Read the encoded word (first 4 bytes in v1-3, 6 bytes in v4+)
            let encoded_bytes = if entry_addr + 4 <= bytes.len() {
                &bytes[entry_addr..entry_addr + 4]
            } else {
                continue;
            };
            
            // Decode the dictionary entry to compare
            match self.decode_dictionary_entry(encoded_bytes) {
                Ok(dict_word) => {
                    // Compare normalized versions
                    let normalized_dict_word = dict_word.to_lowercase();
                    if normalized_dict_word == truncated_word {
                        return Ok(entry_addr as u16);
                    }
                }
                Err(_) => continue, // Skip entries that can't be decoded
            }
        }
        
        Ok(0) // Not found in dictionary
    }
    
    fn decode_dictionary_entry(&self, encoded_bytes: &[u8]) -> Result<String, String> {
        // Decode a dictionary entry from its encoded form
        // This is essentially the reverse of text encoding
        
        if encoded_bytes.len() < 4 {
            return Err("Dictionary entry too short".to_string());
        }
        
        let mut result = String::new();
        let mut pos = 0;
        
        // Process pairs of bytes (words)
        while pos + 1 < encoded_bytes.len() {
            let word = ((encoded_bytes[pos] as u16) << 8) | (encoded_bytes[pos + 1] as u16);
            
            // Extract three 5-bit Z-characters from the 16-bit word
            let z1 = ((word >> 10) & 0x1F) as u8;
            let z2 = ((word >> 5) & 0x1F) as u8; 
            let z3 = (word & 0x1F) as u8;
            
            // Convert Z-characters to text
            for zchar in [z1, z2, z3] {
                match zchar {
                    0 => result.push(' '),
                    1..=3 => {
                        // Abbreviations - skip for now in dictionary lookup
                        // Real implementation would need to expand abbreviations
                    }
                    4 => {
                        // Shift to alphabet A1 (uppercase) - skip for now
                    }
                    5 => {
                        // Shift to alphabet A2 (punctuation) - skip for now  
                    }
                    6..=31 => {
                        // Regular characters from alphabet A0 (lowercase)
                        if zchar >= 6 && zchar <= 31 {
                            let ch = (b'a' + (zchar - 6)) as char;
                            result.push(ch);
                        }
                    }
                    _ => {} // Invalid Z-character
                }
            }
            
            pos += 2;
            
            // Check if this is the last word (bit 15 set)
            if (word & 0x8000) != 0 {
                break;
            }
        }
        
        // Trim trailing null characters and spaces
        Ok(result.trim_end_matches(|c| c == ' ' || c == '\0').to_string())
    }
    
    fn encode_text_to_zchars(&self, text: &str) -> Result<Vec<u16>, String> {
        // Convert text to Z-characters (5-bit packed format)
        // This is a simplified implementation
        
        let mut result = Vec::new();
        let mut chars = text.chars().peekable();
        
        while chars.peek().is_some() {
            let mut word = 0u16;
            let mut shift = 10; // Start with highest 5-bit group
            
            for _ in 0..3 { // Pack 3 characters per word
                if let Some(ch) = chars.next() {
                    let zchar = self.char_to_zchar(ch);
                    word |= (zchar as u16) << shift;
                }
                shift -= 5;
            }
            
            // Set the end bit if this is the last word
            if chars.peek().is_none() {
                word |= 0x8000; // Set highest bit
            }
            
            result.push(word);
        }
        
        // Ensure we have at least one word, even for empty text
        if result.is_empty() {
            result.push(0x8000); // Empty word with end bit set
        }
        
        Ok(result)
    }
    
    fn char_to_zchar(&self, ch: char) -> u8 {
        // Convert a character to a Z-character (5-bit value)
        // This is a simplified mapping
        
        match ch {
            'a'..='z' => (ch as u8) - b'a' + 6,  // a-z maps to 6-31
            'A'..='Z' => (ch as u8) - b'A' + 6,  // A-Z maps to 6-31 (same as lowercase)
            ' ' => 0,
            '0'..='9' => (ch as u8) - b'0' + 8,  // Numbers in A2 alphabet
            '.' => 18, // Period in A2 alphabet
            ',' => 19, // Comma in A2 alphabet
            '!' => 20, // Exclamation in A2 alphabet
            '?' => 21, // Question mark in A2 alphabet
            _ => 0,    // Unknown characters map to space
        }
    }

    fn op_copy_table(&mut self) -> Result<(), String> {
        // COPY_TABLE takes 3 operands:
        // 1. first - source address
        // 2. second - destination address  
        // 3. size - number of bytes to copy (can be negative)
        
        if self.operands_buffer.len() < 3 {
            return Err("COPY_TABLE instruction missing operands".to_string());
        }
        
        let first = self.operands_buffer[0] as usize;
        let second = self.operands_buffer[1] as usize;
        let size = self.operands_buffer[2] as i16; // Signed for direction handling
        
        if size == 0 {
            // Zero size: fill destination with zeros
            if second >= self.memory.len() {
                return Err("COPY_TABLE destination address out of bounds".to_string());
            }
            
            let fill_size = first; // When size=0, first operand is the number of bytes to zero
            for i in 0..fill_size {
                if second + i < self.memory.len() {
                    self.memory[second + i] = 0;
                } else {
                    break; // Stop if we reach memory bounds
                }
            }
            return Ok(());
        }
        
        let abs_size = size.abs() as usize;
        
        // Validate addresses
        if first >= self.memory.len() || second >= self.memory.len() {
            return Err("COPY_TABLE address out of bounds".to_string());
        }
        
        if first + abs_size > self.memory.len() || second + abs_size > self.memory.len() {
            return Err("COPY_TABLE operation would exceed memory bounds".to_string());
        }
        
        if size > 0 {
            // Forward copy (normal case)
            // Use a temporary buffer to handle overlapping regions safely
            let mut temp_buffer = Vec::with_capacity(abs_size);
            
            // Read source data into temporary buffer
            for i in 0..abs_size {
                temp_buffer.push(self.memory[first + i]);
            }
            
            // Write from temporary buffer to destination
            for (i, &byte) in temp_buffer.iter().enumerate() {
                self.memory[second + i] = byte;
            }
        } else {
            // Backward copy (size < 0)
            // Copy from end to beginning to handle overlapping regions
            for i in (0..abs_size).rev() {
                self.memory[second + i] = self.memory[first + i];
            }
        }
        
        Ok(())
    }

    fn op_print_table(&mut self) -> Result<(), String> {
        // PRINT_TABLE takes 4 operands:
        // 1. zscii-text - address of ZSCII text table
        // 2. width - width of each row (in characters)
        // 3. height - number of rows (optional, default 1)
        // 4. skip - characters to skip between rows (optional, default 0)
        
        if self.operands_buffer.len() < 2 {
            return Err("PRINT_TABLE instruction missing operands".to_string());
        }
        
        let text_addr = self.operands_buffer[0] as usize;
        let width = self.operands_buffer[1] as usize;
        let height = if self.operands_buffer.len() >= 3 {
            self.operands_buffer[2] as usize
        } else {
            1 // Default to 1 row
        };
        let skip = if self.operands_buffer.len() >= 4 {
            self.operands_buffer[3] as usize
        } else {
            0 // Default to no skip
        };
        
        if text_addr >= self.memory.len() {
            return Err("PRINT_TABLE text address out of bounds".to_string());
        }
        
        if width == 0 {
            return Err("PRINT_TABLE width cannot be zero".to_string());
        }
        
        let mut current_addr = text_addr;
        
        for row in 0..height {
            // Print characters for this row
            for _col in 0..width {
                if current_addr < self.memory.len() {
                    let ch = self.memory[current_addr] as char;
                    
                    // Only print printable characters (ZSCII)
                    if ch >= ' ' && ch <= '~' {
                        print!("{}", ch);
                    } else if ch == '\n' || ch == '\r' {
                        // Handle newlines
                        print!("{}", ch);
                    } else {
                        // For other characters, print a space
                        print!(" ");
                    }
                    
                    current_addr += 1;
                } else {
                    // If we run out of memory, print spaces
                    print!(" ");
                }
            }
            
            // Add newline after each row (except the last one if height > 1)
            if height > 1 && row < height - 1 {
                println!();
            }
            
            // Skip additional characters between rows
            current_addr += skip;
        }
        
        // Flush output
        io::stdout().flush().unwrap();
        
        Ok(())
    }

    fn op_check_arg_count(&mut self) -> Result<(), String> {
        // CHECK_ARG_COUNT checks if the current routine was called with at least N arguments
        // Returns 1 if the condition is true, 0 otherwise
        
        if self.operands_buffer.is_empty() {
            return Err("CHECK_ARG_COUNT instruction missing operand".to_string());
        }
        
        let arg_number = self.operands_buffer[0];
        
        // We need to check how many arguments the current routine was called with
        // This information should be stored in the call frame
        if let Some(current_frame) = self.call_stack.last() {
            // In a real implementation, we'd need to track the actual argument count
            // For now, we'll assume all local variables that are non-zero were provided as arguments
            // This is a simplification - a proper implementation would track argument counts
            
            let mut arg_count = 0;
            for i in 0..(current_frame.num_locals as usize) {
                if i < 15 && self.local_vars[i] != 0 {
                    arg_count += 1;
                }
            }
            
            let result = if arg_count >= arg_number { 1 } else { 0 };
            self.stack.push(result);
        } else {
            // Not in a routine call - return 0
            self.stack.push(0);
        }
        
        Ok(())
    }

    pub fn op_call_2s(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        // CALL_2S: Call routine with 2 arguments, store result
        let routine_addr = operand1;
        let arg1 = operand2;
        
        
        // Set up operands buffer for the generic call implementation
        self.operands_buffer = vec![routine_addr, arg1];
        
        // Call the generic call implementation
        self.op_call()
    }

    pub fn op_call_2n(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        // CALL_2N: Call routine with 2 arguments, no result stored
        let routine_addr = operand1;
        let arg1 = operand2;
        
        if routine_addr == 0 {
            // Call to routine 0 - just return without storing result
            return Ok(());
        }
        
        // Set up operands buffer for the generic call implementation
        self.operands_buffer = vec![routine_addr, arg1];
        
        // Perform the call but don't store the result
        // We need to temporarily modify how the call works
        let _old_call_stack_len = self.call_stack.len();
        
        // Set up the call frame to not store result
        if let Err(e) = self.setup_call_frame(routine_addr, &[arg1], None) {
            return Err(e);
        }
        
        Ok(())
    }
    
    fn setup_call_frame(&mut self, routine_addr: u16, args: &[u16], result_var: Option<u8>) -> Result<(), String> {
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
            return Err(format!("Too many local variables in routine: {} at address {:#06x} (routine_addr={:#06x})", num_locals, byte_addr, routine_addr));
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
            result_var: result_var,
        };
        self.call_stack.push(call_frame);
        
        // Set up new locals with defaults
        self.local_vars = local_defaults;
        
        // Pass arguments to local variables
        for (i, &arg_value) in args.iter().enumerate() {
            if i < num_locals as usize {
                self.local_vars[i] = arg_value;
            }
        }
        
        // Jump to routine code
        self.pc = routine_pc;
        
        // println!("CALL: Calling routine at {:#06x} with {} locals, {} args", 
        //         byte_addr, num_locals, args.len());
        
        Ok(())
    }

    fn op_set_colour(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        println!("SET_COLOUR: fg {} bg {} (not implemented)", operand1, operand2);
        Ok(())
    }

    fn op_throw(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        // THROW: throw exception with value operand1 to frame operand2
        // This unwinds the call stack to the specified frame level
        let target_frame = operand2 as usize;
        let return_value = operand1;
        
        // According to Z-machine spec, frame values should be those returned by CATCH
        // Frame 0 is undefined behavior, but we'll handle it gracefully
        if target_frame == 0 {
            // This is undefined behavior in the Z-machine spec
            // Instead of quitting, return an error to help debug
            return Err(format!("THROW: frame 0 is undefined behavior (value: {})", return_value));
        }
        
        // Unwind call stack to target frame
        if self.call_stack.len() < target_frame {
            return Err(format!("THROW: target frame {} not found (stack size: {})", 
                              target_frame, self.call_stack.len()));
        }
        
        // Pop frames until we reach the target frame
        while self.call_stack.len() > target_frame {
            self.call_stack.pop();
        }
        
        // Return from the target frame with the throw value
        self.return_from_routine(return_value)
    }

    pub fn return_from_routine(&mut self, value: u16) -> Result<(), String> {
        if let Some(frame) = self.call_stack.pop() {
            self.pc = frame.return_pc;
            self.local_vars = frame.local_vars;
            
            if let Some(result_var) = frame.result_var {
                self.store_variable(result_var, value)?;
            }
            
        } else {
            // Return from main routine - quit
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
                        // println!("BRANCH: rfalse");
                        self.return_from_routine(0)?;
                    },
                    1 => {
                        // Branch offset 1 means RTRUE  
                        // println!("BRANCH: rtrue");
                        self.return_from_routine(1)?;
                    },
                    _ => {
                        // Normal branch: offset is relative to PC after instruction
                        let new_pc = (self.pc as i32) + (actual_offset as i32) - 2;
                        if new_pc < 0 || new_pc >= self.memory.len() as i32 {
                            return Err(format!("Branch target out of bounds: {:#06x}", new_pc));
                        }
                        self.pc = new_pc as usize;
                        // println!("BRANCH: jumping to {:#06x} (offset {})", self.pc, actual_offset);
                    }
                }
            } else {
                // println!("BRANCH: condition false, not branching");
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
            return Err(format!("Attribute number must be 0-31, got {}", attr_num));
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

    // Helper methods for property table access
    fn get_object_properties_addr(&self, obj_num: u16) -> Result<usize, String> {
        if obj_num == 0 {
            return Err("Object number 0 is invalid".to_string());
        }
        
        let obj_addr = self.get_object_addr(obj_num)?;
        if obj_addr + 8 >= self.memory.len() {
            return Err("Object table access out of bounds".to_string());
        }
        
        // Properties address is stored in bytes 7-8 of object (big-endian)
        let props_addr = ((self.memory[obj_addr + 7] as u16) << 8) | (self.memory[obj_addr + 8] as u16);
        Ok(props_addr as usize)
    }
    
    fn find_property(&self, obj_num: u16, prop_num: u8) -> Result<Option<(usize, u8)>, String> {
        let props_addr = self.get_object_properties_addr(obj_num)?;
        
        if props_addr >= self.memory.len() {
            return Err("Property table address out of bounds".to_string());
        }
        
        // Skip object description - first byte is description length in words
        let desc_len = self.memory[props_addr] as usize;
        let mut cursor = props_addr + 1 + (desc_len * 2);
        
        // Search through properties
        while cursor < self.memory.len() {
            let size_byte = self.memory[cursor];
            
            if size_byte == 0 {
                // End of property list
                break;
            }
            
            // Extract property number and size (v1-3 format)
            let property_num = size_byte & 0x1F;  // Lower 5 bits
            let property_size = (size_byte >> 5) + 1;  // Upper 3 bits + 1
            
            if property_num == prop_num {
                // Found the property
                return Ok(Some((cursor + 1, property_size)));  // Return data address and size
            }
            
            // Move to next property
            cursor += 1 + property_size as usize;
        }
        
        // Property not found
        Ok(None)
    }
    
    fn get_property_default(&self, prop_num: u8) -> Result<u16, String> {
        if prop_num == 0 || prop_num > 31 {
            return Err("Property number must be 1-31".to_string());
        }
        
        // Property defaults table starts after header
        let prop_defaults_addr = ((self.memory[10] as u16) << 8) | (self.memory[11] as u16);
        let default_addr = (prop_defaults_addr as usize) + ((prop_num - 1) as usize * 2);
        
        if default_addr + 1 >= self.memory.len() {
            return Err("Property defaults table access out of bounds".to_string());
        }
        
        // Read default value (big-endian)
        let default_value = ((self.memory[default_addr] as u16) << 8) | (self.memory[default_addr + 1] as u16);
        Ok(default_value)
    }
    
    // Helper methods for setting object relationships
    fn set_object_parent(&mut self, obj_num: u16, parent: u16) -> Result<(), String> {
        if obj_num == 0 {
            return Err("Cannot set parent of object 0".to_string());
        }
        
        let obj_addr = self.get_object_addr(obj_num)?;
        if obj_addr + 4 >= self.memory.len() {
            return Err("Object table access out of bounds".to_string());
        }
        
        self.memory[obj_addr + 4] = parent as u8;
        Ok(())
    }
    
    fn set_object_sibling(&mut self, obj_num: u16, sibling: u16) -> Result<(), String> {
        if obj_num == 0 {
            return Err("Cannot set sibling of object 0".to_string());
        }
        
        let obj_addr = self.get_object_addr(obj_num)?;
        if obj_addr + 5 >= self.memory.len() {
            return Err("Object table access out of bounds".to_string());
        }
        
        self.memory[obj_addr + 5] = sibling as u8;
        Ok(())
    }
    
    fn set_object_child(&mut self, obj_num: u16, child: u16) -> Result<(), String> {
        if obj_num == 0 {
            return Err("Cannot set child of object 0".to_string());
        }
        
        let obj_addr = self.get_object_addr(obj_num)?;
        if obj_addr + 6 >= self.memory.len() {
            return Err("Object table access out of bounds".to_string());
        }
        
        self.memory[obj_addr + 6] = child as u8;
        Ok(())
    }
    
    // Helper methods for string processing
    pub fn read_zstring_at_address(&self, addr: usize) -> Result<(String, usize), String> {
        use crate::dictionary::Dictionary;
        
        if addr >= self.memory.len() {
            return Err("String address out of bounds".to_string());
        }
        
        // Use the existing ZTextReader implementation
        match Dictionary::read_text(self.game, addr) {
            Ok(text) => {
                // Add safety check to prevent extremely long text output
                if text.len() > 1000 {
                    eprintln!("WARNING: Text too long ({}), truncating. Address: {:#x}", text.len(), addr);
                    let truncated = text.chars().take(100).collect::<String>();
                    return Ok((format!("{}...[TRUNCATED]", truncated), 200));
                }
                
                // Calculate string length by finding the terminating word
                let mut cursor = addr;
                let mut length = 0;
                
                while cursor + 1 < self.memory.len() {
                    let word = ((self.memory[cursor] as u16) << 8) | (self.memory[cursor + 1] as u16);
                    cursor += 2;
                    length += 2;
                    
                    // High bit set means this is the last word
                    if (word & 0x8000) != 0 {
                        break;
                    }
                    
                    // Safety check to prevent infinite loops
                    if length > 200 {
                        eprintln!("WARNING: String length exceeded 200 bytes at address {:#x}", addr);
                        break;
                    }
                }
                
                Ok((text, length))
            },
            Err(e) => Err(format!("Failed to read Z-string: {}", e))
        }
    }
    
    pub fn read_zstring_inline(&mut self) -> Result<(String, usize), String> {
        // Read Z-string starting at current PC
        let (text, length) = self.read_zstring_at_address(self.pc)?;
        
        // Advance PC past the string
        self.pc += length;
        
        Ok((text, length))
    }
    
    pub fn convert_packed_address(&self, packed_addr: u16) -> usize {
        // Convert packed address to byte address based on Z-machine version
        let version = if self.memory.len() > 0 { self.memory[0] } else { 3 };
        
        match version {
            1 | 2 | 3 => (packed_addr as usize) * 2,
            4 | 5 => (packed_addr as usize) * 4,
            6 | 7 | 8 => {
                // Version 6+ uses routine/string base addresses
                let base_high = if self.memory.len() > 29 { 
                    ((self.memory[28] as usize) << 8) | (self.memory[29] as usize)
                } else { 0 };
                (packed_addr as usize) * 4 + base_high
            },
            _ => (packed_addr as usize) * 2, // Default to v1-3 behavior
        }
    }

    // Extended instruction implementations
    fn op_save_undo(&mut self) -> Result<(), String> {
        // SAVE_UNDO saves the current state for UNDO
        // Returns 1 if successful, 0 if failed
        println!("SAVE_UNDO: Mock implementation");
        self.stack.push(1); // Mock success
        Ok(())
    }

    fn op_restore_undo(&mut self) -> Result<(), String> {
        // RESTORE_UNDO restores the saved state
        // Returns 2 if successful, 0 if failed
        println!("RESTORE_UNDO: Mock implementation");
        self.stack.push(2); // Mock success
        Ok(())
    }

    fn op_log_shift(&mut self) -> Result<(), String> {
        // LOG_SHIFT performs logical shift operation
        if self.operands_buffer.len() < 2 {
            return Err("LOG_SHIFT instruction missing operands".to_string());
        }
        
        let number = self.operands_buffer[0] as i16;
        let places = self.operands_buffer[1] as i16;
        
        let result = if places >= 0 {
            // Left shift
            (number as u16) << places
        } else {
            // Right shift
            (number as u16) >> (-places)
        };
        
        self.stack.push(result);
        Ok(())
    }

    fn op_art_shift(&mut self) -> Result<(), String> {
        // ART_SHIFT performs arithmetic shift operation
        if self.operands_buffer.len() < 2 {
            return Err("ART_SHIFT instruction missing operands".to_string());
        }
        
        let number = self.operands_buffer[0] as i16;
        let places = self.operands_buffer[1] as i16;
        
        let result = if places >= 0 {
            // Left shift
            number << places
        } else {
            // Right shift (arithmetic)
            number >> (-places)
        };
        
        self.stack.push(result as u16);
        Ok(())
    }

    fn op_set_font(&mut self) -> Result<(), String> {
        // SET_FONT sets the current font
        if self.operands_buffer.is_empty() {
            return Err("SET_FONT instruction missing operand".to_string());
        }
        
        let font = self.operands_buffer[0];
        println!("SET_FONT: font {} (not implemented)", font);
        
        // Return the previous font (mock)
        self.stack.push(1); // Mock previous font
        Ok(())
    }

    fn op_draw_picture(&mut self) -> Result<(), String> {
        // DRAW_PICTURE draws a picture
        if self.operands_buffer.len() < 3 {
            return Err("DRAW_PICTURE instruction missing operands".to_string());
        }
        
        let picture = self.operands_buffer[0];
        let y = self.operands_buffer[1];
        let x = self.operands_buffer[2];
        
        println!("DRAW_PICTURE: picture {} at ({}, {}) (not implemented)", picture, x, y);
        Ok(())
    }

    fn op_picture_data(&mut self) -> Result<(), String> {
        // PICTURE_DATA gets picture information
        if self.operands_buffer.len() < 2 {
            return Err("PICTURE_DATA instruction missing operands".to_string());
        }
        
        let picture = self.operands_buffer[0];
        let array = self.operands_buffer[1];
        
        println!("PICTURE_DATA: picture {} array {} (not implemented)", picture, array);
        
        // Return 0 (picture not available)
        self.stack.push(0);
        Ok(())
    }

    fn op_erase_picture(&mut self) -> Result<(), String> {
        // ERASE_PICTURE erases a picture
        if self.operands_buffer.len() < 3 {
            return Err("ERASE_PICTURE instruction missing operands".to_string());
        }
        
        let picture = self.operands_buffer[0];
        let y = self.operands_buffer[1];
        let x = self.operands_buffer[2];
        
        println!("ERASE_PICTURE: picture {} at ({}, {}) (not implemented)", picture, x, y);
        Ok(())
    }

    fn op_set_margins(&mut self) -> Result<(), String> {
        // SET_MARGINS sets window margins
        if self.operands_buffer.len() < 3 {
            return Err("SET_MARGINS instruction missing operands".to_string());
        }
        
        let left = self.operands_buffer[0];
        let right = self.operands_buffer[1];
        let window = self.operands_buffer[2];
        
        println!("SET_MARGINS: left {} right {} window {} (not implemented)", left, right, window);
        Ok(())
    }

    fn op_print_unicode(&mut self) -> Result<(), String> {
        // PRINT_UNICODE prints a Unicode character
        if self.operands_buffer.is_empty() {
            return Err("PRINT_UNICODE instruction missing operand".to_string());
        }
        
        let char_code = self.operands_buffer[0];
        
        // Convert to char and print
        if let Some(ch) = char::from_u32(char_code as u32) {
            self.write_output(&ch.to_string())?;
        }
        
        Ok(())
    }

    fn op_check_unicode(&mut self) -> Result<(), String> {
        // CHECK_UNICODE checks if a Unicode character can be printed
        if self.operands_buffer.is_empty() {
            return Err("CHECK_UNICODE instruction missing operand".to_string());
        }
        
        let char_code = self.operands_buffer[0];
        
        // Check if character is printable
        let result = if char_code <= 127 {
            3 // Can print and read
        } else {
            0 // Cannot print
        };
        
        self.stack.push(result);
        Ok(())
    }

    fn op_set_true_colour(&mut self) -> Result<(), String> {
        // SET_TRUE_COLOUR sets true color values
        if self.operands_buffer.len() < 3 {
            return Err("SET_TRUE_COLOUR instruction missing operands".to_string());
        }
        
        let foreground = self.operands_buffer[0];
        let background = self.operands_buffer[1];
        let window = self.operands_buffer[2];
        
        println!("SET_TRUE_COLOUR: fg {} bg {} window {} (not implemented)", foreground, background, window);
        Ok(())
    }

    fn op_move_window(&mut self) -> Result<(), String> {
        // MOVE_WINDOW moves a window
        if self.operands_buffer.len() < 3 {
            return Err("MOVE_WINDOW instruction missing operands".to_string());
        }
        
        let window = self.operands_buffer[0];
        let y = self.operands_buffer[1];
        let x = self.operands_buffer[2];
        
        println!("MOVE_WINDOW: window {} to ({}, {}) (not implemented)", window, x, y);
        Ok(())
    }

    fn op_window_size(&mut self) -> Result<(), String> {
        // WINDOW_SIZE sets window size
        if self.operands_buffer.len() < 3 {
            return Err("WINDOW_SIZE instruction missing operands".to_string());
        }
        
        let window = self.operands_buffer[0];
        let height = self.operands_buffer[1];
        let width = self.operands_buffer[2];
        
        println!("WINDOW_SIZE: window {} size {}x{} (not implemented)", window, width, height);
        Ok(())
    }

    fn op_window_style(&mut self) -> Result<(), String> {
        // WINDOW_STYLE sets window style
        if self.operands_buffer.len() < 3 {
            return Err("WINDOW_STYLE instruction missing operands".to_string());
        }
        
        let window = self.operands_buffer[0];
        let flags = self.operands_buffer[1];
        let operation = self.operands_buffer[2];
        
        println!("WINDOW_STYLE: window {} flags {} op {} (not implemented)", window, flags, operation);
        Ok(())
    }

    fn op_get_wind_prop(&mut self) -> Result<(), String> {
        // GET_WIND_PROP gets window property
        if self.operands_buffer.len() < 2 {
            return Err("GET_WIND_PROP instruction missing operands".to_string());
        }
        
        let window = self.operands_buffer[0];
        let property = self.operands_buffer[1];
        
        println!("GET_WIND_PROP: window {} property {} (not implemented)", window, property);
        
        // Return 0 (property not available)
        self.stack.push(0);
        Ok(())
    }

    fn op_scroll_window(&mut self) -> Result<(), String> {
        // SCROLL_WINDOW scrolls window content
        if self.operands_buffer.len() < 2 {
            return Err("SCROLL_WINDOW instruction missing operands".to_string());
        }
        
        let window = self.operands_buffer[0];
        let pixels = self.operands_buffer[1];
        
        println!("SCROLL_WINDOW: window {} pixels {} (not implemented)", window, pixels);
        Ok(())
    }

    fn op_pop_stack(&mut self) -> Result<(), String> {
        // POP_STACK pops values from user stack
        if self.operands_buffer.len() < 2 {
            return Err("POP_STACK instruction missing operands".to_string());
        }
        
        let items = self.operands_buffer[0];
        let stack_ptr = self.operands_buffer[1];
        
        println!("POP_STACK: {} items from stack {} (not implemented)", items, stack_ptr);
        Ok(())
    }

    fn op_read_mouse(&mut self) -> Result<(), String> {
        // READ_MOUSE reads mouse position
        if self.operands_buffer.is_empty() {
            return Err("READ_MOUSE instruction missing operand".to_string());
        }
        
        let array = self.operands_buffer[0];
        
        println!("READ_MOUSE: array {} (not implemented)", array);
        Ok(())
    }

    fn op_mouse_window(&mut self) -> Result<(), String> {
        // MOUSE_WINDOW sets mouse window
        if self.operands_buffer.is_empty() {
            return Err("MOUSE_WINDOW instruction missing operand".to_string());
        }
        
        let window = self.operands_buffer[0];
        
        println!("MOUSE_WINDOW: window {} (not implemented)", window);
        Ok(())
    }

    fn op_push_stack(&mut self) -> Result<(), String> {
        // PUSH_STACK pushes values to user stack
        if self.operands_buffer.len() < 2 {
            return Err("PUSH_STACK instruction missing operands".to_string());
        }
        
        let value = self.operands_buffer[0];
        let stack_ptr = self.operands_buffer[1];
        
        println!("PUSH_STACK: value {} to stack {} (not implemented)", value, stack_ptr);
        
        // Return 1 (success)
        self.stack.push(1);
        Ok(())
    }

    fn op_put_wind_prop(&mut self) -> Result<(), String> {
        // PUT_WIND_PROP sets window property
        if self.operands_buffer.len() < 3 {
            return Err("PUT_WIND_PROP instruction missing operands".to_string());
        }
        
        let window = self.operands_buffer[0];
        let property = self.operands_buffer[1];
        let value = self.operands_buffer[2];
        
        println!("PUT_WIND_PROP: window {} property {} value {} (not implemented)", window, property, value);
        Ok(())
    }

    fn op_print_form(&mut self) -> Result<(), String> {
        // PRINT_FORM prints formatted text
        if self.operands_buffer.is_empty() {
            return Err("PRINT_FORM instruction missing operand".to_string());
        }
        
        let formatted_table = self.operands_buffer[0];
        
        println!("PRINT_FORM: table {} (not implemented)", formatted_table);
        Ok(())
    }

    fn op_make_menu(&mut self) -> Result<(), String> {
        // MAKE_MENU creates a menu
        if self.operands_buffer.len() < 2 {
            return Err("MAKE_MENU instruction missing operands".to_string());
        }
        
        let number = self.operands_buffer[0];
        let table = self.operands_buffer[1];
        
        println!("MAKE_MENU: number {} table {} (not implemented)", number, table);
        
        // Return 0 (menu not available)
        self.stack.push(0);
        Ok(())
    }

    fn op_picture_table(&mut self) -> Result<(), String> {
        // PICTURE_TABLE sets picture table
        if self.operands_buffer.is_empty() {
            return Err("PICTURE_TABLE instruction missing operand".to_string());
        }
        
        let table = self.operands_buffer[0];
        
        println!("PICTURE_TABLE: table {} (not implemented)", table);
        Ok(())
    }

    fn op_buffer_screen(&mut self) -> Result<(), String> {
        // BUFFER_SCREEN buffers screen contents
        if self.operands_buffer.is_empty() {
            return Err("BUFFER_SCREEN instruction missing operand".to_string());
        }
        
        let mode = self.operands_buffer[0];
        
        println!("BUFFER_SCREEN: mode {} (not implemented)", mode);
        Ok(())
    }

    // Helper methods for SHOW_STATUS
    fn read_global_variable(&self, var_num: u8) -> Result<u16, String> {
        // Global variables are stored in memory starting at the global variables table
        let globals_addr = ((self.memory[12] as u16) << 8) | (self.memory[13] as u16);
        let var_addr = globals_addr + (var_num as u16 * 2);
        
        if var_addr as usize + 1 >= self.memory.len() {
            return Err("Global variable address out of bounds".to_string());
        }
        
        let value = ((self.memory[var_addr as usize] as u16) << 8) | (self.memory[var_addr as usize + 1] as u16);
        Ok(value)
    }

    fn get_object_name(&self, obj_num: u16) -> Result<String, String> {
        // Get the object's property table address
        let obj_addr = self.get_object_addr(obj_num)?;
        
        if obj_addr + 6 >= self.memory.len() {
            return Err("Object address out of bounds".to_string());
        }
        
        // Read property table address (offset 7-8 in object entry)
        let prop_table_addr = ((self.memory[obj_addr + 7] as u16) << 8) | (self.memory[obj_addr + 8] as u16);
        
        if prop_table_addr as usize >= self.memory.len() {
            return Err("Property table address out of bounds".to_string());
        }
        
        // The object name is stored as a Z-string at the beginning of the property table
        // First byte is the length of the short name
        let name_length = self.memory[prop_table_addr as usize] as usize;
        let name_start = prop_table_addr as usize + 1;
        
        if name_length == 0 {
            return Ok("".to_string());
        }
        
        // Read the Z-string name
        match self.read_zstring_at_address(name_start) {
            Ok((name, _)) => Ok(name),
            Err(_) => Ok(format!("Object {}", obj_num)),
        }
    }
}