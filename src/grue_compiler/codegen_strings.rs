// Z-Machine String & Dictionary Processing System
//
// Handles string collection, encoding, dictionary generation, and text processing
// for the Z-Machine bytecode compiler.

use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ir::*;
use crate::grue_compiler::ZMachineVersion;
use log::debug;

// Re-export common types for string handling
pub use crate::grue_compiler::codegen::{MemorySpace, ZMachineCodeGen};

impl ZMachineCodeGen {
    /// String Collection Functions

    /// Collect all strings from the IR program for later encoding
    pub fn collect_strings(&mut self, ir: &IrProgram) -> Result<(), CompilerError> {
        // Collect from string table in deterministic order (sorted by ID)
        let mut string_entries: Vec<_> = ir.string_table.iter().collect();
        string_entries.sort_by_key(|(_, &id)| id); // Sort by string ID for stable allocation order

        for (string, &id) in string_entries {
            self.strings.push((id, string.clone()));
        }

        // Collect strings from LoadImmediate instructions in all functions
        for function in &ir.functions {
            self.collect_strings_from_block(&function.body)?;
        }

        // Collect strings from init block if present
        if let Some(init_block) = &ir.init_block {
            self.collect_strings_from_block(init_block)?;
        }

        // Collect strings from rooms
        for room in &ir.rooms {
            // Collect room display name
            if !room.display_name.is_empty() {
                let string_id = self.find_or_create_string_id(&room.display_name)?;
                debug!(
                    "🏠 Collected room display name: '{}' -> ID {}",
                    room.display_name, string_id
                );
            }

            // Collect room description
            if !room.description.is_empty() {
                let string_id = self.find_or_create_string_id(&room.description)?;
                debug!(
                    "🏠 Collected room description: '{}' -> ID {}",
                    room.description, string_id
                );
            }

            // Collect strings from room event blocks
            if let Some(on_enter) = &room.on_enter {
                self.collect_strings_from_block(on_enter)?;
            }
            if let Some(on_exit) = &room.on_exit {
                self.collect_strings_from_block(on_exit)?;
            }
            if let Some(on_look) = &room.on_look {
                self.collect_strings_from_block(on_look)?;
            }
        }

        // Collect strings from objects
        for object in &ir.objects {
            // Collect object short name
            if !object.short_name.is_empty() {
                let string_id = self.find_or_create_string_id(&object.short_name)?;
                debug!(
                    "📦 Collected object short name: '{}' -> ID {}",
                    object.short_name, string_id
                );
            }

            // Collect object description
            if !object.description.is_empty() {
                let string_id = self.find_or_create_string_id(&object.description)?;
                debug!(
                    "📦 Collected object description: '{}' -> ID {}",
                    object.description, string_id
                );
            }

            // Collect object vocabulary names
            for name in &object.names {
                if !name.is_empty() {
                    let string_id = self.find_or_create_string_id(name)?;
                    debug!("📦 Collected object name: '{}' -> ID {}", name, string_id);
                }
            }

            // Collect strings from object properties
            for (_, prop_value) in &object.properties.properties {
                if let IrPropertyValue::String(s) = prop_value {
                    if !s.is_empty() {
                        let string_id = self.find_or_create_string_id(s)?;
                        debug!(
                            "📦 Collected object property string: '{}' -> ID {}",
                            s, string_id
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Add main loop strings to the collection and return their IDs
    pub fn add_main_loop_strings(&mut self) -> Result<IrId, CompilerError> {
        // Add specific strings needed for main loop functionality
        // Allocate dynamically after all other strings to avoid conflicts
        let prompt_id = self.find_or_create_string_id("> ")?;
        debug!("🎯 Allocated main loop prompt string ID: {}", prompt_id);
        Ok(prompt_id)
    }

    /// Collect strings from instructions in a block  
    pub fn collect_strings_from_block(&mut self, block: &IrBlock) -> Result<(), CompilerError> {
        for instruction in &block.instructions {
            match instruction {
                IrInstruction::LoadImmediate {
                    target,
                    value: IrValue::String(s),
                } => {
                    self.strings.push((*target, s.clone()));
                }
                _ => {} // Other instructions don't contain strings
            }
        }
        Ok(())
    }

    /// String Encoding Functions

    /// Encode all collected strings using Z-Machine ZSCII encoding
    pub fn encode_all_strings(&mut self) -> Result<(), CompilerError> {
        debug!(
            "🔤 Encoding {} strings using Z-Machine ZSCII",
            self.strings.len()
        );
        for (id, string) in &self.strings {
            let encoded = self.encode_string(string)?;
            self.encoded_strings.insert(*id, encoded);
        }
        Ok(())
    }

    /// Encode a single string using Z-Machine ZSCII encoding
    pub fn encode_string(&self, s: &str) -> Result<Vec<u8>, CompilerError> {
        // Z-Machine text encoding per Z-Machine Standard 1.1, Section 3.5.3
        // Alphabet A0 (6-31): abcdefghijklmnopqrstuvwxyz
        // Alphabet A1 (6-31): ABCDEFGHIJKLMNOPQRSTUVWXYZ
        // Alphabet A2 (6-31):  ^0123456789.,!?_#'"/\-:()

        let mut zchars = Vec::new();

        for ch in s.chars() {
            match ch {
                // Space is always Z-character 0
                ' ' => zchars.push(0),

                // Alphabet A0: lowercase letters (Z-chars 6-31)
                'a'..='z' => {
                    zchars.push(ch as u8 - b'a' + 6);
                }

                // Alphabet A1: uppercase letters (single-shift with 4, then Z-char 6-31)
                'A'..='Z' => {
                    zchars.push(4); // Single shift to alphabet A1
                    zchars.push(ch as u8 - b'A' + 6);
                }

                // Alphabet A2: digits and punctuation (single-shift with 5, then Z-char 6-31)
                '0'..='9' => {
                    zchars.push(5); // Single shift to alphabet A2
                    zchars.push(ch as u8 - b'0' + 7); // A2[7-16] = "0123456789"
                }

                '.' => {
                    zchars.push(5);
                    zchars.push(17); // A2[17] = '.'
                }

                ',' => {
                    zchars.push(5);
                    zchars.push(18); // A2[18] = ','
                }

                '!' => {
                    zchars.push(5);
                    zchars.push(19); // A2[19] = '!'
                }

                '?' => {
                    zchars.push(5);
                    zchars.push(20); // A2[20] = '?'
                }

                '_' => {
                    zchars.push(5);
                    zchars.push(21); // A2[21] = '_'
                }

                '#' => {
                    zchars.push(5);
                    zchars.push(22); // A2[22] = '#'
                }

                '\'' => {
                    zchars.push(5);
                    zchars.push(23); // A2[23] = '\''
                }

                '"' => {
                    zchars.push(5);
                    zchars.push(24); // A2[24] = '"'
                }

                '/' => {
                    zchars.push(5);
                    zchars.push(25); // A2[25] = '/'
                }

                '\\' => {
                    zchars.push(5);
                    zchars.push(26); // A2[26] = '\'
                }

                '-' => {
                    zchars.push(5);
                    zchars.push(27); // A2[27] = '-'
                }

                ':' => {
                    zchars.push(5);
                    zchars.push(28); // A2[28] = ':'
                }

                '(' => {
                    zchars.push(5);
                    zchars.push(29); // A2[29] = '('
                }

                ')' => {
                    zchars.push(5);
                    zchars.push(30); // A2[30] = ')'
                }

                '\n' => {
                    zchars.push(5);
                    zchars.push(7); // Newline encoding
                }

                // Handle other characters with escape sequence
                _ => {
                    // Use escape sequence for characters not in standard alphabets
                    let unicode_val = ch as u32;
                    if unicode_val <= 255 {
                        zchars.push(5); // Shift to A2
                        zchars.push(6); // Escape sequence
                        zchars.push(((unicode_val >> 5) & 0x1F) as u8);
                        zchars.push((unicode_val & 0x1F) as u8);
                    } else {
                        return Err(CompilerError::CodeGenError(format!(
                            "Unicode character '{}' (U+{:04X}) cannot be encoded in Z-Machine text",
                            ch, unicode_val
                        )));
                    }
                }
            }
        }

        // Pack Z-characters into bytes (3 Z-chars per 2 bytes)
        let mut bytes = Vec::new();
        let mut i = 0;

        while i < zchars.len() {
            let z1 = zchars.get(i).copied().unwrap_or(5);
            let z2 = zchars.get(i + 1).copied().unwrap_or(5);
            let z3 = zchars.get(i + 2).copied().unwrap_or(5);

            // Pack: [z1: 5 bits][z2: 5 bits][z3: 5 bits][end: 1 bit] = 16 bits
            let mut word = ((z1 as u16) << 10) | ((z2 as u16) << 5) | (z3 as u16);

            // Set end bit if this is the last group of characters
            if i + 3 >= zchars.len() {
                word |= 0x8000; // Set bit 15 (end bit)
            }

            // Store as big-endian bytes
            bytes.push((word >> 8) as u8);
            bytes.push(word as u8);

            i += 3;
        }

        debug!(
            "🔤 Encoded string '{}' -> {} Z-chars -> {} bytes",
            s,
            zchars.len(),
            bytes.len()
        );
        Ok(bytes)
    }

    /// Dictionary Generation Functions

    /// Generate dictionary space with minimal word parsing dictionary
    pub fn generate_dictionary_space(&mut self, _ir: &IrProgram) -> Result<(), CompilerError> {
        debug!("📚 Generating minimal dictionary for Z-Machine compliance");

        // Z-Machine requires a dictionary even if minimal
        // Format: [entry length][number of entries][entries...]
        // For simplicity, create minimal dictionary with word separators only

        let dictionary_data = vec![
            0x06, // Entry length: 6 bytes per entry
            0x00, // Number of entries: 0 (minimal dictionary)
            0x00, // High byte of entry count
            0x00, // Padding to maintain alignment
        ];

        // Allocate dictionary space and write data
        self.allocate_dictionary_space(dictionary_data.len())?;
        for (i, &byte) in dictionary_data.iter().enumerate() {
            self.write_to_dictionary_space(i, byte)?;
        }

        debug!(
            "📚 Dictionary space generated: {} bytes",
            dictionary_data.len()
        );
        Ok(())
    }

    /// Generate complete dictionary from IR program (future expansion)
    pub fn generate_dictionary(&mut self, _ir: &IrProgram) -> Result<(), CompilerError> {
        debug!("📚 Dictionary generation (placeholder for future expansion)");

        // This is a placeholder for full dictionary generation
        // For now, delegate to minimal dictionary generation
        self.generate_dictionary_space(_ir)?;

        Ok(())
    }

    /// String Utility Functions

    /// Find or create a string ID for the given string
    pub fn find_or_create_string_id(&mut self, s: &str) -> Result<IrId, CompilerError> {
        // Check if string already exists
        for (id, existing_string) in &self.strings {
            if existing_string == s {
                return Ok(*id);
            }
        }

        // Create new string ID
        let new_id: IrId = self.next_string_id;
        self.next_string_id += 1;

        self.strings.push((new_id, s.to_string()));
        debug!("🔤 Created new string ID {} for '{}'", new_id, s);

        Ok(new_id)
    }

    /// Get string value from IR ID
    pub fn get_string_value(&self, ir_id: IrId) -> Result<String, CompilerError> {
        // Check strings collection first
        for (id, string) in &self.strings {
            if *id == ir_id {
                return Ok(string.clone());
            }
        }

        // Check IR ID to string mapping
        if let Some(string) = self.ir_id_to_string.get(&ir_id) {
            return Ok(string.clone());
        }

        Err(CompilerError::CodeGenError(format!(
            "String value not found for IR ID {}",
            ir_id
        )))
    }

    /// Address and Space Management

    /// Allocate space in string space and return offset
    pub fn allocate_string_space(
        &mut self,
        string_id: IrId,
        data: &[u8],
    ) -> Result<usize, CompilerError> {
        let offset = self.string_address;
        let size = data.len();

        // Ensure capacity
        if self.string_address + size > self.string_space.len() {
            self.string_space.resize(self.string_address + size, 0);
        }

        // Copy the data to string space
        for (i, &byte) in data.iter().enumerate() {
            self.string_space[self.string_address + i] = byte;
        }

        // Record the string offset for later reference resolution
        self.string_offsets.insert(string_id, offset);

        self.string_address += size;
        debug!(
            "🔤 STRING_ALLOCATED: ID={}, offset=0x{:04x}, size={}",
            string_id, offset, size
        );

        Ok(offset)
    }

    /// Allocate space in dictionary space and return offset
    pub fn allocate_dictionary_space(&mut self, size: usize) -> Result<usize, CompilerError> {
        let offset = self.dictionary_address;

        // Ensure capacity
        if self.dictionary_address + size > self.dictionary_space.len() {
            self.dictionary_space
                .resize(self.dictionary_address + size, 0);
        }

        self.dictionary_address += size;
        debug!(
            "📚 DICTIONARY_ALLOCATED: offset=0x{:04x}, size={}",
            offset, size
        );

        Ok(offset)
    }

    /// Write to string space at specific offset
    pub fn write_to_string_space(&mut self, offset: usize, byte: u8) -> Result<(), CompilerError> {
        if offset >= self.string_space.len() {
            self.string_space.resize(offset + 1, 0);
        }

        debug!(
            "🔤 STRING_SPACE: Write 0x{:02x} at offset 0x{:04x}",
            byte, offset
        );
        self.string_space[offset] = byte;
        Ok(())
    }

    /// Write to dictionary space at specific offset
    pub fn write_to_dictionary_space(
        &mut self,
        offset: usize,
        byte: u8,
    ) -> Result<(), CompilerError> {
        if offset >= self.dictionary_space.len() {
            self.dictionary_space.resize(offset + 1, 0);
        }

        debug!(
            "📚 DICTIONARY_SPACE: Write 0x{:02x} at offset 0x{:04x}",
            byte, offset
        );
        self.dictionary_space[offset] = byte;
        Ok(())
    }

    /// Pack string address for Z-Machine (divide by 2 for v3, 4 for v4+)
    pub fn pack_string_address(&self, byte_address: usize) -> Result<u16, CompilerError> {
        let packed = match self.version {
            ZMachineVersion::V3 => {
                if byte_address % 2 != 0 {
                    return Err(CompilerError::CodeGenError(format!(
                        "V3 string address 0x{:04x} is not even-aligned",
                        byte_address
                    )));
                }
                byte_address / 2
            }
            ZMachineVersion::V4 | ZMachineVersion::V5 => {
                if byte_address % 4 != 0 {
                    return Err(CompilerError::CodeGenError(format!(
                        "V4/V5 string address 0x{:04x} is not 4-byte aligned",
                        byte_address
                    )));
                }
                byte_address / 4
            }
        };

        if packed > 0xFFFF {
            return Err(CompilerError::CodeGenError(format!(
                "Packed string address 0x{:04x} exceeds 16-bit limit",
                packed
            )));
        }

        Ok(packed as u16)
    }

    /// Allocate string address for IR ID with proper alignment
    pub fn allocate_string_address(&mut self, ir_id: IrId, string_length: usize) -> usize {
        let address = self.string_address;

        // Allocate space and update address
        self.string_address += string_length;

        // Record the address for this string ID
        self.string_offsets.insert(ir_id, address);

        debug!(
            "🔤 STRING_ADDRESS: IR ID {} -> address 0x{:04x} (length {})",
            ir_id, address, string_length
        );

        address
    }

    /// String Processing Functions for IR Translation

    /// Translate to_string builtin function calls
    pub fn translate_to_string_builtin_inline(
        &mut self,
        args: &[IrId],
        target: Option<IrId>,
    ) -> Result<(), CompilerError> {
        if args.len() != 1 {
            return Err(CompilerError::CodeGenError(
                "to_string() requires exactly 1 argument".to_string(),
            ));
        }

        // For now, implement as a placeholder that converts the argument to string
        // This is a simplified implementation
        let arg_operand = self.resolve_ir_id_to_operand(args[0])?;

        if let Some(target_id) = target {
            // Store result in IR mapping
            self.ir_id_to_string
                .insert(target_id, format!("{:?}", arg_operand));
            debug!("🔤 to_string builtin: IR {} -> string mapping", target_id);
        }

        Ok(())
    }

    /// Translate string concatenation operations
    pub fn translate_string_concatenation(
        &mut self,
        left: IrId,
        right: IrId,
        target: IrId,
    ) -> Result<(), CompilerError> {
        // Get the string values for left and right operands
        let left_str = self.get_string_value(left)?;
        let right_str = self.get_string_value(right)?;

        // Concatenate the strings
        let result_str = format!("{}{}", left_str, right_str);

        // Create new string entry and encode it
        let encoded = self.encode_string(&result_str)?;
        self.encoded_strings.insert(target, encoded);
        self.ir_id_to_string.insert(target, result_str.clone());

        debug!(
            "🔤 String concatenation: '{}' + '{}' = '{}' (IR ID {})",
            left_str, right_str, result_str, target
        );

        Ok(())
    }

    /// Generate string concatenation instruction
    pub fn generate_string_concatenation(
        &mut self,
        left: IrId,
        right: IrId,
        target: IrId,
    ) -> Result<(), CompilerError> {
        // This is a more complex version that generates actual Z-Machine instructions
        // For now, delegate to the simpler translate version
        self.translate_string_concatenation(left, right, target)
    }

    /// Encode object name for Z-Machine object table
    pub fn encode_object_name(&self, name: &str) -> Vec<u8> {
        // Encode object name using Z-Machine text encoding
        // This is a simplified version - in full implementation would use ZSCII
        let mut encoded = Vec::new();
        for byte in name.bytes().take(8) {
            // Z-Machine object names are limited
            encoded.push(byte);
        }

        // Pad to minimum length if needed
        while encoded.len() < 4 {
            encoded.push(0);
        }

        encoded
    }

    /// Encode property value with proper size calculation
    pub fn encode_property_value(
        &self,
        prop_num: u8,
        prop_value: &IrPropertyValue,
    ) -> (u8, Vec<u8>) {
        let data = match prop_value {
            IrPropertyValue::String(s) => {
                // Encode string using Z-Machine encoding
                if let Ok(encoded) = self.encode_string(s) {
                    encoded
                } else {
                    Vec::new()
                }
            }
            IrPropertyValue::Byte(b) => {
                // Single byte property
                vec![*b]
            }
            IrPropertyValue::Word(w) => {
                // Two-byte property (big-endian)
                vec![(w >> 8) as u8, (*w & 0xFF) as u8]
            }
            IrPropertyValue::Bytes(bytes) => {
                // Multi-byte property
                bytes.clone()
            }
        };

        // Calculate size byte (property header format)
        let size = data.len().min(63) as u8;
        let size_byte = (prop_num << 2) | (size & 0x3F);

        (size_byte, data)
    }
}
