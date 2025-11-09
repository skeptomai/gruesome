use crate::header::Header;
use log::debug;
use std::fmt;

/// Maximum size of the VM stack
const STACK_SIZE: usize = 1024;

/// Maximum number of local variables per routine
const MAX_LOCALS: usize = 16;

/// Represents a call frame on the VM call stack
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// Return address (PC to return to)
    pub return_pc: u32,
    /// Where to store the return value (None = discard, Some(n) = variable n)
    pub return_store: Option<u8>,
    /// Number of local variables in this frame
    pub num_locals: u8,
    /// Local variable values
    pub locals: [u16; MAX_LOCALS],
    /// Stack pointer when this routine was called
    pub stack_base: usize,
}

/// Represents a loaded game with owned memory
pub struct Game {
    /// The raw game memory
    pub memory: Vec<u8>,
    /// The parsed header
    pub header: Header,
    /// Original memory for save game compression
    pub original_memory: Option<Vec<u8>>,
}

impl Game {
    /// Create a new game from memory bytes
    pub fn from_memory(memory: Vec<u8>) -> Result<Self, String> {
        if memory.len() < 64 {
            return Err("Game file too small for header".to_string());
        }
        let header = Header::new(&memory);

        // Keep a copy of the original memory for save games
        let original_memory = Some(memory.clone());

        Ok(Game {
            memory,
            header,
            original_memory,
        })
    }
}

/// The Z-Machine virtual machine state
pub struct VM {
    /// The game being executed
    pub game: Game,
    /// Program counter - current instruction address
    pub pc: u32,
    /// Main evaluation stack
    pub stack: Vec<u16>,
    /// Call stack for routine invocations
    pub call_stack: Vec<CallFrame>,
    /// Global variables (stored in memory, but cached here for speed)
    globals_addr: u16,
    /// Current instruction PC (for debugging - set by interpreter before execution)
    pub current_instruction_pc: Option<u32>,
}

impl VM {
    /// Create a new VM instance with the given game
    pub fn new(game: Game) -> Self {
        let initial_pc = game.header.initial_pc as u32;
        let globals_addr = game.header.global_variables as u16;

        // For V1-5, we need to set up an initial "main" context
        // that has no locals but allows stack operations
        let mut vm = VM {
            game,
            pc: initial_pc,
            stack: Vec::with_capacity(STACK_SIZE),
            call_stack: Vec::new(),
            globals_addr,
            current_instruction_pc: None,
        };

        // Set up initial call frame for V1-5 (V6+ uses main routine)
        if vm.game.header.version <= 5 {
            let main_frame = CallFrame {
                return_pc: 0,       // No return from main
                return_store: None, // No return value storage for main
                num_locals: 0,
                locals: [0; 16],
                stack_base: 0,
            };
            vm.call_stack.push(main_frame);
        }

        vm
    }

    /// Reset the VM to initial state
    pub fn reset(&mut self) {
        self.pc = self.game.header.initial_pc as u32;
        self.stack.clear();
        self.call_stack.clear();
    }

    /// Decode an instruction at a specific PC (for debugging)
    pub fn decode_instruction_at(
        &self,
        pc: u32,
    ) -> Result<crate::instruction::Instruction, String> {
        use crate::instruction::Instruction;
        let version = self.game.header.version;
        Instruction::decode(&self.game.memory, pc as usize, version)
            .map_err(|e| format!("Decode error at PC 0x{:04x}: {}", pc, e))
    }

    /// Format an instruction at a specific PC as a human-readable string
    pub fn format_instruction_at(&self, pc: u32) -> String {
        match self.decode_instruction_at(pc) {
            Ok(inst) => format!("{:?}", inst),
            Err(e) => format!("DECODE_ERROR({})", e),
        }
    }

    /// Push a value onto the evaluation stack
    pub fn push(&mut self, value: u16) -> Result<(), String> {
        if self.stack.len() >= STACK_SIZE {
            return Err("Stack overflow".to_string());
        }

        // Optional stack tracing (enable with TRACE_STACK=1)
        if std::env::var("TRACE_STACK").is_ok() {
            log::debug!(
                "📥 PUSH: value=0x{:04x} ({}), PC=0x{:04x}, depth={}",
                value,
                value,
                self.pc,
                self.stack.len()
            );
        }

        // Also log specific interesting values (including 3 which becomes 0xC000)
        if value == 0xC000 || value == 0x0300 || value == 0xC300 || value == 3 {
            let executing_inst = if let Some(pc) = self.current_instruction_pc {
                self.format_instruction_at(pc)
            } else {
                "unknown".to_string()
            };
            log::debug!(
                "📥 PUSH_INTERESTING: value=0x{:04x} ({}), executing_inst_pc=0x{:04x}, depth={}, inst: {}",
                value, value, self.current_instruction_pc.unwrap_or(self.pc), self.stack.len(), executing_inst
            );
        }

        self.stack.push(value);
        Ok(())
    }

    /// Pop a value from the evaluation stack
    pub fn pop(&mut self) -> Result<u16, String> {
        if self.stack.is_empty() {
            log::debug!(
                " STACK UNDERFLOW: Attempted to pop from empty stack at PC 0x{:04x}",
                self.pc
            );
            log::debug!(
                " Stack state: depth={}, call_stack_depth={}",
                self.stack.len(),
                self.call_stack.len()
            );
            if let Some(frame) = self.call_stack.last() {
                log::debug!(
                    " Current routine: return_PC={:04x}, locals={}",
                    frame.return_pc,
                    frame.locals.len()
                );
            }

            // Add bytecode analysis at underflow point
            log::debug!(
                " Bytecode at PC 0x{:04x}: {:02x} {:02x} {:02x} {:02x} {:02x}",
                self.pc,
                self.game.memory.get(self.pc as usize).unwrap_or(&0xff),
                self.game
                    .memory
                    .get((self.pc + 1) as usize)
                    .unwrap_or(&0xff),
                self.game
                    .memory
                    .get((self.pc + 2) as usize)
                    .unwrap_or(&0xff),
                self.game
                    .memory
                    .get((self.pc + 3) as usize)
                    .unwrap_or(&0xff),
                self.game
                    .memory
                    .get((self.pc + 4) as usize)
                    .unwrap_or(&0xff)
            );

            // Add stack trace to see what's calling pop()
            log::debug!(" STACK UNDERFLOW BACKTRACE:");
            let backtrace = std::backtrace::Backtrace::capture();
            log::debug!("{}", backtrace);

            return Err("Stack underflow".to_string());
        }

        let value = self.stack.pop().unwrap();

        // Optional stack tracing (enable with TRACE_STACK=1)
        if std::env::var("TRACE_STACK").is_ok() {
            log::debug!(
                "📤 POP: value=0x{:04x} ({}), PC=0x{:04x}, depth={}",
                value,
                value,
                self.pc,
                self.stack.len()
            );
        }

        // Also log specific interesting values (including 3 which becomes 0xC000)
        if value == 0xC000 || value == 0x0300 || value == 0xC300 || value == 3 {
            log::debug!(
                "📤 POP_INTERESTING: value=0x{:04x} ({}), PC=0x{:04x}, depth={}",
                value,
                value,
                self.pc,
                self.stack.len()
            );
        }

        Ok(value)
    }

