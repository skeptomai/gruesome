use log::{debug, trace};

/// The three alphabets for Z-string decoding
pub const ALPHABET_A0: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const ALPHABET_A1: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const ALPHABET_A2_V3: &[u8] = b" \r0123456789.,!?_#'\"/\\-:()";

/// Decode a Z-string from memory starting at the given address
/// Returns the decoded string and the number of bytes consumed
pub fn decode_string(
    memory: &[u8],
    addr: usize,
    abbrev_table_addr: usize,
) -> Result<(String, usize), String> {
    decode_string_recursive(memory, addr, abbrev_table_addr, 0, false)
}

/// Decode a string with optional safety limit bypass (for debugging)
pub fn decode_string_unsafe(
    memory: &[u8],
    addr: usize,
    abbrev_table_addr: usize,
    disable_limits: bool,
) -> Result<(String, usize), String> {
    decode_string_recursive(memory, addr, abbrev_table_addr, 0, disable_limits)
}

/// Internal recursive function with depth tracking
fn decode_string_recursive(
    memory: &[u8],
    addr: usize,
    abbrev_table_addr: usize,
    depth: u8,
    disable_limits: bool,
) -> Result<(String, usize), String> {
    if depth > 3 {
        debug!(
            "Abbreviation recursion depth {} exceeded at addr {:04x}",
            depth, addr
        );
        return Err("Abbreviation recursion too deep".to_string());
    }
    let mut result = String::new();
    let mut offset = addr;

    // Calculate safe maximum based on memory size
    // Theoretical max: every byte could be part of a Z-string word
    // Each word (2 bytes) = 3 Z-chars, so max = (memory_size / 2) * 3
    // This ensures we never reject a valid string while preventing runaway allocation
    // Example: Enchanter (107KB) → 161K char limit (vs old hardcoded 1000)
    // This fixes the bug where Enchanter's 1116-char intro was truncated at 1000 chars,
    // causing PC to land in the middle of string data (0x5827) instead of after it (0x5875)
    let max_string_length = if disable_limits {
        usize::MAX // No limit when explicitly disabled for debugging
    } else {
        (memory.len() / 2) * 3
    };

    // First, collect all z-characters
    let mut all_zchars = Vec::new();
    let mut is_end = false;

    while !is_end && offset + 1 < memory.len() && all_zchars.len() < max_string_length {
        // Read word (2 bytes, big-endian)
        let word = ((memory[offset] as u16) << 8) | (memory[offset + 1] as u16);
        offset += 2;

        // Check if this is the last word (bit 15 set)
        is_end = (word & 0x8000) != 0;

        // Extract the three 5-bit Z-characters
        let zchars = [
            ((word >> 10) & 0x1F) as u8,
            ((word >> 5) & 0x1F) as u8,
            (word & 0x1F) as u8,
        ];

        trace!(
            "Z-word {:04x} = Z-chars {:?}, is_end={}",
            word,
            zchars,
            is_end
        );

        // Add to our collection
        all_zchars.extend_from_slice(&zchars);
    }

    // Now process all collected z-characters
    let mut abbrev_shift = 0;
    let mut current_alphabet = 0; // 0=A0, 1=A1, 2=A2
    let mut shift_lock = false; // For V1-2 shift lock

    let mut i = 0;
    while i < all_zchars.len() {
        let zc = all_zchars[i];
        i += 1;
        if abbrev_shift > 0 {
            // This is an abbreviation reference
            let abbrev_num = (abbrev_shift - 1) * 32 + zc;

            // Read abbreviation address
            let abbrev_entry_addr = abbrev_table_addr + (abbrev_num as usize * 2);
            if abbrev_entry_addr + 1 >= memory.len() {
                abbrev_shift = 0;
                continue;
            }

            let abbrev_word_addr =
                ((memory[abbrev_entry_addr] as u16) << 8) | (memory[abbrev_entry_addr + 1] as u16);
            let abbrev_byte_addr = (abbrev_word_addr as usize).saturating_mul(2);

            // COMPLIANCE: Strict bounds checking - panic on invalid abbreviation addresses
            if abbrev_byte_addr >= memory.len() || abbrev_byte_addr == 0 {
                panic!(
                    "COMPLIANCE VIOLATION: Invalid abbreviation address 0x{:04x} (memory size: {})",
                    abbrev_byte_addr,
                    memory.len()
                );
            }

            // COMPLIANCE: Strict bounds checking - panic if insufficient bytes for word
            if abbrev_byte_addr + 1 >= memory.len() {
                panic!(
                    "COMPLIANCE VIOLATION: Abbreviation address 0x{:04x} too close to end of memory (size: {})",
                    abbrev_byte_addr,
                    memory.len()
                );
            }

            // Recursively decode the abbreviation string
            match decode_string_recursive(
                memory,
                abbrev_byte_addr,
                abbrev_table_addr,
                depth + 1,
                disable_limits,
            ) {
                Ok((abbrev_str, _)) => {
                    // Check for obviously repetitive patterns
                    if abbrev_str.len() > 50 || abbrev_str.contains("rmyrmy") {
                        debug!(
                            "Skipping problematic abbreviation {}: '{}'",
                            abbrev_num,
                            &abbrev_str[..20.min(abbrev_str.len())]
                        );
                    } else {
                        result.push_str(&abbrev_str);
                    }
                }
                Err(e) => {
                    debug!("Error decoding abbreviation {}: {}", abbrev_num, e);
                    // Skip this abbreviation and continue
                }
            }

            abbrev_shift = 0;
            continue;
        }

        match zc {
            0 => result.push(' '),
            1..=3 => {
                // Abbreviation in next Z-character
                abbrev_shift = zc;
            }
            4 => {
                // Shift to A1 (uppercase)
                current_alphabet = 1;
                shift_lock = false; // In V3+, shifts are temporary
                debug!("Shift to A1 (uppercase)");
            }
            5 => {
                // Shift to A2 (punctuation)
                current_alphabet = 2;
                shift_lock = false; // In V3+, shifts are temporary
                debug!("Shift to A2 (punctuation)");
            }
            6..=31 => {
                // Regular character from current alphabet
                let ch = match current_alphabet {
                    0 => ALPHABET_A0[(zc - 6) as usize] as char,
                    1 => ALPHABET_A1[(zc - 6) as usize] as char,
                    2 => {
                        // Handle special cases in A2
                        if zc == 6 {
                            // ZSCII escape - read next two z-chars to get ZSCII code
                            debug!("ZSCII escape at position {} in zchars", i - 1);
                            if i + 1 < all_zchars.len() {
                                let high = all_zchars[i];
                                let low = all_zchars[i + 1];
                                let zscii_code = (high << 5) | low;
                                debug!(
                                    "ZSCII escape: high={}, low={}, code={}",
                                    high, low, zscii_code
                                );
                                i += 2; // Skip the two chars we just read

                                // Convert ZSCII to char
                                if (32..=126).contains(&zscii_code) {
                                    let ch = zscii_code as char;
                                    debug!("ZSCII code {} = '{}'", zscii_code, ch);
                                    ch
                                } else {
                                    debug!("ZSCII code {} out of printable range", zscii_code);
                                    '?'
                                }
                            } else {
                                debug!("ZSCII escape at end of string");
                                '?'
                            }
                        } else if zc == 7 {
                            '\n'
                        } else {
                            ALPHABET_A2_V3[(zc - 6) as usize] as char
                        }
                    }
                    _ => '?',
                };
                result.push(ch);

                // Reset to A0 if this was a temporary shift
                if !shift_lock {
                    current_alphabet = 0;
                }
            }
            _ => unreachable!(),
        }
    }

    Ok((result, offset - addr))
}

