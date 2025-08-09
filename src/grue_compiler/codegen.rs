// Z-Machine Code Generator
//
// Transforms IR into executable Z-Machine bytecode following the Z-Machine Standard v1.1
// Supports both v3 and v5 target formats with proper memory layout and instruction encoding.

use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::*;
use crate::grue_compiler::ZMachineVersion;
use std::collections::HashMap;

/// Z-Machine memory layout constants
const HEADER_SIZE: usize = 64; // Fixed 64-byte header
const DEFAULT_HIGH_MEMORY: u16 = 0x8000; // Start of high memory
const DEFAULT_PC_START: u16 = 0x1000; // Initial program counter

/// Code generation context
pub struct ZMachineCodeGen {
    version: ZMachineVersion,

    // Memory layout
    story_data: Vec<u8>,
    current_address: usize,

    // Code generation state
    label_addresses: HashMap<IrId, usize>, // IR label ID -> byte address
    string_addresses: HashMap<IrId, usize>, // IR string ID -> byte address
    function_addresses: HashMap<IrId, usize>, // IR function ID -> byte address

    // Tables for Z-Machine structures
    object_table_addr: usize,
    property_table_addr: usize,
    dictionary_addr: usize,
    global_vars_addr: usize,

    // String encoding
    strings: Vec<(IrId, String)>, // Collected strings for encoding
    encoded_strings: HashMap<IrId, Vec<u8>>, // IR string ID -> encoded bytes
}

impl ZMachineCodeGen {
    pub fn new(version: ZMachineVersion) -> Self {
        ZMachineCodeGen {
            version,
            story_data: vec![0; HEADER_SIZE],
            current_address: HEADER_SIZE,
            label_addresses: HashMap::new(),
            string_addresses: HashMap::new(),
            function_addresses: HashMap::new(),
            object_table_addr: 0,
            property_table_addr: 0,
            dictionary_addr: 0,
            global_vars_addr: 0,
            strings: Vec::new(),
            encoded_strings: HashMap::new(),
        }
    }

    pub fn generate(&mut self, ir: IrProgram) -> Result<Vec<u8>, CompilerError> {
        // Phase 1: Collect and encode all strings
        self.collect_strings(&ir)?;
        self.encode_all_strings()?;

        // Phase 2: Reserve space for Z-Machine structures
        self.layout_memory_structures(&ir)?;

        // Phase 3: Generate object and property tables
        self.generate_object_tables(&ir)?;

        // Phase 4: Generate dictionary
        self.generate_dictionary(&ir)?;

        // Phase 5: Generate global variables
        self.generate_global_variables(&ir)?;

        // Phase 6: Generate code for all functions
        self.generate_functions(&ir)?;

        // Phase 7: Generate init block (if present)
        if let Some(init_block) = &ir.init_block {
            self.generate_init_block(init_block)?;
        }

        // Phase 8: Write Z-Machine header
        self.write_header()?;

        // Phase 9: Resolve all addresses and patch jumps
        self.resolve_addresses()?;

        Ok(self.story_data.clone())
    }

    /// Collect all strings from the IR program for later encoding
    fn collect_strings(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        // Collect from string table
        for (string, &id) in &ir.string_table {
            self.strings.push((id, string.clone()));
        }

        // TODO: Collect strings from other IR elements (rooms, objects, etc.)

        Ok(())
    }

    /// Encode all collected strings using Z-Machine text encoding
    fn encode_all_strings(&mut self) -> Result<(), CompilerError> {
        for (id, string) in &self.strings {
            let encoded = self.encode_string(string)?;
            self.encoded_strings.insert(*id, encoded);
        }
        Ok(())
    }