    /// Peek at the top of the stack without removing it
    pub fn peek(&self) -> Result<u16, String> {
        if self.stack.is_empty() {
            log::debug!(
                "STACK UNDERFLOW: Stack peek attempted on empty stack. PC: 0x{:04x}",
                self.pc
            );
            log::debug!(" Call stack depth: {}", self.call_stack.len());
            log::debug!(" Last few instructions executed would help debug this...");

            // Try to decode the current instruction to understand what caused this
            if self.pc < self.game.memory.len() as u32 {
                let opcode = self.game.memory[self.pc as usize];
                log::debug!(
                    " Current instruction opcode: 0x{:02x} at PC 0x{:04x}",
                    opcode,
                    self.pc
                );
            }

            return Err("Stack is empty".to_string());
        }
        self.stack
            .last()
            .copied()
            .ok_or_else(|| "Stack is empty".to_string())
    }

    /// Read a byte from memory
    pub fn read_byte(&self, addr: u32) -> u8 {
        if addr < self.game.memory.len() as u32 {
            self.game.memory[addr as usize]
        } else {
            0
        }
    }

    /// Read a word (2 bytes, big-endian) from memory
    pub fn read_word(&self, addr: u32) -> u16 {
        let high = self.read_byte(addr) as u16;
        let low = self.read_byte(addr + 1) as u16;
        (high << 8) | low
    }

    /// Write a byte to memory (only in dynamic memory)
    pub fn write_byte(&mut self, addr: u32, value: u8) -> Result<(), String> {
        let dynamic_limit = self.game.header.base_static_mem as u32;
        if addr >= dynamic_limit {
            return Err(format!(
                "Attempt to write to non-dynamic memory at {addr:04x}"
            ));
        }
        if addr < self.game.memory.len() as u32 {
            self.game.memory[addr as usize] = value;
            Ok(())
        } else {
            Err(format!("Memory address out of bounds: {addr:04x}"))
        }
    }

    /// Write a word to memory (only in dynamic memory)
    pub fn write_word(&mut self, addr: u32, value: u16) -> Result<(), String> {
        // Track writes to score/moves globals at 0x42 (score) and 0x44 (moves)
        if addr == 0x42 {
            log::debug!(
                "🚨 SCORE CORRUPTION: Writing 0x{:04x} ({}) to addr 0x{:04x} at PC=0x{:04x}",
                value,
                value,
                addr,
                self.pc
            );
            log::debug!(
                "   Stack depth: {}, top 5 values: {:?}",
                self.stack.len(),
                self.stack.iter().rev().take(5).collect::<Vec<_>>()
            );
            log::debug!("   Call stack depth: {}", self.call_stack.len());
            if let Some(frame) = self.call_stack.last() {
                log::debug!(
                    "   Current function: PC start=0x{:04x}, locals: {:?}",
                    frame.return_pc,
                    &frame.locals[0..frame.num_locals as usize]
                );
            }
        }
        self.write_byte(addr, (value >> 8) as u8)?;
        self.write_byte(addr + 1, (value & 0xFF) as u8)?;
        Ok(())
    }

    /// Read a global variable (0x10-0xFF)
    pub fn read_global(&self, var: u8) -> Result<u16, String> {
        if var < 0x10 {
            return Err(format!("Invalid global variable number: {var:02x}"));
        }
        let offset = ((var - 0x10) as u32) * 2;
        let addr = self.globals_addr as u32 + offset;
        let value = self.read_word(addr);

        // Debug logging for critical globals
        if var == 0x10 {
            // Global G00 - Player object reference
            debug!(
 "Reading global 0x{:02x} (G00/player) from addr 0x{:04x} = 0x{:04x} ({}) at PC {:05x}",
 var, addr, value, value, self.pc
 );
        }
        if var == 0x52 {
            // LIT variable
            debug!(
                "Reading global 0x{:02x} (LIT) from addr 0x{:04x} = 0x{:04x} ({}) at PC {:05x}",
                var, addr, value, value, self.pc
            );
        }

        Ok(value)
    }

    /// Write a global variable (0x10-0xFF)
    pub fn write_global(&mut self, var: u8, value: u16) -> Result<(), String> {
        if var < 0x10 {
            return Err(format!("Invalid global variable number: {var:02x}"));
        }
        let offset = ((var - 0x10) as u32) * 2;
        let addr = self.globals_addr as u32 + offset;

        // Debug logging for critical globals
        // Track writes to score (global 17 = var 0x11) and moves (global 18 = var 0x12)
        if var == 0x11 || var == 0x12 {
            log::debug!(
                "🚨 SCORE/MOVES WRITE_GLOBAL: var=0x{:02x} (G{:02}), addr=0x{:04x}, value=0x{:04x} ({}), PC=0x{:04x}",
                var,
                var - 0x10,
                addr,
                value,
                value,
                self.pc
            );
        }
        if var == 0x52 {
            // LIT variable
            let old_value = self.read_word(addr);
            debug!(
                "Writing global 0x{:02x} (LIT) at addr 0x{:04x}: {} -> {} at PC {:05x}",
                var, addr, old_value, value, self.pc
            );
        }

        self.write_word(addr, value)
    }

