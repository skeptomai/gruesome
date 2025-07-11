// Test utilities for creating mock ZMachine instances without GameFile complexity
use crate::zmachine::ZMachine;
use std::collections::HashMap;

pub struct MockZMachine {
    pub memory: Vec<u8>,
    pub pc: usize,
    pub stack: Vec<u16>,
    pub call_stack: Vec<crate::zmachine::CallFrame>,
    pub global_vars: HashMap<u8, u16>,
    pub local_vars: [u16; 15],
    pub running: bool,
    pub operands_buffer: Vec<u16>,
    pub current_branch_offset: Option<i16>,
    pub random_seed: u32,
    
    // I/O Stream management
    pub output_streams: Vec<u16>,
    pub input_stream: u16,
    pub memory_stream_addr: Option<u16>,
    pub memory_stream_data: Vec<u8>,
    pub test_output: String,  // For capturing output in tests
    
    // Window management
    pub windows: Vec<crate::zmachine::Window>,
    pub current_window: u16,
    pub screen_height: u16,
    pub screen_width: u16,
}

impl MockZMachine {
    pub fn new() -> Self {
        Self {
            memory: vec![0u8; 4096],
            pc: 0x100,
            stack: Vec::new(),
            call_stack: Vec::new(),
            global_vars: HashMap::new(),
            local_vars: [0; 15],
            running: true,
            operands_buffer: Vec::new(),
            current_branch_offset: None,
            random_seed: 12345,
            
            // Initialize I/O streams
            output_streams: vec![1],
            input_stream: 0,
            memory_stream_addr: None,
            memory_stream_data: Vec::new(),
            test_output: String::new(),
            
            // Initialize window system
            windows: Self::create_default_windows(),
            current_window: 0,
            screen_height: 24,
            screen_width: 80,
        }
    }

    pub fn with_memory_size(size: usize) -> Self {
        let mut mock = Self::new();
        mock.memory = vec![0u8; size];
        mock.memory[0] = 3; // Version 3
        mock
    }

    pub fn setup_version_3(&mut self) {
        self.memory[0] = 3; // Version 3
        // Set up minimal header
        self.memory[6] = 0x01; self.memory[7] = 0x00; // Initial PC at 0x100
        self.memory[10] = 0x00; self.memory[11] = 0x40; // Object table at 0x40
        self.memory[8] = 0x00; self.memory[9] = 0x80; // Dictionary at 0x80
    }

    pub fn setup_routine_at(&mut self, packed_addr: u16, num_locals: u8) {
        let version = self.memory[0];
        let byte_addr = match version {
            1 | 2 | 3 => (packed_addr as usize) * 2,
            4 | 5 => (packed_addr as usize) * 4,
            _ => (packed_addr as usize) * 4,
        };
        
        if byte_addr < self.memory.len() {
            self.memory[byte_addr] = num_locals;
            // Set up local variable defaults for version 1-4
            if version <= 4 {
                let mut addr = byte_addr + 1;
                for _ in 0..num_locals {
                    if addr + 1 < self.memory.len() {
                        self.memory[addr] = 0; // Default value high byte
                        self.memory[addr + 1] = 0; // Default value low byte
                        addr += 2;
                    }
                }
                // Add a simple RET instruction
                if addr < self.memory.len() {
                    self.memory[addr] = 0xB0; // RET opcode
                }
            }
        }
    }
}

// Manual implementations of ZMachine methods for testing
impl MockZMachine {
    pub fn store_variable(&mut self, var: u8, value: u16) -> Result<(), String> {
        match var {
            0 => {
                // Store on stack
                self.stack.push(value);
                Ok(())
            }
            1..=15 => {
                // Local variable
                self.local_vars[(var - 1) as usize] = value;
                Ok(())
            }
            16..=255 => {
                // Global variable
                self.global_vars.insert(var, value);
                Ok(())
            }
        }
    }

    pub fn op_random(&mut self) -> Result<(), String> {
        if self.operands_buffer.is_empty() {
            return Err("RANDOM instruction missing operand".to_string());
        }
        
        let range = self.operands_buffer[0] as i16;
        
        let result = if range > 0 {
            // Generate random number in range 1..=range
            self.random_seed = self.random_seed.wrapping_mul(1103515245).wrapping_add(12345);
            let random_val = ((self.random_seed / 65536) % 32768) as u16;
            (random_val % (range as u16)) + 1
        } else if range < 0 {
            // Set seed and return 0
            self.random_seed = (-range) as u32;
            0
        } else {
            // Re-seed randomly and return 0
            self.random_seed = 1; // For testing, use fixed seed
            0
        };
        
        self.stack.push(result);
        Ok(())
    }

