// Z-Machine Header Generation and Address Fixup System
//
// Handles Z-Machine header generation, memory layout calculation, and address resolution
// for the Z-Machine bytecode compiler.

use crate::grue_compiler::error::CompilerError;
use crate::grue_compiler::ZMachineVersion;

// Constants for header processing
const HEADER_SIZE: usize = 64; // Fixed 64-byte header
const PLACEHOLDER_BYTE: u8 = 0xFF;

// Re-export common types for header handling
pub use crate::grue_compiler::codegen::{
    MemorySpace, PendingFixup, ReferenceType, UnresolvedReference, ZMachineCodeGen,
};

impl ZMachineCodeGen {
    /// Phase 1: Generate static header fields (version, serial, flags)
    ///
    /// Writes only fields that don't depend on final memory layout.
    /// Address fields are left as 0x0000 placeholders.
    ///
    pub fn generate_static_header_fields(&mut self) -> Result<(), CompilerError> {
        log::debug!("📝 Phase 1: Generating static header fields (version, serial, flags)");

        // Z-Machine header is always 64 bytes, write directly to final_data
        let header = &mut self.final_data[0..HEADER_SIZE];

        // Byte 0: Z-Machine version (3, 4, or 5)
        header[0] = match self.version {
            ZMachineVersion::V3 => 3,
            ZMachineVersion::V4 => 4,
            ZMachineVersion::V5 => 5,
        };

        // Byte 1: Flags 1 (default for our version)
        header[1] = 0x00;

        // Bytes 2-3: Release number (required, use 1 for our compiler)
        header[2] = 0x00; // Release high byte
        header[3] = 0x01; // Release low byte = 1

        // Bytes 16-17: Flags 2 (default)
        header[16] = 0x00;
        header[17] = 0x00;

        // Bytes 18-23 (0x12-0x17): Serial number (6 ASCII bytes) - Infocom convention
        let serial = b"250905"; // YYMMDD format: September 5, 2025
        header[18] = serial[0]; // '2'
        header[19] = serial[1]; // '5'
        header[20] = serial[2]; // '0'
        header[21] = serial[3]; // '9'
        header[22] = serial[4]; // '0'
        header[23] = serial[5]; // '5'

        // Bytes 50-51: Standard revision number (Z-Machine spec 1.1)
        header[50] = 0x01; // Standard revision 1
        header[51] = 0x01; // Sub-revision 1

        log::debug!(
            "📝 Static header fields: Version {}, Serial {}",
            header[0],
            std::str::from_utf8(serial).unwrap()
        );

        Ok(())
    }

    /// Phase 2: Update header address fields with final memory layout
    pub fn fixup_header_addresses(
        &mut self,
        pc_start: u16,
        dictionary_addr: u16,
        objects_addr: u16,
        globals_addr: u16,
        static_memory_base: u16,
        abbreviations_addr: u16,
        high_mem_base: u16,
    ) -> Result<(), CompilerError> {
        log::debug!("📝 Phase 2: Updating header address fields with final memory layout");

        let header = &mut self.final_data[0..HEADER_SIZE];

        // Bytes 4-5: High memory base (start of high memory section)
        header[4] = (high_mem_base >> 8) as u8;
        header[5] = (high_mem_base & 0xFF) as u8;

        // Bytes 6-7: PC initial value (start of executable code section)
        header[6] = (pc_start >> 8) as u8;
        header[7] = (pc_start & 0xFF) as u8;

        // Bytes 8-9: Dictionary address
        header[8] = (dictionary_addr >> 8) as u8;
        header[9] = (dictionary_addr & 0xFF) as u8;

        // Bytes 10-11: Object table address
        header[10] = (objects_addr >> 8) as u8;
        header[11] = (objects_addr & 0xFF) as u8;

        // Bytes 12-13: Global variables address
        header[12] = (globals_addr >> 8) as u8;
        header[13] = (globals_addr & 0xFF) as u8;

        // Bytes 14-15: Static memory base (end of dynamic memory)
        header[14] = (static_memory_base >> 8) as u8;
        header[15] = (static_memory_base & 0xFF) as u8;

        // Bytes 24-25 (0x18-0x19): Abbreviations address
        header[24] = (abbreviations_addr >> 8) as u8;
        header[25] = (abbreviations_addr & 0xFF) as u8;

        log::debug!(
            "📝 Address fields updated: PC=0x{:04x}, Dict=0x{:04x}, Obj=0x{:04x}",
            pc_start,
            dictionary_addr,
            objects_addr
        );

        Ok(())
    }

