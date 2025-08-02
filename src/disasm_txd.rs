use crate::instruction::Instruction;
use crate::vm::Game;
use log::debug;
use std::collections::HashMap;

/// A comprehensive Z-Machine disassembler following TXD's approach
pub struct TxdDisassembler<'a> {
    game: &'a Game,
    version: u8,
    /// Low address boundary for code
    low_address: u32,
    /// High address boundary for code  
    high_address: u32,
    /// Initial program counter
    initial_pc: u32,
    /// Base address where code starts scanning
    code_base: u32,
    /// File size
    file_size: u32,
    /// Code scaling factor (2 for v3, 4 for v4-5, etc.)
    code_scaler: u32,
    /// Story scaling factor (typically 1)
    story_scaler: u32,
    /// Discovered routines
    routines: HashMap<u32, RoutineInfo>,
    /// Routine addresses to process
    routine_queue: Vec<u32>,
    /// First pass flag
    first_pass: bool,
}

#[derive(Debug, Clone)]
struct RoutineInfo {
    address: u32,
    locals_count: u8,
    validated: bool,
    instructions: Vec<InstructionInfo>,
}

#[derive(Debug, Clone)]
struct InstructionInfo {
    address: u32,
    instruction: Instruction,
    targets: Vec<u32>, // Call targets or branch targets
}

impl<'a> TxdDisassembler<'a> {
    /// Create a new TXD-style disassembler
    pub fn new(game: &'a Game) -> Self {
        let version = game.header.version;
        let file_size = game.memory.len() as u32;
        
        // Calculate scaling factors based on version
        let (code_scaler, story_scaler) = match version {
            1..=3 => (2, 1),
            4..=5 => (4, 1), 
            6..=7 => (4, 1), // More complex for v6/7 but start simple
            8 => (8, 1),
            _ => (2, 1), // Default to v3
        };

        // Determine code base and initial PC
        let (code_base, initial_pc) = if version == 6 || version == 7 {
            // V6/7 have routines offset
            let routines_offset = ((game.memory[0x28] as u32) << 8) | (game.memory[0x29] as u32);
            let code_base = routines_offset * story_scaler;
            let start_pc = ((game.memory[0x06] as u32) << 8) | (game.memory[0x07] as u32);
            let initial_pc = code_base + start_pc * code_scaler;
            (code_base, initial_pc)
        } else {
            // V1-5: code starts after dictionary, initial PC from header
            let dict_end = Self::calculate_dict_end(game);
            let initial_pc = ((game.memory[0x06] as u32) << 8) | (game.memory[0x07] as u32);
            // For Zork, TXD uses dict_end + 1 as the start boundary
            (dict_end + 1, initial_pc)
        };

        debug!("TXD_INIT: version={}, code_base={:04x}, initial_pc={:04x}, file_size={:04x}", 
               version, code_base, initial_pc, file_size);

        Self {
            game,
            version,
            low_address: code_base,
            high_address: code_base,
            initial_pc,
            code_base,
            file_size,
            code_scaler,
            story_scaler,
            routines: HashMap::new(),
            routine_queue: Vec::new(),
            first_pass: true,
        }
    }

    /// Calculate where dictionary ends (for V1-5)
    fn calculate_dict_end(game: &Game) -> u32 {
        let dict_addr = ((game.memory[0x08] as u32) << 8) | (game.memory[0x09] as u32);
        
        // Skip separator characters count
        let mut addr = dict_addr;
        let sep_count = game.memory[addr as usize];
        addr += 1 + sep_count as u32;
        
        // Get word length and count
        let word_len = game.memory[addr as usize] as u32;
        addr += 1;
        let word_count = ((game.memory[addr as usize] as u32) << 8) | (game.memory[(addr + 1) as usize] as u32);
        addr += 2;
        
        // End of dictionary
        addr + (word_count * word_len)
    }