    /// Read a variable (0x00 = stack, 0x01-0x0F = local, 0x10-0xFF = global)
    pub fn read_variable(&self, var: u8) -> Result<u16, String> {
        let result = match var {
            0x00 => {
                log::debug!(
                    "Reading from stack (Variable 0x00) at PC 0x{:04x}, stack size: {}",
                    self.pc,
                    self.stack.len()
                );
                self.peek()
            }
            0x01..=0x0F => {
                // Local variable
                let frame = self
                    .call_stack
                    .last()
                    .ok_or("No active routine for local variable access")?;
                let index = (var - 1) as usize;
                if index >= frame.num_locals as usize {
                    debug!("WARNING: Reading local variable {} but routine only has {} locals - returning 0", 
 var, frame.num_locals);
                    return Ok(0);
                }
                Ok(frame.locals[index])
            }
            _ => self.read_global(var),
        };

        // Debug logging for critical variable reads
        if var == 1 {
            log::debug!(
                "🔍 READ_VAR: var={}, value={:04x?}, PC=0x{:04x}",
                var,
                result,
                self.pc
            );
        }
        // Log reads of variables used in exit corruption chain
        if var == 236 || var == 237 || var == 239 {
            if let Ok(val) = result {
                log::debug!(
                    "🔍 READ_VAR_{}: value=0x{:04x} ({}), PC=0x{:04x}",
                    var,
                    val,
                    val,
                    self.current_instruction_pc.unwrap_or(self.pc)
                );
            }
        }
        if var == 0x10 {
            debug!(
                "read_variable(0x{:02x}) [Variable(16)/G00] at PC {:05x} returning value: {:?}",
                var, self.pc, result
            );
        }
        if var == 0x52 && self.pc >= 0x8d50 && self.pc <= 0x8d60 {
            debug!(
                "read_variable(0x{:02x}) at PC {:05x} returning value: {:?}",
                var, self.pc, result
            );
        }

        result
    }

    /// Write a variable (0x00 = stack, 0x01-0x0F = local, 0x10-0xFF = global)
    pub fn write_variable(&mut self, var: u8, value: u16) -> Result<(), String> {
        // Track writes to score (variable 17 = 0x11 = global G01)
        if var == 0x11 {
            log::debug!(
                "🚨 SCORE WRITE: var=0x{:02x} (G01/score), value=0x{:04x} ({}), PC=0x{:04x}",
                var,
                value,
                value,
                self.pc
            );
        }
        // Log writes to Variable 2 (exit local variable)
        if var == 2 {
            // Get current instruction bytes for debugging
            let inst_bytes = if self.pc < self.game.memory.len() as u32 {
                let pc = self.pc as usize;
                let end = (pc + 8).min(self.game.memory.len());
                &self.game.memory[pc..end]
            } else {
                &[]
            };
            // Show call stack depth to distinguish frames
            let stack_depth = self.call_stack.len();
            log::error!(
                "🔧 WRITE_VAR_2: value=0x{:04x} ({}), PC=0x{:04x}, frame_depth={}, inst_bytes={:02x?}",
                value,
                value,
                self.pc,
                stack_depth,
                inst_bytes
            );

            // Dump entire call stack with return addresses to "weave together" execution flow
            log::debug!("🔍 CALL_STACK (depth={}):", self.call_stack.len());
            for (i, frame) in self.call_stack.iter().enumerate() {
                log::debug!(
                    "  Frame[{}]: return_pc=0x{:04x}, num_locals={}, stack_base={}, return_store={:?}",
                    i,
                    frame.return_pc,
                    frame.num_locals,
                    frame.stack_base,
                    frame.return_store
                );
            }

            // Also show memory around current PC to understand context
            if self.pc >= 10 && (self.pc as usize) < self.game.memory.len() {
                let start = (self.pc as usize) - 10;
                let end = ((self.pc as usize) + 10).min(self.game.memory.len());
                log::debug!(
                    "🔍 MEMORY_CONTEXT (PC-10 to PC+10): {:02x?}",
                    &self.game.memory[start..end]
                );
            }
        }
        // Log writes to Variable 3 (literal pattern debugging)
        if var == 3 {
            let stack_depth = self.call_stack.len();
            log::error!(
                "🔧 WRITE_VAR_3: value=0x{:04x} ({}), PC=0x{:04x}, frame_depth={}",
                value,
                value,
                self.pc,
                stack_depth
            );
        }
        // Log writes to Variable 216 (0xD8) - source of corruption
        if var == 216 {
            // Decode the instruction at current PC
            let inst_str = self.format_instruction_at(self.pc);

            log::debug!(
                "🔍 WRITE_VAR_216: value=0x{:04x} ({}), PC=0x{:04x}, frame_depth={}, next_inst: {}",
                value,
                value,
                self.pc,
                self.call_stack.len(),
                inst_str
            );
        }
        // Log writes to Variables 236 and 239 (used in loadb that reads value 3 from address 0)
        if var == 236 || var == 239 {
            log::debug!(
                "🔍 WRITE_VAR_{}: value=0x{:04x} ({}), PC=0x{:04x}",
                var,
                value,
                value,
                self.current_instruction_pc.unwrap_or(self.pc)
            );
        }
        // Log writes to variables 235-244 (used by get_exit builtin) and Variable(1) (direction parameter)
        if (var >= 235 && var <= 244) || var == 1 {
            log::debug!(
                "🔍 WRITE_VAR: var={}, value=0x{:04x}, PC=0x{:04x}",
                var,
                value,
                self.pc
            );
        }
        match var {
            0x00 => {
                // Writing to variable 0 means push onto stack
                self.push(value)
            }
            0x01..=0x0F => {
                // Local variable
                let frame = self
                    .call_stack
                    .last_mut()
                    .ok_or("No active routine for local variable access")?;
                let index = (var - 1) as usize;
                if index >= frame.num_locals as usize {
                    debug!("WARNING: Writing to local variable {} but routine only has {} locals - ignoring", 
 var, frame.num_locals);
                    return Ok(());
                }
                frame.locals[index] = value;
                Ok(())
            }
            _ => self.write_global(var, value),
        }
    }

    /// Get the current call depth
    pub fn call_depth(&self) -> usize {
        self.call_stack.len()
    }

    /// Check if we're in the main routine (no call stack)
    pub fn is_main_routine(&self) -> bool {
        self.call_stack.is_empty()
    }

