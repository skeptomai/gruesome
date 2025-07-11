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
}