    /// Encode a single string using Z-Machine ZSCII encoding
    fn encode_string(&self, s: &str) -> Result<Vec<u8>, CompilerError> {
        // Simplified Z-Machine string encoding (ZSCII)
        // TODO: Implement proper dictionary lookup and abbreviations

        let mut encoded = Vec::new();
        let chars: Vec<char> = s.chars().collect();

        // Process characters in groups of 3 (Z-Machine uses 3 5-bit chars per word)
        for chunk in chars.chunks(3) {
            let mut word = 0u16;

            // Pack up to 3 characters into a 16-bit word
            for (i, &ch) in chunk.iter().enumerate() {
                let zchar = match ch {
                    'a'..='z' => (ch as u8 - b'a' + 6) as u16,
                    'A'..='Z' => (ch as u8 - b'A' + 6) as u16, // Same as lowercase in alphabet 0
                    ' ' => 0,
                    '?' => 31, // Common punctuation
                    '.' => 31,
                    ',' => 31,
                    _ => 6, // Default to 'a' for unsupported chars
                };

                word |= (zchar & 0x1F) << (10 - i * 5);
            }

            // If this is the last word in the string, set the high bit
            if chunk.len() < 3 || chars.len() <= encoded.len() / 2 * 3 + 3 {
                word |= 0x8000;
            }

            // Store as big-endian
            encoded.push((word >> 8) as u8);
            encoded.push(word as u8);
        }

        // Ensure we have at least one word (empty strings become a single word with high bit set)
        if encoded.is_empty() {
            encoded.push(0x80);
            encoded.push(0x00);
        } else {
            // Make sure the last word has the high bit set to terminate the string
            let last_idx = encoded.len() - 2;
            encoded[last_idx] |= 0x80;
        }

        Ok(encoded)
    }

    /// Plan the memory layout for all Z-Machine structures
    fn layout_memory_structures(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        // Start after header
        let mut addr = HEADER_SIZE;

        // Reserve space for global variables (480 bytes for 240 globals)
        self.global_vars_addr = addr;
        addr += 480;

        // Reserve space for object table
        self.object_table_addr = addr;
        let object_count = ir.objects.len() + ir.rooms.len(); // Rooms become objects
        addr += match self.version {
            ZMachineVersion::V3 => object_count * 9 + 62, // v3: 9 bytes per object + 62 byte tree table
            ZMachineVersion::V5 => object_count * 14 + 126, // v5: 14 bytes per object + 126 byte tree table
        };

        // Reserve space for property tables
        self.property_table_addr = addr;
        addr += ir.objects.len() * 50; // Rough estimate: 50 bytes per object for properties

        // Reserve space for dictionary
        self.dictionary_addr = addr;
        addr += 1000; // Rough estimate for dictionary

        // Reserve space for encoded strings
        for encoded in self.encoded_strings.values() {
            self.string_addresses.insert(0, addr); // TODO: Use actual string ID
            addr += encoded.len();
        }

        // Code starts after all data structures
        self.current_address = addr;

        Ok(())
    }

    /// Generate object and property tables
    fn generate_object_tables(&mut self, _ir: &IrProgram) -> Result<(), CompilerError> {
        // TODO: Implement object table generation
        // For now, create minimal object table

        let obj_table_start = self.object_table_addr;
        self.ensure_capacity(obj_table_start + 100);

        // Write minimal object tree (31 or 63 default property values)
        let default_props = match self.version {
            ZMachineVersion::V3 => 31,
            ZMachineVersion::V5 => 63,
        };

        for i in 0..default_props {
            let addr = obj_table_start + i * 2;
            self.ensure_capacity(addr + 2);
            self.story_data[addr] = 0; // Default property value (high byte)
            self.story_data[addr + 1] = 0; // Default property value (low byte)
        }

        Ok(())
    }

    /// Generate dictionary
    fn generate_dictionary(&mut self, _ir: &IrProgram) -> Result<(), CompilerError> {
        // TODO: Implement dictionary generation
        // For now, create minimal dictionary

        let dict_start = self.dictionary_addr;
        self.ensure_capacity(dict_start + 10);

        // Minimal dictionary header
        self.story_data[dict_start] = 4; // Entry length (4 bytes for v3/v5)
        self.story_data[dict_start + 1] = 0; // Number of entries (high byte)
        self.story_data[dict_start + 2] = 0; // Number of entries (low byte)

        Ok(())
    }

    /// Generate global variables table
    fn generate_global_variables(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        let globals_start = self.global_vars_addr;
        self.ensure_capacity(globals_start + 480); // Space for 240 globals

        // Initialize all globals to 0
        for i in 0..240 {
            let addr = globals_start + i * 2;
            self.story_data[addr] = 0; // High byte
            self.story_data[addr + 1] = 0; // Low byte
        }

        // Set specific globals from IR
        for _global in &ir.globals {
            // TODO: Map IR globals to Z-Machine global variables
            // For now, just ensure the space is allocated
        }

        Ok(())
    }