    /// Get the value of an object property
    pub fn get_property(&self, obj_num: u16, prop_num: u8) -> Result<u16, String> {
        if obj_num == 0 {
            return Ok(0); // Object 0 has no properties
        }
        let max_objects = if self.game.header.version <= 3 {
            255
        } else {
            65535
        };
        if obj_num > max_objects {
            log::debug!(
                "Object validation error at PC 0x{:04x}: invalid object {} exceeds maximum {}",
                self.pc,
                obj_num,
                max_objects
            );
            return Err(format!("Invalid object number: {obj_num}"));
        }

        // Search in object's property table first
        let prop_addr = self.get_property_addr(obj_num, prop_num)?;
        if prop_addr != 0 {
            // Property found in object - read its value
            let prop_info = self.get_property_info(prop_addr - 1)?;
            let prop_size = prop_info.1;

            if prop_size == 1 {
                return Ok(self.read_byte(prop_addr as u32) as u16);
            } else {
                return Ok(self.read_word(prop_addr as u32));
            }
        }

        // Property not found in object, return default
        let max_defaults = if self.game.header.version <= 3 {
            31
        } else {
            63
        };
        if prop_num > 0 && prop_num <= max_defaults {
            let obj_table_addr = self.game.header.object_table_addr;
            let default_addr = obj_table_addr + ((prop_num - 1) as usize * 2);
            Ok(self.read_word(default_addr as u32))
        } else {
            Ok(0)
        }
    }

    /// Parse property size byte to get property number, size, and header size
    fn get_property_info(&self, prop_addr: usize) -> Result<(u8, usize, usize), String> {
        let size_byte = self.game.memory[prop_addr];

        if self.game.header.version <= 3 {
            // V1-3: prop num in bottom 5 bits
            let prop_num = size_byte & 0x1F;

            // Check for two-byte format (bit 7 set, bit 6 clear)
            if size_byte & 0x80 != 0 {
                // Two-byte header: next byte contains size
                let size_byte_2 = self.game.memory[prop_addr + 1];
                let prop_size = if size_byte_2 == 0 {
                    64
                } else {
                    size_byte_2 as usize
                };
                Ok((prop_num, prop_size, 2))
            } else {
                // Single-byte format: size in top 3 bits (bits 7-5)
                let prop_size = ((size_byte >> 5) & 0x07) + 1;
                Ok((prop_num, prop_size as usize, 1))
            }
        } else {
            // V4+: prop num in bottom 6 bits
            let prop_num = size_byte & 0x3F;

            if size_byte & 0x80 != 0 {
                // Two-byte header
                let size_byte_2 = self.game.memory[prop_addr + 1];
                let size_val = size_byte_2 & 0x3F;
                let prop_size = if size_val == 0 { 64 } else { size_val as usize };
                Ok((prop_num, prop_size, 2))
            } else if size_byte & 0x40 != 0 {
                // Bit 6 set: size 2
                Ok((prop_num, 2, 1))
            } else {
                // Bit 6 clear: size 1
                Ok((prop_num, 1, 1))
            }
        }
    }

    /// Get the address of an object property's data
    pub fn get_property_addr(&self, obj_num: u16, prop_num: u8) -> Result<usize, String> {
        if obj_num == 0 {
            return Ok(0); // Object 0 has no properties
        }
        let max_objects = if self.game.header.version <= 3 {
            255
        } else {
            65535
        };
        if obj_num > max_objects {
            log::debug!(
                "Object validation error at PC 0x{:04x}: invalid object {} exceeds maximum {}",
                self.pc,
                obj_num,
                max_objects
            );
            return Err(format!("Invalid object number: {obj_num}"));
        }

        // Get object table base
        let obj_table_addr = self.game.header.object_table_addr;
        let property_defaults = obj_table_addr;
        let default_props = if self.game.header.version <= 3 {
            31
        } else {
            63
        };
        let obj_tree_base = property_defaults + default_props * 2;

        // Calculate object entry address
        let obj_entry_size = if self.game.header.version <= 3 { 9 } else { 14 };
        let obj_addr = obj_tree_base + ((obj_num - 1) as usize * obj_entry_size);

        // Get property table address
        let prop_addr_offset = if self.game.header.version <= 3 { 7 } else { 12 };
        let prop_table_addr = self.read_word((obj_addr + prop_addr_offset) as u32) as usize;

        // Skip the description byte length
        let desc_len = self.game.memory[prop_table_addr] as usize;
        let mut prop_addr = prop_table_addr + 1 + desc_len * 2;

        // Search for the property
        loop {
            let size_byte = self.game.memory[prop_addr];
            if size_byte == 0 {
                // Log when we hit terminator while searching for properties 20-22
                if prop_num >= 20 && prop_num <= 22 {
                    log::debug!("🔍 get_property_addr: obj={}, prop={} -> NOT FOUND (hit terminator at addr=0x{:04x})",
                        obj_num, prop_num, prop_addr);
                }
                return Ok(0); // Property not found
            }

            let (prop_id, prop_size, size_bytes) = self.get_property_info(prop_addr)?;

            // Log each property we encounter when searching for properties 20-22
            if prop_num >= 20 && prop_num <= 22 {
                log::debug!("🔍 PROP_SEARCH: obj={}, looking_for={}, found_prop={} at addr=0x{:04x}, size_byte=0x{:02x}, prop_size={}, size_bytes={}",
                    obj_num, prop_num, prop_id, prop_addr, size_byte, prop_size, size_bytes);
            }

            if prop_id == prop_num {
                // Found the property - return address of data
                let data_addr = prop_addr + size_bytes;
                if prop_num >= 20 && prop_num <= 22 {
                    log::debug!(
                        "🔍 get_property_addr: obj={}, prop={} -> addr=0x{:04x}, size={}",
                        obj_num,
                        prop_num,
                        data_addr,
                        prop_size
                    );
                }
                return Ok(data_addr);
            }

            // Move to next property
            prop_addr += size_bytes + prop_size;
        }
    }

