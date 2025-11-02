/// Z-Machine Memory Constants and Utilities
///
/// This module contains constants, enums, and utility functions related to Z-Machine
/// memory management and address calculation.
///
/// Extracted from codegen.rs as part of Plan A refactoring to reduce file size
/// and improve code organization.
///
/// # Constants
///
/// - `HEADER_SIZE`: Fixed 64-byte Z-Machine header size
/// - `PLACEHOLDER_BYTE`: 0xFF byte used for unresolved address placeholders
///
/// # Functions
///
/// - `placeholder_word()`: Creates 16-bit placeholder (0xFFFF) for address resolution

/// Z-Machine memory layout constants
pub const HEADER_SIZE: usize = 64; // Fixed 64-byte header
pub const DEFAULT_HIGH_MEMORY: u16 = 0x8000; // Start of high memory

/// Distinctive placeholder byte for unresolved references
/// 0xFF is chosen because:
/// 1. In Z-Machine, 0xFF as an instruction byte would be an invalid Extended form instruction
/// 2. As operand data, 0xFFFF would represent -1 or 65535, which are uncommon values
/// 3. It's easily recognizable in hex dumps as "unresolved"
/// 4. Creates a clear pattern when examining bytecode (FFFF stands out)
pub const PLACEHOLDER_BYTE: u8 = 0xFF;

/// Create a 16-bit placeholder value using the distinctive placeholder byte
pub const fn placeholder_word() -> u16 {
    ((PLACEHOLDER_BYTE as u16) << 8) | (PLACEHOLDER_BYTE as u16)
}

/// Memory space types for the separated compilation model
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemorySpace {
    Header,
    Globals,
    Abbreviations,
    Objects,
    Dictionary,
    Strings,
    Code,
    CodeSpace, // Alternative name for Code
}