    /// Finalize header metadata (checksums, high/low water marks)  
    pub fn finalize_header_metadata(&mut self) -> Result<(), CompilerError> {
        log::debug!("📝 Phase 3: Finalizing header metadata");

        // Calculate values first before borrowing header mutably
        let file_length = self.final_data.len();
        let packed_length = match self.version {
            ZMachineVersion::V3 => file_length / 2,
            ZMachineVersion::V4 | ZMachineVersion::V5 => file_length / 4,
        };
        let checksum = self.calculate_checksum();

        // Now borrow header mutably and write the values
        let header = &mut self.final_data[0..HEADER_SIZE];

        // Bytes 26-27: File length (in bytes / 2 for v3, /4 for v4+)
        header[26] = (packed_length >> 8) as u8;
        header[27] = (packed_length & 0xFF) as u8;

        // Bytes 28-29: Checksum (calculated over entire file except checksum bytes)
        header[28] = (checksum >> 8) as u8;
        header[29] = (checksum & 0xFF) as u8;

        log::debug!(
            "📝 Metadata finalized: File length={}, Checksum=0x{:04x}",
            file_length,
            checksum
        );

        Ok(())
    }

    /// Calculate Z-Machine checksum for the assembled data
    fn calculate_checksum(&self) -> u16 {
        let mut sum: u32 = 0;

        // Sum all bytes except the checksum bytes themselves (28-29)
        for (i, &byte) in self.final_data.iter().enumerate() {
            if i != 28 && i != 29 {
                sum = sum.wrapping_add(byte as u32);
            }
        }

        // Z-Machine checksum is the low 16 bits
        (sum & 0xFFFF) as u16
    }

    /// Write header space data to specific offset  
    pub fn write_to_header_space(&mut self, offset: usize, byte: u8) -> Result<(), CompilerError> {
        if offset >= HEADER_SIZE {
            return Err(CompilerError::CodeGenError(format!(
                "Header offset {} exceeds header size {}",
                offset, HEADER_SIZE
            )));
        }

        log::debug!(
            "📝 HEADER_SPACE: Write 0x{:02x} at offset 0x{:04x}",
            byte,
            offset
        );
        self.final_data[offset] = byte;
        Ok(())
    }

    /// Write final header with entry point address
    pub fn write_final_header(&mut self, init_entry_point: usize) -> Result<(), CompilerError> {
        log::debug!(
            "📝 FINAL_HEADER: Writing with entry point 0x{:04x}",
            init_entry_point
        );

        // Entry point goes in bytes 6-7 (PC initial value)
        let header = &mut self.final_data[0..HEADER_SIZE];
        header[6] = (init_entry_point >> 8) as u8;
        header[7] = (init_entry_point & 0xFF) as u8;

        log::debug!("📝 Final header written with PC=0x{:04x}", init_entry_point);
        Ok(())
    }

    /// Legacy header writing method (for compatibility)
    pub fn write_header_to_final_data(&mut self, _entry_point: usize) -> Result<(), CompilerError> {
        // This method is kept for compatibility but functionality has been moved
        // to the more comprehensive header generation pipeline
        log::debug!("📝 Legacy header write call (functionality delegated to pipeline)");
        Ok(())
    }

    /// Address Resolution & Memory Layout Functions