    /// Write a value to an object property
    pub fn put_property(&mut self, obj_num: u16, prop_num: u8, value: u16) -> Result<(), String> {
        // We need to find the property in the object's property table

        let max_objects = if self.game.header.version <= 3 {
            255
        } else {
            65535
        };
        if obj_num == 0 || obj_num > max_objects {
            log::debug!(
                "Object validation error at PC 0x{:04x}: invalid object {} (zero or exceeds maximum {})",
                self.pc,
                obj_num,
                max_objects
            );
            return Err(format!("Invalid object number: {obj_num}"));
        }

        // Get object table base
        let obj_table_addr = self.game.header.object_table_addr;
        let property_defaults = obj_table_addr;
        let default_props = if self.game.header.version <= 3 {
            31
        } else {
            63
        };
        let obj_tree_base = property_defaults + default_props * 2;

        // Calculate object entry address
        let obj_entry_size = if self.game.header.version <= 3 { 9 } else { 14 };
        let obj_addr = obj_tree_base + ((obj_num - 1) as usize * obj_entry_size);

        // Get property table address
        let prop_addr_offset = if self.game.header.version <= 3 { 7 } else { 12 };
        let prop_table_addr = self.read_word((obj_addr + prop_addr_offset) as u32) as usize;
        debug!(
            "put_property: obj_addr=0x{:04x}, prop_table_addr=0x{:04x}",
            obj_addr, prop_table_addr
        );

        // Skip the description byte length
        let desc_len = self.game.memory[prop_table_addr] as usize;
        let mut prop_addr = prop_table_addr + 1 + desc_len * 2;
        debug!(
            "put_property: desc_len={}, prop_addr=0x{:04x}",
            desc_len, prop_addr
        );

        // Search for the property
        loop {
            let size_byte = self.game.memory[prop_addr];
            debug!(
                "put_property: checking prop_addr=0x{:04x}, size_byte=0x{:02x}",
                prop_addr, size_byte
            );
            if size_byte == 0 {
                return Err(format!(
                    "Property {prop_num} not found for object {obj_num}"
                ));
            }

            let (prop_id, prop_size, size_bytes) = self.get_property_info(prop_addr)?;
            debug!(
                "put_property: prop_id={}, prop_size={}, size_bytes={}",
                prop_id, prop_size, size_bytes
            );

            if prop_id == prop_num {
                // Found the property - write the value
                if prop_size == 1 {
                    self.write_byte((prop_addr + size_bytes) as u32, value as u8)?;
                } else if prop_size == 2 {
                    self.write_word((prop_addr + size_bytes) as u32, value)?;
                } else {
                    return Err(format!(
                        "Property {prop_num} has size {prop_size} (>2), cannot use put_prop"
                    ));
                }
                return Ok(());
            }

            // Move to next property
            prop_addr += size_bytes + prop_size;
        }
    }

    /// Get the next property number after a given property
    pub fn get_next_property(&self, obj_num: u16, prop_num: u8) -> Result<u8, String> {
        if obj_num == 0 {
            return Ok(0); // Object 0 has no properties
        }
        let max_objects = if self.game.header.version <= 3 {
            255
        } else {
            65535
        };
        if obj_num > max_objects {
            log::debug!(
                "Object validation error at PC 0x{:04x}: invalid object {} exceeds maximum {}",
                self.pc,
                obj_num,
                max_objects
            );
            return Err(format!("Invalid object number: {obj_num}"));
        }

        // Get object table base
        let obj_table_addr = self.game.header.object_table_addr;
        let property_defaults = obj_table_addr;
        let default_props = if self.game.header.version <= 3 {
            31
        } else {
            63
        };
        let obj_tree_base = property_defaults + default_props * 2;

        // Calculate object entry address
        let obj_entry_size = if self.game.header.version <= 3 { 9 } else { 14 };
        let obj_addr = obj_tree_base + ((obj_num - 1) as usize * obj_entry_size);

        // Get property table address
        let prop_addr_offset = if self.game.header.version <= 3 { 7 } else { 12 };
        let prop_table_addr = self.read_word((obj_addr + prop_addr_offset) as u32) as usize;

        // Skip the description byte length
        let desc_len = self.game.memory[prop_table_addr] as usize;
        let mut prop_addr = prop_table_addr + 1 + desc_len * 2;

        // If prop_num is 0, return the first property
        if prop_num == 0 {
            let size_byte = self.game.memory[prop_addr];
            if size_byte == 0 {
                return Ok(0); // No properties
            }
            let (prop_id, _, _) = self.get_property_info(prop_addr)?;
            return Ok(prop_id);
        }

        // Search for the given property, then return the next one
        loop {
            let size_byte = self.game.memory[prop_addr];
            if size_byte == 0 {
                return Ok(0); // End of properties
            }

            let (prop_id, prop_size, size_bytes) = self.get_property_info(prop_addr)?;

            if prop_id == prop_num {
                // Found the property, now get the next one
                prop_addr += size_bytes + prop_size;
                let next_size_byte = self.game.memory[prop_addr];
                if next_size_byte == 0 {
                    return Ok(0); // No next property
                }
                let (next_prop_id, _, _) = self.get_property_info(prop_addr)?;
                return Ok(next_prop_id);
            }

            // Move to next property
            prop_addr += size_bytes + prop_size;
        }
    }

    /// Test if an object has a specific attribute
    pub fn test_attribute(&self, obj_num: u16, attr_num: u8) -> Result<bool, String> {
        if obj_num == 0 {
            return Ok(false); // Object 0 has no attributes
        }

        // OBJECT_BOUNDS_CHECK: Handle invalid object IDs gracefully (architectural issue with grammar system)
        let max_objects = if self.game.header.version <= 3 {
            255
        } else {
            65535
        };
        if obj_num > max_objects {
            log::debug!("OBJECT_BOUNDS_CHECK: Returning false for test_attribute on invalid object ID {} (max: {})", obj_num, max_objects);
            return Ok(false); // Return false for invalid objects
        }

        let obj_addr = self.get_object_addr(obj_num)?;

        let max_attrs = if self.game.header.version <= 3 {
            31
        } else {
            47
        };
        if attr_num > max_attrs {
            debug!(
                "WARNING: test_attribute with invalid attribute {} - returning false",
                attr_num
            );
            return Ok(false);
        }

        // Attributes are in the first bytes of the object entry
        // Z-Machine uses big-endian attribute storage:
        // Byte 0: attrs 31-24, Byte 1: attrs 23-16, Byte 2: attrs 15-8, Byte 3: attrs 7-0
        let attr_bytes_total = if self.game.header.version <= 3 { 4 } else { 6 };
        let attr_byte = (attr_bytes_total - 1) - (attr_num / 8);
        let attr_bit = attr_num % 8; // Bit position within the byte (0=LSB, 7=MSB)

        let byte_val = self.game.memory[obj_addr + attr_byte as usize];
        let is_set = (byte_val & (1 << attr_bit)) != 0;

        // DEBUG: Trace attribute tests for objects 1-15
        if obj_num <= 15 {
            log::debug!(
                "🔍 ATTR_TEST: obj={} attr={} addr=0x{:04x} byte_offset={} bit_pos={} byte_val=0x{:02x} mask=0x{:02x} result={}",
                obj_num, attr_num, obj_addr, attr_byte, attr_bit, byte_val, (1 << attr_bit), is_set
            );
        }

        Ok(is_set)
    }