    /// Generate code for all functions
    fn generate_functions(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        for function in &ir.functions {
            let func_addr = self.current_address;
            self.function_addresses.insert(function.id, func_addr);

            // Generate function header (local variable count + types)
            self.generate_function_header(function)?;

            // Generate function body
            self.generate_block(&function.body)?;

            // Ensure function ends with a return
            if !self.block_ends_with_return(&function.body) {
                self.emit_return(None)?;
            }
        }

        Ok(())
    }

    /// Generate function header with local variable declarations
    fn generate_function_header(&mut self, function: &IrFunction) -> Result<(), CompilerError> {
        // Z-Machine function header: 1 byte for local count + 2 bytes per local (v3 only)
        let local_count = function.local_vars.len();

        if local_count > 15 {
            return Err(CompilerError::CodeGenError(format!(
                "Function '{}' has {} locals, maximum is 15",
                function.name, local_count
            )));
        }

        self.emit_byte(local_count as u8)?;

        // In v3, emit default values for locals (v4+ doesn't need this)
        if self.version == ZMachineVersion::V3 {
            for _i in 0..local_count {
                self.emit_word(0)?; // Default local value = 0
            }
        }

        Ok(())
    }

    /// Generate code for a basic block
    fn generate_block(&mut self, block: &IrBlock) -> Result<(), CompilerError> {
        for instruction in &block.instructions {
            self.generate_instruction(instruction)?;
        }
        Ok(())
    }

    /// Check if a block ends with a return instruction
    fn block_ends_with_return(&self, block: &IrBlock) -> bool {
        matches!(
            block.instructions.last(),
            Some(IrInstruction::Return { .. })
        )
    }

    /// Generate code for a single IR instruction
    fn generate_instruction(&mut self, instruction: &IrInstruction) -> Result<(), CompilerError> {
        match instruction {
            IrInstruction::LoadImmediate {
                target: _target,
                value,
            } => {
                self.generate_load_immediate(value)?;
            }

            IrInstruction::BinaryOp {
                target: _target,
                op,
                left: _left,
                right: _right,
            } => {
                self.generate_binary_op(op)?;
            }

            IrInstruction::Call {
                target: _target,
                function: _function,
                args: _args,
            } => {
                self.generate_call()?;
            }

            IrInstruction::Return { value } => {
                self.emit_return(*value)?;
            }

            IrInstruction::Branch {
                condition: _condition,
                true_label,
                false_label: _false_label,
            } => {
                self.generate_branch(*true_label)?;
            }

            IrInstruction::Jump { label } => {
                self.generate_jump(*label)?;
            }

            IrInstruction::Label { id } => {
                self.label_addresses.insert(*id, self.current_address);
            }

            _ => {
                // TODO: Implement remaining instructions
                return Err(CompilerError::CodeGenError(format!(
                    "Instruction {:?} not yet implemented",
                    instruction
                )));
            }
        }

        Ok(())
    }

