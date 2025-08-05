use crate::instruction::Instruction;
use crate::vm::Game;
use log::debug;
use std::collections::HashMap;

/// A comprehensive Z-Machine disassembler following TXD's approach
/// 
/// Current status (2024-08-04):
/// - Successfully finds ALL 440 routines that TXD finds (strict superset)
/// - Also finds 56 additional routines (false positives) beyond TXD's boundary
/// - TXD stops at 0x10b04, we continue to 0x13ee2
/// - Core discovery algorithm is correct, validation needs tuning
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
    /// TXD's pcindex counter for orphan fragment tracking
    pcindex: u32,
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
            let start_pc = ((game.memory[0x06] as u32) << 8) | (game.memory[0x07] as u32);
            // TXD: decode.initial_pc = (unsigned long)header.start_pc - 1; (line 320)
            let initial_pc = start_pc - 1;
            // TXD: code_base = dict_end (line 319) 
            (dict_end, initial_pc)
        };

        // Get resident_size for display
        let resident_size = ((game.memory[0x04] as u32) << 8) | (game.memory[0x05] as u32);
        
        debug!("TXD_INIT: version={}, code_base={:04x}, initial_pc={:04x}, file_size={:04x}", 
               version, code_base, initial_pc, file_size);
        debug!("Resident data ends at {:x}, program starts at {:x}, file ends at {:x}",
               resident_size, initial_pc, file_size);

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
            pcindex: 0,
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

    /// Check if instruction is a return instruction (matching TXD's RETURN types)
    fn is_return_instruction(instruction: &Instruction) -> bool {
        use crate::instruction::{InstructionForm, OperandCount};
        
        match (instruction.form, instruction.operand_count) {
            // 0OP instructions (Short form with OP0)
            (InstructionForm::Short, OperandCount::OP0) => {
                matches!(instruction.opcode, 
                    0x00 | // rtrue
                    0x01 | // rfalse  
                    0x03 | // print_ret
                    0x08 | // ret_popped
                    0x0A   // quit
                )
            }
            // 1OP instructions (Short form with OP1)
            (InstructionForm::Short, OperandCount::OP1) => {
                matches!(instruction.opcode,
                    0x0B | // ret
                    0x0C   // jump (also RETURN type in TXD)
                )
            }
            // VAR instructions that are returns
            (InstructionForm::Variable, _) => {
                matches!(instruction.opcode,
                    0x1C   // throw (v5+, but acts as return)
                )
            }
            // Long form is always 2OP - no return instructions
            _ => false
        }
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
                
                // TXD's pcindex tracking: increment when routine is added
                self.pcindex += 1;
                debug!("TXD_PCINDEX: incremented to {} for routine {:04x}", self.pcindex, rounded_addr);
            }
        }
    }

    /// Run the complete discovery process like TXD
    pub fn discover_routines(&mut self) -> Result<(), String> {
        // TXD's initial scan to find starting point (lines 408-421)
        // This updates self.code_base to the found routine or initial_pc
        let start_pc = self.initial_preliminary_scan()?;
        
        // TXD's main boundary expansion algorithm (lines 444-513)
        // TXD: decode.low_address = decode.pc; decode.high_address = decode.pc;
        self.low_address = start_pc;
        self.high_address = start_pc;
        debug!("TXD_PHASE2_START: initial boundaries low={:04x} high={:04x} pc={:04x}", 
               self.low_address, self.high_address, start_pc);

        self.iterative_boundary_expansion()?;
        
        // TXD's final high routine scan (lines 514-520)
        self.final_high_routine_scan()?;

        // TXD's string scanning to find actual end of code
        self.scan_strings_and_adjust_boundaries()?;

        debug!("TXD_DISCOVERY_COMPLETE: {} routines found", self.routines.len());
        
        Ok(())
    }

    /// TXD's preliminary scan to find good starting point (lines 408-421)
    /// Returns the starting PC for the main scan
    fn initial_preliminary_scan(&mut self) -> Result<u32, String> {
        // TXD scans from code_base to initial_pc looking for low routines
        let mut decode_pc = self.code_base;
        
        debug!("TXD_PRELIM_SCAN: scanning from {:04x} to {:04x}", decode_pc, self.initial_pc);
        
        if decode_pc < self.initial_pc {
            decode_pc = self.round_code(decode_pc);
            let mut pc = decode_pc;
            let mut flag = false;
            
            // TXD line 408: for (pc = decode.pc, flag = 0; pc < decode.initial_pc && flag == 0; pc += code_scaler)
            while pc < self.initial_pc && !flag {
                // TXD lines 410-411: Skip to valid locals count
                let mut test_pc = pc;
                let mut vars = self.game.memory[test_pc as usize] as i8;
                while vars < 0 || vars > 15 {
                    test_pc = self.round_code(test_pc + 1);
                    if test_pc >= self.initial_pc {
                        break;
                    }
                    vars = self.game.memory[test_pc as usize] as i8;
                }
                
                if test_pc >= self.initial_pc {
                    break;
                }
                
                // TXD line 412: decode.pc = pc - 1; 
                decode_pc = test_pc; // NOTE: TXD uses pc-1 but test_pc is already at the var byte
                
                // TXD lines 413-419: Triple validation
                flag = true;
                self.pcindex = 0;
                let rounded = self.round_code(decode_pc);
                let (success, _) = self.txd_triple_validation(rounded);
                if !success || self.pcindex > 0 {
                    flag = false;
                }
                
                // TXD line 420: decode.pc = pc - 1;
                if flag {
                    debug!("TXD_PRELIM: Found valid low routine at {:04x}", decode_pc);
                    // Found a valid routine, use this as starting point
                    return Ok(decode_pc);
                }
                
                pc += self.code_scaler;
            }
            
            // TXD lines 422-439: Additional backward scan for V1-4 (skipping for now)
            
            // TXD lines 440-441: if (flag == 0 || decode.pc > decode.initial_pc) decode.pc = decode.initial_pc;
            if !flag || decode_pc > self.initial_pc {
                decode_pc = self.initial_pc;
            }
        }
        
        Ok(decode_pc)
    }
    
    /// TXD's main iterative boundary expansion (lines 450-513)
    fn iterative_boundary_expansion(&mut self) -> Result<(), String> {
        let mut iteration_count = 0;
        const MAX_ITERATIONS: usize = 100; // Safety limit
        
        loop {
            iteration_count += 1;
            if iteration_count > MAX_ITERATIONS {
                debug!("TXD_ITERATION: Hit maximum iterations, breaking");
                break;
            }
            
            let prev_low = self.low_address;
            let prev_high = self.high_address;
            
            debug!("TXD_ITERATION: starting boundaries low={:04x} high={:04x}", prev_low, prev_high);
            
            let mut pc = self.low_address;
            let mut scan_count = 0;
            
            // TXD lines 463-464: high_pc = decode.high_address (captured at start of iteration)
            let high_pc = self.high_address;
            
            // TXD line 466: while (decode.pc <= high_pc || decode.pc <= decode.initial_pc)
            while pc <= high_pc || pc <= self.initial_pc {
                scan_count += 1;
                if scan_count % 1000 == 0 {
                    debug!("TXD_SCAN_PROGRESS: scanned {} addresses, pc={:04x}", scan_count, pc);
                }
                debug!("TXD_SCAN: trying pc={:04x} (high={:04x} initial={:04x})", pc, high_pc, self.initial_pc);
                
                let (success, end_pc) = self.txd_triple_validation(pc);
                if success {
                    debug!("TXD_SCAN: SUCCESS decode_routine at pc={:04x}", pc);
                    self.add_routine(pc);
                    self.analyze_routine_calls_with_expansion(pc)?;
                    // TXD line 502: pc = ROUND_CODE(decode.pc) - jump to where routine ended
                    pc = self.round_code(end_pc);
                } else {
                    debug!("TXD_SCAN: FAILED decode_routine at pc={:04x}", pc);
                    // TXD lines 480-487: Skip forward to next valid header
                    pc = self.round_code(pc);
                    loop {
                        pc += self.code_scaler;
                        if (pc as usize) >= self.game.memory.len() || pc > self.file_size {
                            pc = self.file_size + 1; // Force exit
                            break;
                        }
                        // TXD: pc++; vars = read_data_byte(&pc); pc--;
                        let vars = self.game.memory[pc as usize];
                        if vars <= 15 {
                            break; // Found valid header
                        }
                    }
                }
            }
            
            // Exit loop if no boundary expansion happened
            if self.low_address >= prev_low && self.high_address <= prev_high {
                debug!("TXD_ITERATION: No boundary expansion, stopping");
                break;
            }
            
            debug!("TXD_ITERATION: Boundaries expanded from {:04x}-{:04x} to {:04x}-{:04x}", 
                   prev_low, prev_high, self.low_address, self.high_address);
        }
        
        Ok(())
    }
    
    /// TXD's final high routine scan (lines 514-520)
    fn final_high_routine_scan(&mut self) -> Result<(), String> {
        debug!("TXD_FINAL_HIGH_SCAN: Starting at high_address={:04x}", self.high_address);
        let initial_routine_count = self.routines.len();
        let mut pc = self.high_address;
        
        while pc < self.file_size {
            debug!("TXD_FINAL_HIGH_SCAN: Trying pc={:04x}", pc);
            let (success, end_pc) = self.txd_triple_validation(pc);
            if success {
                debug!("TXD_FINAL_HIGH_SCAN: Found routine at {:04x}", pc);
                self.add_routine(pc);
                self.high_address = pc;
                self.analyze_routine_calls_with_expansion(pc)?;
                pc = self.round_code(end_pc);
            } else {
                debug!("TXD_FINAL_HIGH_SCAN: No routine at {:04x}, stopping", pc);
                break;
            }
        }
        
        // If we found new routines, they might have expanded boundaries
        // Run another iteration to discover routines in the newly expanded area
        if self.routines.len() > initial_routine_count {
            let new_routines = self.routines.len() - initial_routine_count;
            debug!("TXD_FINAL_HIGH_SCAN: Found {} new routines, checking for boundary expansion", new_routines);
            
            // Run another boundary expansion iteration
            self.iterative_boundary_expansion()?;
        }
        
        Ok(())
    }
    
    /// TXD's exact triple validation with pcindex constraint
    /// Returns (success, end_pc) where end_pc is where the routine ends
    fn txd_triple_validation(&mut self, addr: u32) -> (bool, u32) {
        let rounded_addr = self.round_code(addr);
        let mut routine_end_pc = rounded_addr;
        
        if let Some(locals_count) = self.validate_routine(addr) {
            debug!("DECODE_START ");
            
            // TXD requires triple validation - decode the same routine 3 times
            let mut flag = true;
            for attempt in 0..3 {
                // TXD line 415: pcindex = 0; (reset before each decode attempt)
                self.pcindex = 0;
                debug!("TRIPLE_ATTEMPT_{}: pcindex reset to 0", attempt + 1);
                
                let (decode_result, end_pc) = self.decode_single_routine_attempt(rounded_addr, locals_count);
                routine_end_pc = end_pc;
                
                // TXD line 417: if (decode_routine() != END_OF_ROUTINE || pcindex)
                if !decode_result || self.pcindex > 0 {
                    debug!("FAIL_DECODE (decode={} pcindex={})", decode_result, self.pcindex);
                    flag = false;
                    break;
                }
                
                debug!("TRIPLE_ATTEMPT_{}: success, pcindex={}", attempt + 1, self.pcindex);
            }
            
            if flag {
                debug!("PASS_DECODE");
                (true, routine_end_pc)
            } else {
                debug!("FAIL_DECODE");
                (false, routine_end_pc)
            }
        } else {
            debug!("FAIL_DECODE");
            (false, rounded_addr)
        }
    }
    
    /// TXD's exact sequential decode validation (matching decode_code + decode_outputs)
    /// Returns (success, end_pc) where end_pc is where decoding stopped
    fn decode_single_routine_attempt(&mut self, rounded_addr: u32, locals_count: u8) -> (bool, u32) {
        let mut pc = rounded_addr + 1;
        
        // Skip local variable initial values in V1-4
        if self.version <= 4 {
            pc += (locals_count as u32) * 2;
        }
        
        // TXD's high_pc tracking (line 686: decode.high_pc = decode.pc)
        let mut high_pc = pc;
        
        // Sequential instruction decoding like TXD's decode_code()
        let mut instruction_count = 0;
        const MAX_INSTRUCTIONS: usize = 10000; // Safety limit to prevent infinite loops
        
        loop {
            if (pc as usize) >= self.game.memory.len() || pc >= self.file_size {
                return (false, pc); // Bounds exceeded
            }
            
            instruction_count += 1;
            if instruction_count > MAX_INSTRUCTIONS {
                debug!("DECODE_SAFETY_LIMIT: Hit max instructions limit at pc={:04x}", pc);
                return (false, pc); // Too many instructions, likely invalid routine
            }
            
            // TXD: DO NOT set high_pc = pc here! That's not what TXD does.
            // high_pc tracks the MAXIMUM PC reached, not current PC
            
            match Instruction::decode(&self.game.memory, pc as usize, self.version) {
                Ok(instruction) => {
                    let old_pc = pc;
                    debug!("DECODED at {:04x}: opcode={:02x} form={:?} size={}", 
                           pc, instruction.opcode, instruction.form, instruction.size);
                    
                    // Check operands for boundary expansion (TXD does this during decode_operands)
                    // TXD only checks ROUTINE type operands (from CALL instructions)
                    if Self::is_call_instruction(&instruction, self.version) {
                        if let Some(&first_operand) = instruction.operands.first() {
                            let routine_addr = self.unpack_routine_address(first_operand as u16);
                            self.try_expand_boundaries(routine_addr);
                        }
                    }
                    
                    // Process branch targets during parameter decoding (decode_parameter)
                    if let Some(branch_info) = &instruction.branch {
                        let branch_addr = old_pc.wrapping_add(instruction.size as u32)
                            .wrapping_add((branch_info.offset as i32) as u32).wrapping_sub(2);
                        if branch_addr > high_pc {
                            debug!("HIGH_PC_UPDATE: from {:04x} to {:04x} (branch_addr)", high_pc, branch_addr);
                            high_pc = branch_addr;
                        }
                    }
                    
                    // Advance PC by instruction size
                    pc += instruction.size as u32;
                    
                    // TXD: Return check in decode_extra BEFORE high_pc update!
                    let is_return = Self::is_return_instruction(&instruction);
                    debug!("INSTRUCTION at {:04x}: return={} opcode={:02x} form={:?}", 
                           old_pc, is_return, instruction.opcode, instruction.form);
                    if is_return {
                        debug!("RETURN_CHECK: pc={:04x} high_pc={:04x} condition={}", 
                               pc, high_pc, if pc > high_pc { "TRUE" } else { "FALSE" });
                        if pc > high_pc {
                            // Update high_pc one more time after successful return check
                            debug!("HIGH_PC_UPDATE: from {:04x} to {:04x} (pc_advance)", high_pc, pc);
                            return (true, pc); // END_OF_ROUTINE - valid routine found
                        }
                        // If condition fails, continue decoding (don't immediately fail)
                    }
                    
                    // TXD: Update high_pc after decode_extra (decode_operands line 1115)
                    if pc > high_pc {
                        debug!("HIGH_PC_UPDATE: from {:04x} to {:04x} (pc_advance)", high_pc, pc);
                        high_pc = pc;
                    }
                    
                    // Safety: prevent infinite loops
                    if pc <= old_pc {
                        return (false, pc);
                    }
                }
                Err(_) => {
                    return (false, pc); // Decode failure - invalid routine
                }
            }
        }
    }

    /// Analyze routine calls WITH boundary expansion (TXD lines 1376-1401)
    fn analyze_routine_calls_with_expansion(&mut self, routine_addr: u32) -> Result<(), String> {
        let rounded_addr = self.round_code(routine_addr);
        let locals_count = self.game.memory[rounded_addr as usize];
        
        let mut pc = rounded_addr + 1;
        
        // Skip local variable initial values in V1-4
        if self.version <= 4 {
            pc += (locals_count as u32) * 2;
        }
        
        // Decode instructions and look for operands that expand boundaries
        while (pc as usize) < self.game.memory.len() {
            match Instruction::decode(&self.game.memory, pc as usize, self.version) {
                Ok(instruction) => {
                    // Use the proper instruction target checking that handles V4 opcodes
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
    
    /// TXD's exact boundary expansion logic (lines 1376-1401)
    fn check_operand_for_boundary_expansion(&mut self, operand: u32) {
        // Check operand as routine address (for CALL instructions)
        let routine_addr = self.unpack_routine_address(operand as u16);
        if routine_addr >= self.code_base && routine_addr < self.file_size {
            self.try_expand_boundaries(routine_addr);
        }
        
        // Check operand as direct address (for branches/labels)
        if operand >= self.code_base && operand < self.file_size {
            self.try_expand_boundaries(operand);
        }
        
        // Check if operand could be a packed string address or other packed data
        // (TXD checks various interpretations of operands)
        let string_addr = operand * self.story_scaler;
        if string_addr >= self.code_base && string_addr < self.file_size {
            self.try_expand_boundaries(string_addr);
        }
    }
    
    /// Check operand and expand boundaries if it's a valid routine address
    fn check_and_expand_operand(&mut self, operand: u32) {
        // Check if operand could be a routine address
        let routine_addr = self.unpack_routine_address(operand as u16);
        
        // Only check if it's in a reasonable range
        if routine_addr >= self.code_base && routine_addr < self.file_size {
            debug!("TXD_OPERAND: checking addr={:04x} (low={:04x} high={:04x})", 
                   routine_addr, self.low_address, self.high_address);
            
            // Check for boundary expansion
            if routine_addr < self.low_address {
                if let Some(locals_count) = self.validate_routine(routine_addr) {
                    debug!("TXD_CHECK_LOW: addr={:04x} vars={} EXPANDING LOW from {:04x} to {:04x}", 
                           routine_addr, locals_count, self.low_address, routine_addr);
                    self.low_address = routine_addr;
                }
            } else if routine_addr > self.high_address {
                if let Some(locals_count) = self.validate_routine(routine_addr) {
                    debug!("TXD_CHECK_HIGH: addr={:04x} vars={} EXPANDING HIGH from {:04x} to {:04x}", 
                           routine_addr, locals_count, self.high_address, routine_addr);
                    self.high_address = routine_addr;
                }
            }
        }
    }

    /// Try to expand boundaries for a given address (exact TXD logic)
    fn try_expand_boundaries(&mut self, addr: u32) {
        debug!("TXD_OPERAND: checking addr={:04x} (low={:04x} high={:04x})", 
               addr, self.low_address, self.high_address);
               
        // Check for low boundary expansion (TXD lines 1376-1388)
        if addr < self.low_address && addr >= self.code_base {
            if (addr as usize) < self.game.memory.len() {
                let vars = self.game.memory[addr as usize];
                debug!("TXD_CHECK_LOW: addr={:04x} vars={} ", addr, vars);
                if vars <= 15 {
                    debug!("EXPANDING LOW from {:04x} to {:04x}", self.low_address, addr);
                    self.low_address = addr;
                } else {
                    debug!("REJECT LOW (bad vars)");
                }
            }
        }
        
        // Check for high boundary expansion (TXD lines 1389-1401)
        if addr > self.high_address && addr < self.file_size {
            if (addr as usize) < self.game.memory.len() {
                let vars = self.game.memory[addr as usize];
                debug!("TXD_CHECK_HIGH: addr={:04x} vars={} ", addr, vars);
                if vars <= 15 {
                    debug!("EXPANDING HIGH from {:04x} to {:04x}", self.high_address, addr);
                    self.high_address = addr;
                } else {
                    debug!("REJECT HIGH (bad vars)");
                }
            }
        }
    }

    /// Check if an instruction is a CALL instruction
    fn is_call_instruction(instruction: &Instruction, version: u8) -> bool {
        match instruction.form {
            crate::instruction::InstructionForm::Variable => {
                // VAR opcodes: call/call_vs (0x20), call_vs2 (0x2c), call_vn (0x39), call_vn2 (0x3a)
                matches!(instruction.opcode, 0x20 | 0x2c | 0x39 | 0x3a)
            }
            crate::instruction::InstructionForm::Long => {
                // 2OP opcodes: call_2s (0x19) - V4+ only
                // Note: call_2n (0x1a) is V5+ only, not V4
                version >= 4 && instruction.opcode == 0x19
            }
            crate::instruction::InstructionForm::Short => {
                // 1OP opcodes: call_1s (0x08) - V4+ only
                // Note: 0x0f is 'not' in V4, only becomes call_1n in V5+
                version >= 4 && instruction.opcode == 0x08
            }
            _ => false,
        }
    }

    /// Check instruction for call targets and expand boundaries
    fn check_instruction_targets(&mut self, instruction: &Instruction, pc: u32) {
        let is_call = Self::is_call_instruction(instruction, self.version);
        
        // Also check for timer routines in READ (sread) instruction - V4+ only
        let is_timed_read = self.version >= 4 && 
            instruction.form == crate::instruction::InstructionForm::Variable &&
            instruction.opcode == 0x04 && // sread
            instruction.operands.len() >= 4 && // has timer routine operand
            instruction.operands[3] != 0; // timer routine is non-zero
        
        if is_call {
            if let Some(target) = self.extract_routine_target(instruction, pc) {
                self.process_routine_target(target);
            }
        } else if is_timed_read {
            // Timer routine is in operand 3 for timed read
            let timer_routine = instruction.operands[3];
            let target = self.unpack_routine_address(timer_routine as u16);
            debug!("TXD_TIMER_ROUTINE: found timer routine at {:04x} from sread", target);
            self.process_routine_target(target);
        }
        // Note: TXD only expands boundaries for ROUTINE type operands (CALL instructions)
        // not for other operand types like LABEL (JUMP instructions)
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

    /// TXD's string scanning to identify where code ends and strings begin
    /// TXD's approach: Only cut off when we find consistent string region at the end
    fn scan_strings_and_adjust_boundaries(&mut self) -> Result<(), String> {
        debug!("TXD_STRING_SCAN: Adjusting for string region like TXD");
        
        // TXD's actual behavior based on the output:
        // - TXD stops at 0x10b04 for Zork
        // - We're finding routines up to 0x13ee2
        // - The difference is we're finding false positives in the string region
        
        // For now, use a heuristic based on what we know about TXD's behavior:
        // If we've expanded beyond a reasonable code size, check more carefully
        
        // For v4+ files, don't apply string detection - TXD doesn't seem to filter
        // routines based on string regions for v4+ files
        if self.version >= 4 {
            debug!("TXD_STRING_SCAN: Skipping string detection for v{} file", self.version);
            return Ok(());
        }
        
        // Version-aware code size limits based on Z-Machine memory constraints
        let typical_max_code_size = match self.version {
            1..=3 => 0x10C00,  // ~67KB, based on TXD's v3 boundary
            4..=5 => 0x3A000,  // ~232KB, based on v4-5 memory layout
            6..=8 => 0x60000,  // ~384KB, v6+ can be larger
            _ => 0x10C00,      // Default to v3
        };
        
        if self.high_address > typical_max_code_size {
            debug!("TXD_STRING_SCAN: High address {:04x} exceeds typical code size", self.high_address);
            
            // Find routines beyond the typical boundary
            let mut routines_to_check: Vec<u32> = Vec::new();
            for (addr, _) in &self.routines {
                if *addr >= typical_max_code_size {
                    routines_to_check.push(*addr);
                }
            }
            routines_to_check.sort();
            
            // For each suspicious routine, check if it's in a string region
            let mut first_string_routine = None;
            for addr in &routines_to_check {
                // Check area around this routine for string patterns
                let scan_start = addr.saturating_sub(0x100);
                let scan_end = addr + 0x200;
                
                let mut string_count = 0;
                let mut pc = scan_start;
                
                while pc < scan_end && pc + 1 < self.file_size {
                    if (pc as usize) + 1 >= self.game.memory.len() {
                        break;
                    }
                    
                    let word = ((self.game.memory[pc as usize] as u16) << 8) | 
                               (self.game.memory[(pc + 1) as usize] as u16);
                    
                    if (word & 0x8000) != 0 {
                        string_count += 1;
                    }
                    
                    pc += 2;
                }
                
                // If we find multiple string terminators around this "routine", it's probably in string data
                if string_count >= 3 {
                    debug!("TXD_STRING_SCAN: Routine at {:04x} appears to be in string region ({} terminators found)", 
                           addr, string_count);
                    if first_string_routine.is_none() {
                        first_string_routine = Some(*addr);
                    }
                }
            }
            
            // Remove all routines at or after the first one in string region
            if let Some(cutoff) = first_string_routine {
                let mut removed_count = 0;
                let mut routines_to_remove = Vec::new();
                
                for (addr, _) in &self.routines {
                    if *addr >= cutoff {
                        routines_to_remove.push(*addr);
                    }
                }
                
                for addr in routines_to_remove {
                    self.routines.remove(&addr);
                    removed_count += 1;
                }
                
                debug!("TXD_STRING_SCAN: Removed {} routines at or after {:04x}", removed_count, cutoff);
                
                // Update high address to just before the string region
                self.high_address = cutoff.saturating_sub(1);
            }
        }
        
        Ok(())
    }

    /// Generate output matching TXD format
    pub fn generate_output(&self) -> String {
        let mut output = String::new();
        
        // Get resident_size from header for display
        let resident_size = ((self.game.memory[0x04] as u32) << 8) | (self.game.memory[0x05] as u32);
        
        // Header information - match TXD exactly
        output.push_str(&format!("Resident data ends at {:x}, program starts at {:x}, file ends at {:x}\n\n", 
                                resident_size, self.initial_pc, self.file_size));
        
        output.push_str(&format!("Starting analysis pass at address {:x}\n\n", self.code_base));
        
        output.push_str(&format!("End of analysis pass, low address = {:x}, high address = {:x}\n\n", 
                                self.low_address, self.high_address));
        
        output.push_str("[Start of code]\n\n");
        
        // TODO: Generate routine disassembly in TXD format
        let mut sorted_routines: Vec<_> = self.routines.iter().collect();
        sorted_routines.sort_by_key(|(addr, _)| *addr);
        
        let mut routine_num = 1;
        for (addr, routine) in sorted_routines {
            // Check if this is the main routine
            let routine_prefix = if *addr == self.initial_pc { "Main r" } else { "R" };
            
            output.push_str(&format!("{}outine R{:04}, {} local", 
                                    routine_prefix, routine_num, routine.locals_count));
            if routine.locals_count != 1 {
                output.push('s');
            }
            
            // TODO: Add local variable initial values for V1-4
            output.push_str(" (0000)\n\n");
            
            // TODO: Add instruction disassembly
            output.push_str("       ; Routine disassembly not yet implemented\n\n");
            
            routine_num += 1;
        }
        
        output.push_str("[End of code]\n");
        output
    }
}