    /// Set or clear an object attribute
    pub fn set_attribute(&mut self, obj_num: u16, attr_num: u8, value: bool) -> Result<(), String> {
        log::debug!(
            "set_attribute called: PC=0x{:04x}, obj={}, attr={}, value={}",
            self.pc,
            obj_num,
            attr_num,
            value
        );
        if obj_num == 0 {
            return Ok(()); // Cannot set attributes on object 0
        }

        // OBJECT_BOUNDS_CHECK: Handle invalid object IDs gracefully (architectural issue with grammar system)
        let max_objects = if self.game.header.version <= 3 {
            255
        } else {
            65535
        };
        if obj_num > max_objects {
            log::debug!(
                "OBJECT_BOUNDS_CHECK: Skipping set_attribute for invalid object ID {} (max: {})",
                obj_num,
                max_objects
            );
            return Ok(()); // Skip invalid operations gracefully
        }

        let obj_addr = self.get_object_addr(obj_num)?;

        let max_attrs = if self.game.header.version <= 3 {
            31
        } else {
            47
        };
        if attr_num > max_attrs {
            debug!(
                "WARNING: set_attribute with invalid attribute {} - ignoring",
                attr_num
            );
            return Ok(());
        }

        // Attributes are in the first bytes of the object entry
        // Z-Machine uses big-endian attribute storage:
        // Byte 0: attrs 31-24, Byte 1: attrs 23-16, Byte 2: attrs 15-8, Byte 3: attrs 7-0
        let attr_bytes_total = if self.game.header.version <= 3 { 4 } else { 6 };
        let attr_byte = (attr_bytes_total - 1) - (attr_num / 8);
        let attr_bit = attr_num % 8; // Bit position within the byte (0=LSB, 7=MSB)

        let byte_val = self.game.memory[obj_addr + attr_byte as usize];
        let new_byte = if value {
            byte_val | (1 << attr_bit) // Set bit
        } else {
            byte_val & !(1 << attr_bit) // Clear bit
        };

        self.game.memory[obj_addr + attr_byte as usize] = new_byte;
        Ok(())
    }

    /// Get object address
    fn get_object_addr(&self, obj_num: u16) -> Result<usize, String> {
        let max_objects = if self.game.header.version <= 3 {
            255
        } else {
            65535
        };
        if obj_num == 0 || obj_num > max_objects {
            log::debug!(
                "Object validation error at PC 0x{:04x}: invalid object {} (zero or exceeds maximum {})",
                self.pc,
                obj_num,
                max_objects
            );
            return Err(format!("Invalid object number: {obj_num}"));
        }

        let obj_table_addr = self.game.header.object_table_addr;
        let property_defaults = obj_table_addr;
        let default_props = if self.game.header.version <= 3 {
            31
        } else {
            63
        };
        let obj_tree_base = property_defaults + default_props * 2;
        let obj_entry_size = if self.game.header.version <= 3 { 9 } else { 14 };

        Ok(obj_tree_base + ((obj_num - 1) as usize * obj_entry_size))
    }

    /// Get parent of object
    pub fn get_parent(&self, obj_num: u16) -> Result<u16, String> {
        if obj_num == 0 {
            return Ok(0); // Object 0 has no parent
        }
        let obj_addr = self.get_object_addr(obj_num)?;
        if self.game.header.version <= 3 {
            Ok(self.game.memory[obj_addr + 4] as u16)
        } else {
            Ok(self.read_word((obj_addr + 6) as u32))
        }
    }

    /// Set parent of object
    pub fn set_parent(&mut self, obj_num: u16, parent: u16) -> Result<(), String> {
        if obj_num == 0 {
            return Err("Cannot set parent of object 0".to_string());
        }
        let obj_addr = self.get_object_addr(obj_num)?;
        if self.game.header.version <= 3 {
            if parent > 255 {
                return Err(format!("Parent object number too large for v3: {parent}"));
            }
            self.game.memory[obj_addr + 4] = parent as u8;
        } else {
            self.write_word((obj_addr + 6) as u32, parent)?;
        }
        Ok(())
    }

    /// Get sibling of object
    pub fn get_sibling(&self, obj_num: u16) -> Result<u16, String> {
        if obj_num == 0 {
            return Ok(0); // Object 0 has no sibling
        }
        let obj_addr = self.get_object_addr(obj_num)?;
        let sibling = if self.game.header.version <= 3 {
            self.game.memory[obj_addr + 5] as u16
        } else {
            self.read_word((obj_addr + 8) as u32)
        };

        // Return sibling object number (0 = no sibling)
        Ok(sibling)
    }

    /// Set sibling of object
    pub fn set_sibling(&mut self, obj_num: u16, sibling: u16) -> Result<(), String> {
        if obj_num == 0 {
            return Err("Cannot set sibling of object 0".to_string());
        }
        let obj_addr = self.get_object_addr(obj_num)?;
        if self.game.header.version <= 3 {
            if sibling > 255 {
                return Err(format!("Sibling object number too large for v3: {sibling}"));
            }
            self.game.memory[obj_addr + 5] = sibling as u8;
        } else {
            self.write_word((obj_addr + 8) as u32, sibling)?;
        }
        Ok(())
    }

    /// Get child of object
    pub fn get_child(&self, obj_num: u16) -> Result<u16, String> {
        if obj_num == 0 {
            return Ok(0); // Object 0 has no child
        }
        let obj_addr = self.get_object_addr(obj_num)?;
        let child = if self.game.header.version <= 3 {
            self.game.memory[obj_addr + 6] as u16
        } else {
            self.read_word((obj_addr + 10) as u32)
        };

        // Return first child object number (0 = no children)
        Ok(child)
    }