    pub fn op_call_1s(&mut self, operand: u16) -> Result<(), String> {
        let routine_addr = operand;
        
        if routine_addr == 0 {
            // Call to routine 0 returns 0
            self.stack.push(0);
            return Ok(());
        }
        
        // For testing, just simulate the call without full implementation
        self.stack.push(42); // Mock return value
        Ok(())
    }

    pub fn op_call_2s(&mut self, operand1: u16, operand2: u16) -> Result<(), String> {
        let routine_addr = operand1;
        
        if routine_addr == 0 {
            self.stack.push(0);
            return Ok(());
        }
        
        // For testing, just simulate the call
        self.stack.push(operand2); // Return the argument as mock result
        Ok(())
    }

    pub fn op_call_2n(&mut self, operand1: u16, _operand2: u16) -> Result<(), String> {
        let routine_addr = operand1;
        
        if routine_addr == 0 {
            return Ok(());
        }
        
        // For testing, just simulate the call without storing result
        Ok(())
    }

    pub fn op_save(&mut self) -> Result<(), String> {
        // Mock save operation
        println!("SAVE: Mock save operation");
        Ok(())
    }

    pub fn op_restore(&mut self) -> Result<(), String> {
        // Mock restore operation
        println!("RESTORE: Mock restore operation");
        Ok(())
    }

    pub fn op_restart(&mut self) -> Result<(), String> {
        // Mock restart operation
        self.stack.clear();
        self.global_vars.clear();
        self.local_vars = [0; 15];
        self.random_seed = 12345;
        println!("RESTART: Mock restart operation");
        Ok(())
    }

    pub fn op_verify(&mut self) -> Result<(), String> {
        // Mock verify operation - always succeed
        println!("VERIFY: Mock verify operation");
        Ok(())
    }

    pub fn op_tokenise(&mut self) -> Result<(), String> {
        // Mock TOKENISE operation
        // For testing, just parse simple space-separated words
        if self.operands_buffer.len() < 2 {
            return Err("TOKENISE instruction missing operands".to_string());
        }
        
        let text_buffer = self.operands_buffer[0] as usize;
        let parse_buffer = self.operands_buffer[1] as usize;
        
        if text_buffer >= self.memory.len() || parse_buffer >= self.memory.len() {
            return Err("TOKENISE buffer address out of bounds".to_string());
        }
        
        // Get input text from buffer
        let input_length = self.memory[text_buffer + 1] as usize;
        let mut input_text = String::new();
        
        for i in 0..input_length {
            if text_buffer + 2 + i < self.memory.len() {
                input_text.push(self.memory[text_buffer + 2 + i] as char);
            }
        }
        
        // Simple word parsing (split by spaces)
        let words: Vec<&str> = input_text.split_whitespace().collect();
        
        // Get maximum number of words that can be stored
        let max_words = self.memory[parse_buffer] as usize;
        let words_to_store = std::cmp::min(words.len(), max_words);
        
        // Store number of words found
        self.memory[parse_buffer + 1] = words_to_store as u8;
        
        // Store each word
        for (i, word) in words.iter().take(words_to_store).enumerate() {
            let entry_offset = parse_buffer + 2 + (i * 4);
            
            if entry_offset + 3 < self.memory.len() {
                self.memory[entry_offset] = word.len() as u8;        // word length
                self.memory[entry_offset + 1] = 2 + i as u8;         // position in buffer
                self.memory[entry_offset + 2] = 0;                   // dict addr high (not found)
                self.memory[entry_offset + 3] = 0;                   // dict addr low (not found)
            }
        }
        
        Ok(())
    }

    pub fn op_encode_text(&mut self) -> Result<(), String> {
        // Mock ENCODE_TEXT operation
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
        
        // Mock encoding: just store length as first word
        if coded_text_addr + 1 < self.memory.len() {
            self.memory[coded_text_addr] = 0x80;  // Mock encoded word with end bit
            self.memory[coded_text_addr + 1] = length as u8;
        }
        
        Ok(())
    }