    /// Generate load immediate instruction
    fn generate_load_immediate(&mut self, value: &IrValue) -> Result<(), CompilerError> {
        match value {
            IrValue::Integer(n) => {
                // Use store instruction to load immediate value
                self.emit_byte(0x2D)?; // store opcode
                self.emit_word(*n as u16)?; // immediate value
                self.emit_byte(0x00)?; // store to local 0 (top of stack)
            }
            _ => {
                return Err(CompilerError::CodeGenError(
                    "Only integer immediates currently supported".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Generate binary operation
    fn generate_binary_op(&mut self, op: &IrBinaryOp) -> Result<(), CompilerError> {
        match op {
            IrBinaryOp::Add => self.emit_byte(0x14)?,      // add opcode
            IrBinaryOp::Subtract => self.emit_byte(0x15)?, // sub opcode
            IrBinaryOp::Multiply => self.emit_byte(0x16)?, // mul opcode
            IrBinaryOp::Divide => self.emit_byte(0x17)?,   // div opcode
            _ => {
                return Err(CompilerError::CodeGenError(format!(
                    "Binary operation {:?} not yet implemented",
                    op
                )));
            }
        }
        Ok(())
    }

    /// Generate function call
    fn generate_call(&mut self) -> Result<(), CompilerError> {
        // TODO: Implement proper call instruction with arguments
        self.emit_byte(0x20)?; // call_vs opcode (variable form)
        Ok(())
    }

    /// Generate return instruction
    fn emit_return(&mut self, value: Option<IrId>) -> Result<(), CompilerError> {
        if value.is_some() {
            // Return with value - use ret opcode
            self.emit_byte(0x8B)?; // ret opcode (1OP form)
                                   // TODO: Add operand for return value
        } else {
            // Return without value - use rtrue
            self.emit_byte(0xB0)?; // rtrue opcode (0OP form)
        }
        Ok(())
    }

    /// Generate branch instruction
    fn generate_branch(&mut self, _true_label: IrId) -> Result<(), CompilerError> {
        // TODO: Implement proper branching with condition and label resolution
        // For now, emit a simple jump
        self.emit_byte(0x8C)?; // jump opcode (1OP form)

        // Emit placeholder for branch offset (will be resolved later)
        self.emit_word(0x0000)?;

        // Record that we need to resolve this label
        // TODO: Track unresolved branches for later patching

        Ok(())
    }

    /// Generate unconditional jump
    fn generate_jump(&mut self, _label: IrId) -> Result<(), CompilerError> {
        self.emit_byte(0x8C)?; // jump opcode (1OP form)

        // Emit placeholder for jump offset (will be resolved later)
        self.emit_word(0x0000)?;

        // Record that we need to resolve this label
        // TODO: Track unresolved jumps for later patching

        Ok(())
    }

    /// Generate init block as the main program entry point
    fn generate_init_block(&mut self, _init_block: &IrBlock) -> Result<(), CompilerError> {
        // TODO: Generate init block code
        // For now, just emit a quit instruction
        self.emit_byte(0xBA)?; // quit opcode
        Ok(())
    }

    /// Write the Z-Machine file header
    fn write_header(&mut self) -> Result<(), CompilerError> {
        // Z-Machine header fields
        self.story_data[0] = match self.version {
            ZMachineVersion::V3 => 3,
            ZMachineVersion::V5 => 5,
        };

        // High memory base
        self.write_word_at(4, DEFAULT_HIGH_MEMORY)?;

        // Initial PC (entry point)
        self.write_word_at(6, DEFAULT_PC_START)?;

        // Dictionary address
        self.write_word_at(8, self.dictionary_addr as u16)?;

        // Object table address
        self.write_word_at(10, self.object_table_addr as u16)?;

        // Global variables address
        self.write_word_at(12, self.global_vars_addr as u16)?;

        // Static memory base (start of dictionary)
        self.write_word_at(14, self.dictionary_addr as u16)?;

        // File length (in 2-byte words for v3, 4-byte for v4+)
        let file_len = match self.version {
            ZMachineVersion::V3 => (self.story_data.len() / 2) as u16,
            ZMachineVersion::V5 => (self.story_data.len() / 4) as u16,
        };
        self.write_word_at(26, file_len)?;

        Ok(())
    }

    /// Resolve all address references and patch jumps/branches
    fn resolve_addresses(&mut self) -> Result<(), CompilerError> {
        // TODO: Implement address resolution for jumps, branches, and function calls
        // This requires tracking unresolved references during code generation

        Ok(())
    }

    // Utility methods for code emission

    /// Emit a single byte and advance current address
    fn emit_byte(&mut self, byte: u8) -> Result<(), CompilerError> {
        self.ensure_capacity(self.current_address + 1);
        self.story_data[self.current_address] = byte;
        self.current_address += 1;
        Ok(())
    }

    /// Emit a 16-bit word (big-endian) and advance current address
    fn emit_word(&mut self, word: u16) -> Result<(), CompilerError> {
        self.emit_byte((word >> 8) as u8)?;
        self.emit_byte(word as u8)?;
        Ok(())
    }

    /// Write a word at a specific address without changing current address
    fn write_word_at(&mut self, addr: usize, word: u16) -> Result<(), CompilerError> {
        self.ensure_capacity(addr + 2);
        self.story_data[addr] = (word >> 8) as u8;
        self.story_data[addr + 1] = word as u8;
        Ok(())
    }

    /// Ensure the story data buffer has enough capacity
    fn ensure_capacity(&mut self, required: usize) {
        if self.story_data.len() < required {
            self.story_data.resize(required, 0);
        }
    }
}

#[cfg(test)]
#[path = "codegen_tests.rs"]
mod tests;