    /// Set child of object
    pub fn set_child(&mut self, obj_num: u16, child: u16) -> Result<(), String> {
        if obj_num == 0 {
            return Err("Cannot set child of object 0".to_string());
        }
        let obj_addr = self.get_object_addr(obj_num)?;
        if self.game.header.version <= 3 {
            if child > 255 {
                return Err(format!("Child object number too large for v3: {child}"));
            }
            self.game.memory[obj_addr + 6] = child as u8;
        } else {
            self.write_word((obj_addr + 10) as u32, child)?;
        }
        Ok(())
    }

    /// Remove object from its current location in the object tree
    pub fn remove_object(&mut self, obj_num: u16) -> Result<(), String> {
        if obj_num == 0 {
            return Ok(()); // Object 0 cannot be removed
        }
        let parent = self.get_parent(obj_num)?;
        if parent == 0 {
            return Ok(()); // Already detached
        }

        // Find previous sibling
        let first_child = self.get_child(parent)?;
        if first_child == obj_num {
            // Object is first child, update parent's child pointer
            let next_sibling = self.get_sibling(obj_num)?;
            self.set_child(parent, next_sibling)?;
        } else {
            // Find previous sibling and update its sibling pointer
            let mut current = first_child;
            while current != 0 {
                let next = self.get_sibling(current)?;
                if next == obj_num {
                    let obj_sibling = self.get_sibling(obj_num)?;
                    self.set_sibling(current, obj_sibling)?;
                    break;
                }
                current = next;
            }
        }

        // Clear object's parent and sibling
        self.set_parent(obj_num, 0)?;
        self.set_sibling(obj_num, 0)?;
        Ok(())
    }

    /// Insert object as first child of destination (Z-Machine insert_obj instruction)
    ///
    /// Standard Z-Machine algorithm:
    /// 1. Remove object from current location
    /// 2. Get destination's current first child
    /// 3. Set object as destination's new first child
    /// 4. Set object's parent to destination
    /// 5. Set object's sibling to old first child (forming linked list)
    pub fn insert_object(&mut self, obj_num: u16, dest_num: u16) -> Result<(), String> {
        if obj_num == 0 {
            log::debug!(" insert_object called with object 0 at PC {:05x}", self.pc);
            log::debug!(" dest_num: {}, stack depth: {}", dest_num, self.stack.len());
            if !self.stack.is_empty() {
                log::debug!(" top of stack: {:?}", self.stack.last());
            }

            // ENHANCED DEBUGGING: Show where this insert_obj(0) came from
            log::error!(
                "🚨 VM_INSERT_OBJ_ZERO: insert_object(obj=0, dest={}) called at PC=0x{:04x}",
                dest_num,
                self.pc
            );
            log::error!(
                "🔍 VM_CONTEXT: Variables: 1={}, 2={}, 3={}",
                self.read_variable(1).unwrap_or(999),
                self.read_variable(2).unwrap_or(999),
                self.read_variable(3).unwrap_or(999)
            );

            // Show raw bytes at PC to understand what instruction this was
            let pc_addr = self.pc as usize;
            if pc_addr + 5 < self.game.memory.len() {
                log::error!(
                    "🔍 VM_BYTES_AT_PC: [{:02x} {:02x} {:02x} {:02x} {:02x}] at PC=0x{:04x}",
                    self.game.memory[pc_addr],
                    self.game.memory[pc_addr + 1],
                    self.game.memory[pc_addr + 2],
                    self.game.memory[pc_addr + 3],
                    self.game.memory[pc_addr + 4],
                    pc_addr
                );
            }

            return Err("Cannot insert object 0".to_string());
        }
        if dest_num == 0 {
            return Err("Cannot insert object into object 0".to_string());
        }

        // First remove object from current location
        self.remove_object(obj_num)?;

        // Get current first child of destination
        let old_child = self.get_child(dest_num)?;

        // CRITICAL FIX: Prevent double insertion bug that causes infinite loops
        //
        // Problem: Objects can be inserted both at compile time and runtime. When runtime
        // code tries to re-insert an object that's already the first child, we would:
        // 1. Get old_child = obj_num (the object itself)
        // 2. Set obj_num.sibling = old_child = obj_num (self-reference!)
        // 3. GetObjectSibling loops infinitely: obj → obj → obj → ...
        //
        // Solution: Skip insertion if object is already first child of destination
        // CRITICAL FIX (Nov 2, 2025): Must still set parent relationship even if skipping insertion
        if old_child == obj_num {
            // Object is already correctly positioned as first child (likely from compile-time placement)
            // but we must ensure the parent pointer is set for visibility checks
            self.set_parent(obj_num, dest_num)?;
            return Ok(());
        }

        // Set object as new first child
        self.set_child(dest_num, obj_num)?;
        self.set_parent(obj_num, dest_num)?;
        self.set_sibling(obj_num, old_child)?;

        Ok(())
    }