    /// Round address to code boundary
    fn round_code(&self, addr: u32) -> u32 {
        (addr + (self.code_scaler - 1)) & !(self.code_scaler - 1)
    }

    /// Validate a routine at the given address
    fn validate_routine(&self, addr: u32) -> Option<u8> {
        if addr as usize >= self.game.memory.len() {
            return None;
        }
        
        let rounded_addr = self.round_code(addr);
        if rounded_addr as usize >= self.game.memory.len() {
            return None;
        }
        
        let locals_count = self.game.memory[rounded_addr as usize];
        debug!("TXD_VALIDATE: addr={:04x} vars={} ", addr, locals_count);
        
        if locals_count <= 15 {
            debug!("PASS_VARS ");
            Some(locals_count)
        } else {
            debug!("FAIL_VARS");
            None
        }
    }

    /// Decode a routine and check if it ends properly
    fn decode_routine_validation(&self, addr: u32) -> bool {
        let rounded_addr = self.round_code(addr);
        
        if let Some(locals_count) = self.validate_routine(addr) {
            let mut pc = rounded_addr + 1;
            
            // Skip local variable initial values in V1-4
            if self.version <= 4 {
                pc += (locals_count as u32) * 2;
            }
            
            debug!("DECODE_START ");
            
            // Try to decode instructions until we find a return
            let mut instruction_count = 0;
            let max_instructions = 1000; // Safety limit
            let mut found_return = false;
            
            while (pc as usize) < self.game.memory.len() && instruction_count < max_instructions {
                match Instruction::decode(&self.game.memory, pc as usize, self.version) {
                    Ok(instruction) => {
                        let old_pc = pc;
                        pc += instruction.size as u32;
                        instruction_count += 1;
                        
                        // Check for return instructions
                        if Self::is_return_instruction(&instruction) {
                            found_return = true;
                            break;
                        }
                        
                        // Additional validation - check for suspicious patterns
                        if pc <= old_pc {
                            // PC didn't advance properly
                            debug!("FAIL_DECODE");
                            return false;
                        }
                        
                        // Don't allow routines to get too large (TXD constraint)
                        if pc > rounded_addr + 2000 {
                            debug!("FAIL_DECODE");
                            return false;
                        }
                    }
                    Err(_) => {
                        debug!("FAIL_DECODE");
                        return false;
                    }
                }
            }
            
            if found_return {
                debug!("PASS_DECODE");
                true
            } else {
                debug!("FAIL_DECODE");
                false
            }
        } else {
            false
        }
    }

    /// Check if instruction is a return instruction
    fn is_return_instruction(instruction: &Instruction) -> bool {
        matches!(instruction.opcode, 
            0x00 | // rtrue (0OP)
            0x01 | // rfalse (0OP)  
            0x08 | // ret_popped (0OP)
            0x0B | // ret (1OP)
            0x0A   // quit (0OP) - also ends execution
        )
    }

    /// Add a routine to the discovery list
    fn add_routine(&mut self, addr: u32) {
        debug!("TXD_ADD_ROUTINE: {:04x}", addr);
        
        let rounded_addr = self.round_code(addr);
        if !self.routines.contains_key(&rounded_addr) {
            if let Some(locals_count) = self.validate_routine(addr) {
                let routine_info = RoutineInfo {
                    address: rounded_addr,
                    locals_count,
                    validated: false,
                    instructions: Vec::new(),
                };
                self.routines.insert(rounded_addr, routine_info);
                self.routine_queue.push(rounded_addr);
            }
        }
    }