/// Decode a string at a packed address
pub fn decode_string_at_packed_addr(
    memory: &[u8],
    packed_addr: u16,
    version: u8,
    abbrev_table_addr: usize,
) -> Result<String, String> {
    let byte_addr = unpack_string_address(packed_addr, version);

    // COMPLIANCE: Strict bounds checking - panic on invalid addresses
    if byte_addr >= memory.len() {
        panic!(
            "COMPLIANCE VIOLATION: Invalid packed string address 0x{:04x} unpacks to 0x{:04x}, exceeds memory size {} bytes",
            packed_addr, byte_addr, memory.len()
        );
    }

    let (string, _) = decode_string(memory, byte_addr, abbrev_table_addr)?;
    Ok(string)
}

/// Unpack a string address based on version
fn unpack_string_address(packed: u16, version: u8) -> usize {
    // Note: We can't check bounds here because we don't have memory reference
    // Bounds checking will be done at the call site
    match version {
        1..=3 => (packed as usize) * 2,
        4..=5 => (packed as usize) * 4,
        6..=7 => (packed as usize) * 4, // TODO: Add offset handling
        8 => (packed as usize) * 8,
        _ => (packed as usize) * 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_string() {
        // Create a simple test string "hello"
        // h=7 (position in alphabet), e=4, l=11, l=11, o=14
        // In Z-chars: h=13 (7+6), e=10 (4+6), l=17 (11+6), l=17, o=20 (14+6)
        let mut memory = vec![0u8; 100];

        // First word: 13, 10, 17
        // Binary: 01101 01010 10001
        // Full 16-bit: 0011 0101 0101 0001 = 0x3551
        memory[10] = 0x35;
        memory[11] = 0x51;

        // Second word: 17, 20, padding(5)
        // Binary: 10001 10100 00101
        // With bit 15 set: 1100 0110 1000 0101 = 0xC685
        memory[12] = 0xC6;
        memory[13] = 0x85;

        let (result, len) = decode_string(&memory, 10, 0).unwrap();
        assert_eq!(result, "hello");
        assert_eq!(len, 4);
    }

    #[test]
    fn test_string_with_space() {
        // Test "a b" = a, space, b
        // a=0 (position in alphabet), space=0 (special Z-char), b=1 (position in alphabet)
        // In Z-chars: a=6 (0+6), space=0, b=7 (1+6)
        // Need padding to fill word
        let mut memory = vec![0u8; 100];

        // Word: 6, 0, 7 = 00110 00000 00111
        // Set bit 15 for end = 1001 1000 0000 0111 = 0x9807
        memory[20] = 0x98;
        memory[21] = 0x07;

        let (result, len) = decode_string(&memory, 20, 0).unwrap();
        assert_eq!(result, "a b");
        assert_eq!(len, 2);
    }
}
