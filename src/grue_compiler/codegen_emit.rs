/// codegen_emit.rs
/// Image assembly methods for ZMachineCodeGen
///
use crate::grue_compiler::codegen::ZMachineCodeGen;
use crate::grue_compiler::error::CompilerError;
use log::debug;

impl ZMachineCodeGen {

    // Utility methods for code emission

    pub fn emit_byte(&mut self, byte: u8) -> Result<(), CompilerError> {
        // Clear labels at current address when we emit actual instruction bytes
        // (but not for padding or alignment bytes)
        if !self.labels_at_current_address.is_empty() && byte != 0x00 {
            log::debug!(
                "Clearing {} labels at address 0x{:04x} - instruction byte 0x{:02x} emitted",
                self.labels_at_current_address.len(),
                self.code_address,
                byte
            );
            self.labels_at_current_address.clear();
        }

        self.ensure_capacity(self.code_address + 1);

        // Remove verbose byte-by-byte logging - we'll log at instruction level instead

        // Use code_address which tracks our position within code_space
        let code_offset = self.code_address;

        // Ensure capacity
        if code_offset >= self.code_space.len() {
            self.code_space.resize(code_offset + 1, 0xFF); // Fill with 0xFF to detect uninitialized/skipped bytes
        }

        // Phase-aware writing: code generation writes to code_space, address patching writes to final_data
        if !self.final_data.is_empty() {
            // Final assembly phase: write to final_data only
            if self.code_address < self.final_data.len() {
                self.final_data[self.code_address] = byte;
            } else {
                return Err(CompilerError::CodeGenError(format!(
 "Cannot write byte at address 0x{:04x}: beyond final_data bounds (len: 0x{:04x})",
 self.code_address, self.final_data.len()
 )));
            }
        } else {
            // Code generation phase: write to code_space
            self.code_space[code_offset] = byte;
        }

        // Advance code_address to next position
        let old_addr = self.code_address;
        self.code_address = code_offset + 1;
        log::debug!(
 "📍 CODE_ADDRESS_INCREMENT: 0x{:04x} -> 0x{:04x} (offset {}) after emitting byte 0x{:02x}",
 old_addr,
 self.code_address,
 code_offset,
 byte
 );
        Ok(())
    }

    /// Emit a 16-bit word (big-endian) and advance code_address
    pub fn emit_word(&mut self, word: u16) -> Result<(), CompilerError> {
        let high_byte = (word >> 8) as u8;
        let low_byte = word as u8;

        debug!("Emit word: word=0x{:04x} -> high_byte=0x{:02x}, low_byte=0x{:02x} at code_address 0x{:04x}", word, high_byte, low_byte, self.code_address);

        // TEMPORARY DEBUG: Check for suspicious value
        if word == 0x019f || word == 415 {
            log::debug!(
                "CRITICAL BUG: emit_word called with 0x{:04x} (415) at code_address=0x{:04x}",
                word,
                self.code_address
            );
            log::debug!(
                "This will produce bytes 0x{:02x} 0x{:02x} which is our problem!",
                word >> 8,
                word & 0xff
            );
            panic!("FOUND THE BUG: emit_word is being called with 415 instead of 0xFFFF!");
        }

        // Also check if we're close to the problematic address
        if self.code_address >= 0x1278 && self.code_address <= 0x1285 {
            log::debug!(
                "emit_word at critical address 0x{:04x}: word=0x{:04x}",
                self.code_address,
                word
            );
        }

        // CRITICAL: Track exactly where null words come from
        if word == 0x0000 {
            log::debug!(
                " NULL_WORD_SOURCE: emit_word(0x0000) called at code_address 0x{:04x}",
                self.code_address
            );
            log::debug!(
                " This might be valid V3 default local values OR invalid placeholder operands"
            );
        }

        self.emit_byte(high_byte)?;
        self.emit_byte(low_byte)?;
        Ok(())
    }

    /// Write byte to globals space (global variables)
    pub fn write_to_globals_space(&mut self, offset: usize, byte: u8) -> Result<(), CompilerError> {
        // Ensure capacity
        if offset >= self.globals_space.len() {
            self.globals_space.resize(offset + 1, 0);
        }

        self.globals_space[offset] = byte;
        self.globals_address = self.globals_address.max(offset + 1);

        log::debug!(
            "🌐 GLOBALS_SPACE: offset={}, byte=0x{:02x}, space_len={}",
            offset,
            byte,
            self.globals_space.len()
        );
        Ok(())
    }
 
    /// Write a single byte at a specific address (no address advancement)
    pub fn write_byte_at(&mut self, addr: usize, byte: u8) -> Result<(), CompilerError> {
        // Direct write to final_data during address patching phase
        if addr < self.final_data.len() {
            self.final_data[addr] = byte;
            Ok(())
        } else {
            Err(CompilerError::CodeGenError(format!(
                "Cannot write byte at address 0x{:04x}: beyond final_data bounds (len: 0x{:04x})",
                addr,
                self.final_data.len()
            )))
        }
    }

    /// Write a 16-bit word at a specific address (no address advancement)
    /// ALL WRITES MUST GO THROUGH THIS FUNCTION FOR MONITORING
    fn write_word_at_safe(&mut self, addr: usize, word: u16) -> Result<(), CompilerError> {
        self.write_byte_at(addr, (word >> 8) as u8)?;
        self.write_byte_at(addr + 1, word as u8)?;
        Ok(())
    }

    /// Write a word at a specific address without changing current address
    fn write_word_at(&mut self, addr: usize, word: u16) -> Result<(), CompilerError> {
        self.ensure_capacity(addr + 2);

        // Use the monitored write functions - NO DIRECT WRITES ALLOWED
        self.write_word_at_safe(addr, word)?;
        Ok(())
    }
}