    /// Get the short name of an object (version-aware)
    pub fn get_object_name(&self, obj_num: u16) -> Result<String, String> {
        log::debug!(" GET_OBJECT_NAME: Accessing object {}", obj_num);

        if obj_num == 0 {
            log::debug!(" Object 0 requested - returning empty string");
            return Ok(String::new()); // Object 0 has no name
        }

        let max_objects = if self.game.header.version <= 3 {
            255
        } else {
            65535
        };
        if obj_num > max_objects {
            log::debug!(
                "Object validation error at PC 0x{:04x}: invalid object {} exceeds maximum {}",
                self.pc,
                obj_num,
                max_objects
            );
            return Err(format!("Invalid object number: {obj_num}"));
        }

        // Get object table base
        let obj_table_addr = self.game.header.object_table_addr;
        let property_defaults = obj_table_addr;
        let default_props = if self.game.header.version <= 3 {
            31
        } else {
            63
        };
        let obj_tree_base = property_defaults + default_props * 2;

        log::debug!(" Object table layout:");
        log::debug!(" object_table_addr: 0x{:04x}", obj_table_addr);
        log::debug!(" property_defaults: 0x{:04x}", property_defaults);
        log::debug!(" default_props: {}", default_props);
        log::debug!(" obj_tree_base: 0x{:04x}", obj_tree_base);

        // Calculate object entry address (version-dependent size)
        let obj_size = if self.game.header.version <= 3 { 9 } else { 14 };
        let obj_addr = obj_tree_base + ((obj_num - 1) as usize * obj_size);

        log::debug!(" Object {} address calculation:", obj_num);
        log::debug!(" obj_size: {}", obj_size);
        log::debug!(
            " obj_addr: 0x{:04x} = 0x{:04x} + (({} - 1) * {})",
            obj_addr,
            obj_tree_base,
            obj_num,
            obj_size
        );

        // Get property table address (last 2 bytes of object entry for both v3 and v4+)
        let prop_table_offset = if self.game.header.version <= 3 { 7 } else { 12 };
        let prop_table_addr = self.read_word((obj_addr + prop_table_offset) as u32) as usize;

        log::debug!(" Property table lookup:");
        log::debug!(" prop_table_offset: {}", prop_table_offset);
        log::debug!(" reading from: 0x{:04x}", obj_addr + prop_table_offset);
        log::debug!(" prop_table_addr: 0x{:04x}", prop_table_addr);
        log::debug!(" file size: {} bytes", self.game.memory.len());

        // Bounds check BEFORE accessing memory
        if prop_table_addr >= self.game.memory.len() {
            log::debug!(
                " BOUNDS ERROR: prop_table_addr 0x{:04x} >= file size {}",
                prop_table_addr,
                self.game.memory.len()
            );
            return Err(format!(
                "Property table address 0x{:04x} out of bounds (file size: {})",
                prop_table_addr,
                self.game.memory.len()
            ));
        }

        // The first byte is the text-length of the short name
        let text_len = self.game.memory[prop_table_addr] as usize;
        log::debug!(" Object {} text_len: {}", obj_num, text_len);

        if text_len > 0 {
            // Decode the object name (stored as Z-string)
            let name_addr = prop_table_addr + 1;
            let abbrev_addr = self.game.header.abbrev_table;

            match crate::text::decode_string(&self.game.memory, name_addr, abbrev_addr) {
                Ok((name, _)) => Ok(name),
                Err(e) => Err(format!("Failed to decode object name: {e}")),
            }
        } else {
            Ok(String::new())
        }
    }
}

impl fmt::Display for VM {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "VM State:")?;
        writeln!(f, " PC: {:04x}", self.pc)?;
        writeln!(f, " Stack depth: {}", self.stack.len())?;
        writeln!(f, " Call depth: {}", self.call_stack.len())?;
        if !self.stack.is_empty() {
            writeln!(f, " Stack top: {:04x}", self.stack.last().unwrap())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_vm() -> VM {
        // Create a minimal game for testing
        let mut memory = vec![0u8; 0x10000];

        // Set up a minimal header
        memory[0x04] = 0x10; // high memory at 0x1000
        memory[0x05] = 0x00;
        memory[0x06] = 0x50; // initial PC at 0x5000
        memory[0x07] = 0x00;
        memory[0x0c] = 0x01; // global variables at 0x0100
        memory[0x0d] = 0x00;
        memory[0x0e] = 0x02; // static memory at 0x0200
        memory[0x0f] = 0x00;

        let game = Game::from_memory(memory).unwrap();
        VM::new(game)
    }

    #[test]
    fn test_vm_creation() {
        let vm = create_test_vm();
        assert_eq!(vm.pc, 0x5000);
        assert!(vm.stack.is_empty());
        // For V1-5 games, there's an initial main frame on the call stack
        assert_eq!(vm.call_stack.len(), 1);
        assert_eq!(vm.call_stack[0].num_locals, 0);
        assert_eq!(vm.call_stack[0].stack_base, 0);
    }

    #[test]
    fn test_stack_operations() {
        let mut vm = create_test_vm();

        // Test push
        vm.push(0x1234).unwrap();
        assert_eq!(vm.stack.len(), 1);

        // Test peek
        assert_eq!(vm.peek().unwrap(), 0x1234);
        assert_eq!(vm.stack.len(), 1);

        // Test pop
        assert_eq!(vm.pop().unwrap(), 0x1234);
        assert!(vm.stack.is_empty());

        // Test underflow
        assert!(vm.pop().is_err());
    }

    #[test]
    fn test_memory_operations() {
        let mut vm = create_test_vm();

        // Test read
        vm.game.memory[0x100] = 0x12;
        vm.game.memory[0x101] = 0x34;
        assert_eq!(vm.read_byte(0x100), 0x12);
        assert_eq!(vm.read_word(0x100), 0x1234);

        // Test write to dynamic memory
        vm.write_byte(0x100, 0xAB).unwrap();
        assert_eq!(vm.read_byte(0x100), 0xAB);

        vm.write_word(0x100, 0xCDEF).unwrap();
        assert_eq!(vm.read_word(0x100), 0xCDEF);

        // Test write to static memory (should fail)
        assert!(vm.write_byte(0x300, 0xFF).is_err());
    }

    #[test]
    fn test_global_variables() {
        let mut vm = create_test_vm();

        // Set up a global variable value
        vm.game.memory[0x100] = 0x12; // First global at offset 0
        vm.game.memory[0x101] = 0x34;

        // Read global 0x10
        assert_eq!(vm.read_global(0x10).unwrap(), 0x1234);

        // Write global 0x10
        vm.write_global(0x10, 0xABCD).unwrap();
        assert_eq!(vm.read_global(0x10).unwrap(), 0xABCD);

        // Test invalid global
        assert!(vm.read_global(0x0F).is_err());
    }

    #[test]
    fn test_variable_access() {
        let mut vm = create_test_vm();

        // Test stack variable (0x00)
        vm.push(0x1234).unwrap();
        assert_eq!(vm.read_variable(0x00).unwrap(), 0x1234);

        // Writing to variable 0 pushes
        vm.write_variable(0x00, 0x5678).unwrap();
        assert_eq!(vm.stack.len(), 2);
        assert_eq!(vm.peek().unwrap(), 0x5678);

        // Test global variable
        vm.game.memory[0x100] = 0xAB;
        vm.game.memory[0x101] = 0xCD;
        assert_eq!(vm.read_variable(0x10).unwrap(), 0xABCD);
    }
}