    /// Set final assembly address with logging for debugging
    pub fn set_final_assembly_address(&mut self, new_addr: usize, context: &str) {
        let old_addr = self.final_assembly_address;
        self.final_assembly_address = new_addr;
        log::warn!(
            "🔄 FINAL_ASSEMBLY_ADDRESS_UPDATE: {} | 0x{:04x} → 0x{:04x} (delta: {:+})",
            context,
            old_addr,
            new_addr,
            new_addr as i32 - old_addr as i32
        );
    }

    /// Resolve a single fixup in the final assembled data
    pub fn resolve_fixup(&mut self, fixup: &PendingFixup) -> Result<(), CompilerError> {
        let final_source_address = match fixup.source_space {
            MemorySpace::Header => 64 + fixup.source_address,
            MemorySpace::Globals => 64 + 480 + fixup.source_address,
            MemorySpace::Abbreviations => 64 + 480 + 192 + fixup.source_address,
            MemorySpace::Objects => self.final_object_base + fixup.source_address,
            MemorySpace::Dictionary => {
                64 + 480 + 192 + self.object_space.len() + fixup.source_address
            }
            MemorySpace::Strings => self.final_string_base + fixup.source_address,
            MemorySpace::Code => self.final_code_base + fixup.source_address,
            MemorySpace::CodeSpace => self.final_code_base + fixup.source_address, // Same as Code
        };

        let target_address = match &fixup.reference_type {
            ReferenceType::StringRef { string_id } => {
                if let Some(&string_offset) = self.string_offsets.get(string_id) {
                    let final_addr = self.final_string_base + string_offset;
                    // For Z-Machine, string addresses are packed (divided by 2 for v3, 4 for v4+)
                    match self.version {
                        ZMachineVersion::V3 => final_addr / 2,
                        ZMachineVersion::V4 | ZMachineVersion::V5 => final_addr / 4,
                    }
                } else {
                    return Err(CompilerError::CodeGenError(format!(
                        "String ID {} not found in string_offsets",
                        string_id
                    )));
                }
            }

            ReferenceType::ObjectRef { object_id } => {
                if let Some(&object_offset) = self.object_offsets.get(object_id) {
                    self.final_object_base + object_offset
                } else {
                    return Err(CompilerError::CodeGenError(format!(
                        "Object ID {} not found in object_offsets",
                        object_id
                    )));
                }
            }

            ReferenceType::RoutineCall { routine_id } => {
                if let Some(&code_label_addr) = self.code_labels.get(routine_id) {
                    let final_addr = self.final_code_base + code_label_addr;
                    // Routine addresses are packed like strings
                    match self.version {
                        ZMachineVersion::V3 => final_addr / 2,
                        ZMachineVersion::V4 | ZMachineVersion::V5 => final_addr / 4,
                    }
                } else {
                    return Err(CompilerError::CodeGenError(format!(
                        "Routine ID {} not found in code_labels",
                        routine_id
                    )));
                }
            }
        };

        // Apply the fixup to final_data
        if final_source_address + fixup.operand_size > self.final_data.len() {
            return Err(CompilerError::CodeGenError(format!(
                "Fixup address out of bounds: 0x{:04x}",
                final_source_address
            )));
        }

        if fixup.operand_size == 2 {
            let word_value = target_address as u16;
            self.final_data[final_source_address] = (word_value >> 8) as u8;
            self.final_data[final_source_address + 1] = word_value as u8;
            log::trace!(
                "🔧 FIXED_WORD: addr=0x{:04x}, value=0x{:04x}",
                final_source_address,
                word_value
            );
        } else {
            let byte_value = target_address as u8;
            self.final_data[final_source_address] = byte_value;
            log::trace!(
                "🔧 FIXED_BYTE: addr=0x{:04x}, value=0x{:02x}",
                final_source_address,
                byte_value
            );
        }

        Ok(())
    }
}