    /// Run the complete discovery process like TXD
    pub fn discover_routines(&mut self) -> Result<(), String> {
        debug!("TXD_PHASE2_START: initial boundaries low={:04x} high={:04x} pc={:04x}", 
               self.low_address, self.high_address, self.code_base);

        // Initial setup - start from the calculated boundaries
        self.low_address = self.initial_pc.min(self.code_base);
        self.high_address = self.initial_pc.max(self.code_base);
        
        // Iterative boundary expansion like TXD
        loop {
            let prev_low = self.low_address;
            let prev_high = self.high_address;
            
            debug!("TXD_ITERATION: starting boundaries low={:04x} high={:04x}", 
                   prev_low, prev_high);

            // Clear routines before each iteration to rebuild them
            self.routines.clear();
            self.routine_queue.clear();

            self.scan_routines_in_range()?;
            
            // Check if boundaries expanded
            if self.low_address >= prev_low && self.high_address <= prev_high {
                break; // No more expansion
            }
            
            // Safety check to prevent infinite expansion
            if self.high_address > self.file_size || 
               self.low_address < self.code_base ||
               (self.high_address - self.low_address) > 50000 {
                debug!("TXD_SAFETY: stopping expansion at low={:04x} high={:04x}", 
                       self.low_address, self.high_address);
                break;
            }
        }

        debug!("TXD_DISCOVERY_COMPLETE: final boundaries low={:04x} high={:04x}, {} routines found", 
               self.low_address, self.high_address, self.routines.len());
        
        Ok(())
    }

    /// Scan for routines in the current boundary range
    fn scan_routines_in_range(&mut self) -> Result<(), String> {
        let mut pc = self.low_address;
        let high_limit = self.high_address.max(self.initial_pc);
        
        while pc <= high_limit || pc <= self.initial_pc {
            debug!("TXD_SCAN: trying pc={:04x} (high={:04x} initial={:04x})", 
                   pc, high_limit, self.initial_pc);
            
            if self.decode_routine_validation(pc) {
                debug!("TXD_SCAN: SUCCESS decode_routine at pc={:04x}", pc);
                self.add_routine(pc);
                
                // Analyze this routine for call targets
                self.analyze_routine_calls(pc)?;
                
                // Move to next potential routine
                pc = self.round_code(pc + 1);
            } else {
                debug!("TXD_SCAN: FAILED decode_routine at pc={:04x}", pc);
                // Skip to next code boundary  
                pc = self.round_code(pc + self.code_scaler);
            }
            
            // Safety check to prevent infinite loops
            if pc > self.file_size {
                break;
            }
        }
        
        Ok(())
    }

