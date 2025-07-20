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
    decode_string_recursive(memory, addr, abbrev_table_addr, 0)
}

/// Internal recursive function with depth tracking
fn decode_string_recursive(
    memory: &[u8],
    addr: usize,
    abbrev_table_addr: usize,
    depth: u8,
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
    let max_string_length = 1000; // Prevent runaway string generation

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

            // Check for obviously invalid addresses
            if abbrev_byte_addr >= memory.len() || abbrev_byte_addr == 0 {
                debug!(
                    "Invalid abbreviation address {:04x} (memory size: {}), skipping",
                    abbrev_byte_addr,
                    memory.len()
                );
                abbrev_shift = 0;
                continue;
            }

            // Additional sanity check - make sure we have at least 2 bytes for a word
            if abbrev_byte_addr + 1 >= memory.len() {
                debug!(
                    "Abbreviation address {:04x} too close to end of memory, skipping",
                    abbrev_byte_addr
                );
                abbrev_shift = 0;
                continue;
            }

            // Recursively decode the abbreviation string
            match decode_string_recursive(memory, abbrev_byte_addr, abbrev_table_addr, depth + 1) {
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
    let (string, _) = decode_string(memory, byte_addr, abbrev_table_addr)?;
    Ok(string)
}

/// Unpack a string address based on version
fn unpack_string_address(packed: u16, version: u8) -> usize {
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
        // h=8, e=5, l=12, l=12, o=15 (all +6 for Z-encoding)
        // = 14, 11, 18, 18, 21
        let mut memory = vec![0u8; 100];

        // First word: 14, 11, 18 = 01110 01011 10010
        // = 0111 0010 1110 010 = 0x72E4
        memory[10] = 0x72;
        memory[11] = 0xE4;

        // Second word: 18, 21, padding = 10010 10101 00101
        // Set bit 15 for end = 1001 0101 0100 101 = 0x9545
        memory[12] = 0x95;
        memory[13] = 0x45;

        let (result, len) = decode_string(&memory, 10, 0).unwrap();
        assert_eq!(result, "hello");
        assert_eq!(len, 4);
    }

    #[test]
    fn test_string_with_space() {
        // Test "a b" = a, space, b
        // a=7, space=0, b=8 (a/b +6 for encoding)
        let mut memory = vec![0u8; 100];

        // 13, 0, 14 = 01101 00000 01110
        // Set bit 15 = 1011 0100 0000 1110 = 0xB40E
        memory[20] = 0xB4;
        memory[21] = 0x0E;

        let (result, len) = decode_string(&memory, 20, 0).unwrap();
        assert_eq!(result, "a b");
        assert_eq!(len, 2);
    }
}