    pub fn op_copy_table(&mut self) -> Result<(), String> {
        // Mock COPY_TABLE operation
        if self.operands_buffer.len() < 3 {
            return Err("COPY_TABLE instruction missing operands".to_string());
        }
        
        let first = self.operands_buffer[0] as usize;
        let second = self.operands_buffer[1] as usize;
        let size = self.operands_buffer[2] as i16;
        
        if size == 0 {
            // Zero fill
            let fill_size = first;
            for i in 0..fill_size {
                if second + i < self.memory.len() {
                    self.memory[second + i] = 0;
                }
            }
            return Ok(());
        }
        
        let abs_size = size.abs() as usize;
        
        if first >= self.memory.len() || second >= self.memory.len() {
            return Err("COPY_TABLE address out of bounds".to_string());
        }
        
        if first + abs_size > self.memory.len() || second + abs_size > self.memory.len() {
            return Err("COPY_TABLE operation would exceed memory bounds".to_string());
        }
        
        // Simple copy implementation for testing
        let mut temp_buffer = Vec::with_capacity(abs_size);
        for i in 0..abs_size {
            temp_buffer.push(self.memory[first + i]);
        }
        
        for (i, &byte) in temp_buffer.iter().enumerate() {
            self.memory[second + i] = byte;
        }
        
        Ok(())
    }

    pub fn op_scan_table(&mut self) -> Result<(), String> {
        // Mock SCAN_TABLE operation
        if self.operands_buffer.len() < 3 {
            return Err("SCAN_TABLE instruction missing operands".to_string());
        }
        
        let search_value = self.operands_buffer[0];
        let table_addr = self.operands_buffer[1] as usize;
        let table_len = self.operands_buffer[2] as usize;
        let form = if self.operands_buffer.len() >= 4 {
            self.operands_buffer[3] as u8
        } else {
            0x02 // Default: 2-byte entries, compare full words
        };
        
        if table_addr >= self.memory.len() {
            return Err("SCAN_TABLE table address out of bounds".to_string());
        }
        
        let entry_length = (form & 0x7F) as usize;
        let compare_byte_only = (form & 0x80) != 0;
        
        
        if entry_length == 0 {
            return Err("SCAN_TABLE entry length cannot be zero".to_string());
        }
        
        // Simple search for testing
        for i in 0..table_len {
            let entry_addr = table_addr + (i * entry_length);
            
            if entry_addr + entry_length > self.memory.len() {
                break;
            }
            
            let found = if compare_byte_only {
                let entry_value = self.memory[entry_addr] as u16;
                let search_low_byte = search_value & 0xFF;
                entry_value == search_low_byte
            } else {
                let entry_value = if entry_length >= 2 {
                    ((self.memory[entry_addr] as u16) << 8) | (self.memory[entry_addr + 1] as u16)
                } else {
                    self.memory[entry_addr] as u16
                };
                entry_value == search_value
            };
            
            if found {
                self.store_variable(0, entry_addr as u16)?;
                return Ok(());
            }
        }
        
        // Not found
        self.store_variable(0, 0)?;
        Ok(())
    }

    pub fn op_print_table(&mut self) -> Result<(), String> {
        // Mock PRINT_TABLE operation
        if self.operands_buffer.len() < 2 {
            return Err("PRINT_TABLE instruction missing operands".to_string());
        }
        
        let text_addr = self.operands_buffer[0] as usize;
        let width = self.operands_buffer[1] as usize;
        let height = if self.operands_buffer.len() >= 3 {
            self.operands_buffer[2] as usize
        } else {
            1
        };
        
        if text_addr >= self.memory.len() {
            return Err("PRINT_TABLE text address out of bounds".to_string());
        }
        
        if width == 0 {
            return Err("PRINT_TABLE width cannot be zero".to_string());
        }
        
        // For testing, just print a simple representation
        println!("TABLE: addr={:#x}, width={}, height={}", text_addr, width, height);
        
        Ok(())
    }

    pub fn op_read_char(&mut self) -> Result<(), String> {
        // Mock READ_CHAR operation
        let device = if !self.operands_buffer.is_empty() {
            self.operands_buffer[0]
        } else {
            1
        };
        
        let char_code = match device {
            1 => {
                // Mock keyboard input - return 'A' for testing
                65
            }
            2 => {
                return Err("READ_CHAR from file not implemented".to_string());
            }
            _ => {
                return Err("READ_CHAR: invalid input device".to_string());
            }
        };
        
        self.stack.push(char_code);
        Ok(())
    }
    