    /// Analyze a routine for CALL instructions to discover new routines
    fn analyze_routine_calls(&mut self, routine_addr: u32) -> Result<(), String> {
        let rounded_addr = self.round_code(routine_addr);
        let locals_count = self.game.memory[rounded_addr as usize];
        
        let mut pc = rounded_addr + 1;
        
        // Skip local variable initial values in V1-4
        if self.version <= 4 {
            pc += (locals_count as u32) * 2;
        }
        
        // Decode instructions and look for calls
        while (pc as usize) < self.game.memory.len() {
            match Instruction::decode(&self.game.memory, pc as usize, self.version) {
                Ok(instruction) => {
                    // Check for call instructions and extract targets
                    self.check_instruction_targets(&instruction, pc);
                    
                    pc += instruction.size as u32;
                    
                    // Stop at return instructions
                    if Self::is_return_instruction(&instruction) {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        
        Ok(())
    }

    /// Check instruction for call targets and expand boundaries
    fn check_instruction_targets(&mut self, instruction: &Instruction, pc: u32) {
        // Look for CALL instructions and routine operands based on form and opcode
        let is_call = match instruction.form {
            crate::instruction::InstructionForm::Variable => {
                // VAR opcodes: call (0x20), call_2s (0x19), call_vs2 (0x2c), call_vn (0x39), call_vn2 (0x3a)
                matches!(instruction.opcode, 0x20 | 0x19 | 0x2c | 0x39 | 0x3a)
            }
            crate::instruction::InstructionForm::Short => {
                // 1OP opcodes: call_1s (0x08), call_1n (0x0f)
                matches!(instruction.opcode, 0x08 | 0x0f)
            }
            _ => false,
        };
        
        if is_call {
            if let Some(target) = self.extract_routine_target(instruction, pc) {
                self.process_routine_target(target);
            }
        } else {
            // For non-call instructions, check operands that might be routine addresses
            for operand in &instruction.operands {
                if let Some(target) = self.check_operand_for_routine_address(*operand as u32) {
                    self.process_routine_target(target);
                }
            }
        }
    }

    /// Extract routine target from CALL instruction
    fn extract_routine_target(&self, instruction: &Instruction, _pc: u32) -> Option<u32> {
        if !instruction.operands.is_empty() {
            let packed_addr = instruction.operands[0];
            if packed_addr != 0 {
                Some(self.unpack_routine_address(packed_addr as u16))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Check if operand could be a routine address
    fn check_operand_for_routine_address(&self, operand: u32) -> Option<u32> {
        // This is a heuristic - in a real implementation we'd be more sophisticated
        let addr = self.unpack_routine_address(operand as u16);
        if addr >= self.code_base && addr < self.file_size {
            if self.validate_routine(addr).is_some() {
                return Some(addr);
            }
        }
        None
    }

    /// Process a discovered routine target
    fn process_routine_target(&mut self, target: u32) {
        debug!("TXD_OPERAND: checking addr={:04x} (low={:04x} high={:04x})", 
               target, self.low_address, self.high_address);
        
        if target < self.low_address && target >= self.code_base {
            if let Some(locals_count) = self.validate_routine(target) {
                debug!("TXD_CHECK_LOW: addr={:04x} vars={} EXPANDING LOW from {:04x} to {:04x}", 
                       target, locals_count, self.low_address, target);
                self.low_address = target;
            } else {
                debug!("TXD_CHECK_LOW: addr={:04x} REJECT LOW (bad vars)", target);
            }
        }
        
        if target > self.high_address && target < self.file_size {
            if let Some(locals_count) = self.validate_routine(target) {
                debug!("TXD_CHECK_HIGH: addr={:04x} vars={} EXPANDING HIGH from {:04x} to {:04x}", 
                       target, locals_count, self.high_address, target);
                self.high_address = target;
            } else {
                debug!("TXD_CHECK_HIGH: addr={:04x} REJECT HIGH (bad vars)", target);
            }
        }
    }

    /// Unpack a routine address based on version
    fn unpack_routine_address(&self, packed: u16) -> u32 {
        match self.version {
            1..=3 => (packed as u32) * 2,
            4..=5 => (packed as u32) * 4,
            6..=7 => {
                // V6/7 more complex - add routines offset  
                let routines_offset = ((self.game.memory[0x28] as u32) << 8) | (self.game.memory[0x29] as u32);
                (packed as u32) * 4 + routines_offset * self.story_scaler
            }
            8 => (packed as u32) * 8,
            _ => (packed as u32) * 2,
        }
    }

    /// Generate output matching TXD format
    pub fn generate_output(&self) -> String {
        let mut output = String::new();
        
        // Header information
        output.push_str(&format!("Resident data ends at {:x}, program starts at {:x}, file ends at {:x}\n\n", 
                                self.code_base, self.initial_pc, self.file_size));
        
        output.push_str(&format!("Starting analysis pass at address {:x}\n\n", self.code_base));
        
        output.push_str(&format!("End of analysis pass, low address = {:x}, high address = {:x}\n\n", 
                                self.low_address, self.high_address));
        
        output.push_str("[Start of code]\n\n");
        
        // TODO: Generate routine disassembly in TXD format
        for (addr, routine) in &self.routines {
            output.push_str(&format!("Routine R{:04}, {} local", 
                                    addr, routine.locals_count));
            if routine.locals_count != 1 {
                output.push('s');
            }
            output.push_str(" (0000)\n\n");
            
            // TODO: Add instruction disassembly
            output.push_str("       ; Routine disassembly not yet implemented\n\n");
        }
        
        output.push_str("[End of code]\n");
        output
    }
}