    pub fn op_output_stream(&mut self) -> Result<(), String> {
        // Mock OUTPUT_STREAM operation
        if self.operands_buffer.is_empty() {
            return Err("OUTPUT_STREAM instruction missing operand".to_string());
        }
        
        let stream_spec = self.operands_buffer[0] as i16;
        let stream_num = stream_spec.abs() as u16;
        let enable = stream_spec > 0;
        
        match stream_num {
            1 => {
                if enable {
                    if !self.output_streams.contains(&1) {
                        self.output_streams.push(1);
                    }
                } else {
                    self.output_streams.retain(|&x| x != 1);
                }
            }
            2 => {
                if enable {
                    if !self.output_streams.contains(&2) {
                        self.output_streams.push(2);
                    }
                } else {
                    self.output_streams.retain(|&x| x != 2);
                }
            }
            3 => {
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
                        self.flush_memory_stream()?;
                    }
                }
            }
            4 => {
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
    
    pub fn op_input_stream(&mut self) -> Result<(), String> {
        // Mock INPUT_STREAM operation
        if self.operands_buffer.is_empty() {
            return Err("INPUT_STREAM instruction missing operand".to_string());
        }
        
        let stream_num = self.operands_buffer[0];
        
        match stream_num {
            0 => {
                self.input_stream = 0;
            }
            1 => {
                return Err("INPUT_STREAM from file not implemented".to_string());
            }
            _ => {
                return Err(format!("INPUT_STREAM: invalid stream number {}", stream_num));
            }
        }
        
        Ok(())
    }
    
    pub fn op_buffer_mode(&mut self) -> Result<(), String> {
        // Mock BUFFER_MODE operation
        if self.operands_buffer.is_empty() {
            return Err("BUFFER_MODE instruction missing operand".to_string());
        }
        
        let flag = self.operands_buffer[0];
        
        match flag {
            0 | 1 => {
                // Mock implementation - just accept the flag
                Ok(())
            }
            _ => {
                Err(format!("BUFFER_MODE: invalid flag {}", flag))
            }
        }
    }
    
    pub fn flush_memory_stream(&mut self) -> Result<(), String> {
        // Mock flush_memory_stream operation
        if let Some(addr) = self.memory_stream_addr {
            let addr = addr as usize;
            if addr + 2 + self.memory_stream_data.len() > self.memory.len() {
                return Err("OUTPUT_STREAM: memory stream overflow".to_string());
            }
            
            let length = self.memory_stream_data.len() as u16;
            self.memory[addr] = (length >> 8) as u8;
            self.memory[addr + 1] = length as u8;
            
            for (i, &byte) in self.memory_stream_data.iter().enumerate() {
                self.memory[addr + 2 + i] = byte;
            }
            
            self.memory_stream_addr = None;
            self.memory_stream_data.clear();
        }
        Ok(())
    }
    
    pub fn write_output(&mut self, text: &str) -> Result<(), String> {
        // Mock write_output for tests
        for &stream in &self.output_streams.clone() {
            match stream {
                1 => {
                    // Capture screen output for tests
                    self.test_output.push_str(text);
                }
                2 => {
                    // Mock transcript output
                }
                3 => {
                    // Memory output
                    for ch in text.chars() {
                        self.memory_stream_data.push(ch as u8);
                    }
                }
                4 => {
                    // Mock command output
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn create_default_windows() -> Vec<crate::zmachine::Window> {
        vec![
            // Window 0: Lower window (main text window)
            crate::zmachine::Window {
                id: 0,
                top: 1,
                left: 1,
                height: 24,
                width: 80,
                cursor_row: 1,
                cursor_col: 1,
                text_style: 0,
                wrap: true,
                scrolling: true,
            },
            // Window 1: Upper window (status line)
            crate::zmachine::Window {
                id: 1,
                top: 1,
                left: 1,
                height: 0,
                width: 80,
                cursor_row: 1,
                cursor_col: 1,
                text_style: 0,
                wrap: false,
                scrolling: false,
            },
        ]
    }

    pub fn op_split_window(&mut self) -> Result<(), String> {
        if self.operands_buffer.is_empty() {
            return Err("SPLIT_WINDOW instruction missing operand".to_string());
        }
        
        let lines = self.operands_buffer[0];
        
        if lines == 0 {
            // Unsplit screen
            if let Some(upper_window) = self.windows.get_mut(1) {
                upper_window.height = 0;
            }
            if let Some(lower_window) = self.windows.get_mut(0) {
                lower_window.top = 1;
                lower_window.height = self.screen_height;
            }
            self.current_window = 0;
        } else {
            // Split screen
            let upper_lines = std::cmp::min(lines, self.screen_height - 1);
            let lower_lines = self.screen_height - upper_lines;
            
            if let Some(upper_window) = self.windows.get_mut(1) {
                upper_window.top = 1;
                upper_window.height = upper_lines;
                upper_window.width = self.screen_width;
                upper_window.cursor_row = 1;
                upper_window.cursor_col = 1;
            }
            
            if let Some(lower_window) = self.windows.get_mut(0) {
                lower_window.top = upper_lines + 1;
                lower_window.height = lower_lines;
                lower_window.width = self.screen_width;
            }
        }
        
        Ok(())
    }

    pub fn op_set_window(&mut self) -> Result<(), String> {
        if self.operands_buffer.is_empty() {
            return Err("SET_WINDOW instruction missing operand".to_string());
        }
        
        let window_num = self.operands_buffer[0];
        
        match window_num {
            0 => {
                self.current_window = 0;
            }
            1 => {
                if let Some(upper_window) = self.windows.get(1) {
                    if upper_window.height > 0 {
                        self.current_window = 1;
                    } else {
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

    pub fn op_erase_window(&mut self) -> Result<(), String> {
        if self.operands_buffer.is_empty() {
            return Err("ERASE_WINDOW instruction missing operand".to_string());
        }
        
        let window_spec = self.operands_buffer[0] as i16;
        
        match window_spec {
            0 => {
                // Clear lower window
                if let Some(window) = self.windows.get_mut(0) {
                    window.cursor_row = 1;
                    window.cursor_col = 1;
                }
            }
            1 => {
                // Clear upper window
                if let Some(window) = self.windows.get_mut(1) {
                    window.cursor_row = 1;
                    window.cursor_col = 1;
                }
            }
            -1 => {
                // Clear entire screen
                for window in &mut self.windows {
                    if window.height > 0 {
                        window.cursor_row = 1;
                        window.cursor_col = 1;
                    }
                }
            }
            -2 => {
                // Clear entire screen and unsplit
                for window in &mut self.windows {
                    if window.height > 0 {
                        window.cursor_row = 1;
                        window.cursor_col = 1;
                    }
                }
                // Unsplit
                self.operands_buffer = vec![0];
                self.op_split_window()?;
            }
            _ => {
                return Err(format!("ERASE_WINDOW: invalid window specification {}", window_spec));
            }
        }
        
        Ok(())
    }

    pub fn op_erase_line(&mut self) -> Result<(), String> {
        // Mock implementation - just accept the instruction
        Ok(())
    }

    pub fn op_set_cursor(&mut self) -> Result<(), String> {
        if self.operands_buffer.len() < 2 {
            return Err("SET_CURSOR instruction missing operands".to_string());
        }
        
        let row = self.operands_buffer[0];
        let col = self.operands_buffer[1];
        
        if row == 0 || col == 0 {
            return Err("SET_CURSOR: invalid cursor position (1-based)".to_string());
        }
        
        if let Some(window) = self.windows.get_mut(self.current_window as usize) {
            let max_row = window.height;
            let max_col = window.width;
            
            window.cursor_row = std::cmp::min(row, max_row);
            window.cursor_col = std::cmp::min(col, max_col);
        } else {
            return Err("SET_CURSOR: invalid current window".to_string());
        }
        
        Ok(())
    }

    pub fn op_get_cursor(&mut self) -> Result<(), String> {
        if self.operands_buffer.is_empty() {
            return Err("GET_CURSOR instruction missing operand".to_string());
        }
        
        let table_addr = self.operands_buffer[0] as usize;
        
        if table_addr + 3 >= self.memory.len() {
            return Err("GET_CURSOR: table address out of bounds".to_string());
        }
        
        if let Some(window) = self.windows.get(self.current_window as usize) {
            let row = window.cursor_row;
            let col = window.cursor_col;
            
            self.memory[table_addr] = (row >> 8) as u8;
            self.memory[table_addr + 1] = row as u8;
            self.memory[table_addr + 2] = (col >> 8) as u8;
            self.memory[table_addr + 3] = col as u8;
        } else {
            return Err("GET_CURSOR: invalid current window".to_string());
        }
        
        Ok(())
    }

    pub fn op_set_text_style(&mut self) -> Result<(), String> {
        if self.operands_buffer.is_empty() {
            return Err("SET_TEXT_STYLE instruction missing operand".to_string());
        }
        
        let style = self.operands_buffer[0];
        
        if let Some(window) = self.windows.get_mut(self.current_window as usize) {
            window.text_style = style;
        } else {
            return Err("SET_TEXT_STYLE: invalid current window".to_string());
        }
        
        Ok(())
    }
}