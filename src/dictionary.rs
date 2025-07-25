/// Dictionary lookup for Z-Machine with version-aware encoding
///
/// The Z-Machine dictionary format differs significantly between versions:
/// - **v3**: 6 Z-characters encoded in 4 bytes (2 words)
/// - **v4+**: 9 Z-characters encoded in 6 bytes (3 words)
///
/// This module provides automatic version detection and appropriate encoding
/// for dictionary lookups. The encoding uses 5-bit Z-characters packed into
/// words, with character 5 representing space and characters 6-31 representing
/// letters a-z.
///
/// ## Binary Search:
/// Dictionary entries are stored in sorted order, allowing efficient binary
/// search lookup. For v4+ games, lexicographic comparison across all 3 words
/// is used to maintain proper ordering.
use crate::vm::VM;
use log::debug;

/// Tokenize text using dictionary separators, not just whitespace
/// Returns a vector of (word, position_in_original_text) tuples
fn tokenize_with_separators(input: &str, separators: &[char]) -> Vec<(String, usize)> {
    let mut tokens = Vec::new();
    let mut current_word = String::new();
    let mut word_start_pos = 0;
    let mut pos = 0;

    for ch in input.chars() {
        if ch.is_whitespace() {
            // Finish current word if any
            if !current_word.is_empty() {
                tokens.push((current_word.clone(), word_start_pos));
                current_word.clear();
            }
            // Skip whitespace - next non-whitespace will start a new word
            pos += 1;
        } else if separators.contains(&ch) {
            // Finish current word if any
            if !current_word.is_empty() {
                tokens.push((current_word.clone(), word_start_pos));
                current_word.clear();
            }
            // The separator itself becomes a token
            tokens.push((ch.to_string(), pos));
            pos += 1;
        } else {
            // Regular character - start new word if needed
            if current_word.is_empty() {
                word_start_pos = pos;
            }
            current_word.push(ch);
            pos += 1;
        }
    }

    // Add final word if any
    if !current_word.is_empty() {
        tokens.push((current_word, word_start_pos));
    }

    tokens
}

/// Encode a word for dictionary lookup (V3) - 6 Z-characters in 4 bytes
fn encode_word_v3(word: &str) -> (u16, u16) {
    let mut chars = Vec::new();

    // Convert to lowercase and take first 6 chars
    for ch in word.chars().take(6) {
        let ch = ch.to_ascii_lowercase();
        let code = match ch {
            'a'..='z' => ch as u8 - b'a' + 6,
            ' ' => 5,
            _ => 5, // Default to space for unknown chars
        };
        chars.push(code);
    }

    // Pad with 5s (spaces) if needed
    while chars.len() < 6 {
        chars.push(5);
    }

    // Pack into two 16-bit words
    let word1 = ((chars[0] as u16) << 10) | ((chars[1] as u16) << 5) | (chars[2] as u16);
    let word2 = ((chars[3] as u16) << 10) | ((chars[4] as u16) << 5) | (chars[5] as u16);

    // Set the end-of-string bit on word2
    let word2 = word2 | 0x8000;

    (word1, word2)
}

/// Encode a word for dictionary lookup (V4+) - 9 Z-characters in 6 bytes  
fn encode_word_v4_plus(word: &str) -> (u16, u16, u16) {
    let mut chars = Vec::new();

    // Convert to lowercase and take first 9 chars
    for ch in word.chars().take(9) {
        let ch = ch.to_ascii_lowercase();
        let code = match ch {
            'a'..='z' => ch as u8 - b'a' + 6,
            ' ' => 5,
            _ => 5, // Default to space for unknown chars
        };
        chars.push(code);
    }

    // Pad with 5s (spaces) if needed to reach 9 characters
    while chars.len() < 9 {
        chars.push(5);
    }

    // Pack all 9 characters into three 16-bit words
    let word1 = ((chars[0] as u16) << 10) | ((chars[1] as u16) << 5) | (chars[2] as u16);
    let word2 = ((chars[3] as u16) << 10) | ((chars[4] as u16) << 5) | (chars[5] as u16);
    let word3 = ((chars[6] as u16) << 10) | ((chars[7] as u16) << 5) | (chars[8] as u16);

    // Set the end-of-string bit on word3 (last word)
    let word3 = word3 | 0x8000;

    (word1, word2, word3)
}

impl VM {
    /// Look up a word in the dictionary (version-aware)
    pub fn lookup_dictionary(&self, word: &str) -> u16 {
        if self.game.header.version <= 3 {
            self.lookup_dictionary_v3(word)
        } else {
            self.lookup_dictionary_v4_plus(word)
        }
    }

    /// Look up a word in the v3 dictionary
    fn lookup_dictionary_v3(&self, word: &str) -> u16 {
        let dict_addr = self.game.header.dictionary as u32;

        // Read dictionary header
        let sep_count = self.read_byte(dict_addr);
        let entry_start = dict_addr + 1 + sep_count as u32;
        let entry_length = self.read_byte(entry_start);
        let entry_count = self.read_word(entry_start + 1);

        debug!(
            "V3 dictionary lookup for '{}': {} entries, length {} bytes",
            word, entry_count, entry_length
        );

        // Encode the search word (v3: 6 Z-characters in 2 words)
        let (search_word1, search_word2) = encode_word_v3(word);
        debug!(
            "V3 encoded '{}' as: {:04x} {:04x}",
            word, search_word1, search_word2
        );

        // Dictionary entries start here
        let entries_addr = entry_start + 3;

        // Binary search (dictionary is sorted)
        let mut low = 0i32;
        let mut high = entry_count as i32 - 1;

        while low <= high {
            let mid = (low + high) / 2;
            let addr = entries_addr + (mid as u32 * entry_length as u32);

            // Read dictionary entry (v3: 2 words = 4 bytes)
            let dict_word1 = self.read_word(addr);
            let dict_word2 = self.read_word(addr + 2);

            // Compare as 32-bit values
            if search_word1 < dict_word1
                || (search_word1 == dict_word1 && search_word2 < dict_word2)
            {
                high = mid - 1;
            } else if search_word1 > dict_word1
                || (search_word1 == dict_word1 && search_word2 > dict_word2)
            {
                low = mid + 1;
            } else {
                // Found!
                debug!("V3 dictionary found '{}' at {:04x}", word, addr);

                // Check the dictionary entry type
                let type_byte = self.read_byte(addr + 4);
                let byte5 = self.read_byte(addr + 5);
                let byte6 = self.read_byte(addr + 6);

                debug!(
                    "  Dictionary entry: type={:02x}, data={:02x} {:02x}",
                    type_byte, byte5, byte6
                );

                if type_byte == 0x32 {
                    debug!(
                        "  *** WARNING: Type 0x32 dictionary entry - special handling needed! ***"
                    );
                }

                return addr as u16;
            }
        }

        // Not found
        debug!("V3 dictionary: '{}' not found", word);
        0
    }

    /// Look up a word in the v4+ dictionary
    fn lookup_dictionary_v4_plus(&self, word: &str) -> u16 {
        let dict_addr = self.game.header.dictionary as u32;

        // Read dictionary header
        let sep_count = self.read_byte(dict_addr);
        let entry_start = dict_addr + 1 + sep_count as u32;
        let entry_length = self.read_byte(entry_start);
        let entry_count = self.read_word(entry_start + 1);

        debug!(
            "V4+ dictionary lookup for '{}': {} entries, length {} bytes",
            word, entry_count, entry_length
        );

        // Encode the search word (v4+: 9 Z-characters in 3 words)
        let (search_word1, search_word2, search_word3) = encode_word_v4_plus(word);
        debug!(
            "V4+ encoded '{}' as: {:04x} {:04x} {:04x}",
            word, search_word1, search_word2, search_word3
        );

        // Dictionary entries start here
        let entries_addr = entry_start + 3;

        // Binary search (dictionary is sorted)
        let mut low = 0i32;
        let mut high = entry_count as i32 - 1;

        while low <= high {
            let mid = (low + high) / 2;
            let addr = entries_addr + (mid as u32 * entry_length as u32);

            // Read dictionary entry (v4+: 3 words = 6 bytes)
            let dict_word1 = self.read_word(addr);
            let dict_word2 = self.read_word(addr + 2);
            let dict_word3 = self.read_word(addr + 4);

            // Compare as 48-bit values (lexicographic order)
            let cmp = if search_word1 != dict_word1 {
                search_word1.cmp(&dict_word1)
            } else if search_word2 != dict_word2 {
                search_word2.cmp(&dict_word2)
            } else {
                search_word3.cmp(&dict_word3)
            };

            match cmp {
                std::cmp::Ordering::Less => high = mid - 1,
                std::cmp::Ordering::Greater => low = mid + 1,
                std::cmp::Ordering::Equal => {
                    // Found!
                    debug!("V4+ dictionary found '{}' at {:04x}", word, addr);

                    // Check the dictionary entry type
                    let type_byte = self.read_byte(addr + 6);
                    let byte7 = self.read_byte(addr + 7);
                    let byte8 = self.read_byte(addr + 8);

                    debug!(
                        "  Dictionary entry: type={:02x}, data={:02x} {:02x}",
                        type_byte, byte7, byte8
                    );

                    if type_byte == 0x32 {
                        debug!(
                            "  *** WARNING: Type 0x32 dictionary entry - special handling needed! ***"
                        );
                    }

                    return addr as u16;
                }
            }
        }

        // Not found
        debug!("V4+ dictionary: '{}' not found", word);
        0
    }

    /// Parse text buffer into parse buffer
    pub fn parse_text(&mut self, text_buffer: u32, parse_buffer: u32) -> Result<(), String> {
        // Read dictionary separators from dictionary header
        let dict_addr = self.game.header.dictionary as u32;
        let sep_count = self.read_byte(dict_addr);
        let mut separators = Vec::new();
        for i in 0..sep_count {
            separators.push(self.read_byte(dict_addr + 1 + i as u32) as char);
        }
        debug!("Dictionary separators: {:?}", separators);

        // Read text from text buffer
        let text_len = self.read_byte(text_buffer + 1) as usize;
        let mut text = String::new();
        for i in 0..text_len {
            let ch = self.read_byte(text_buffer + 2 + i as u32);
            text.push(ch as char);
        }
        debug!("Input text to parse: '{}'", text);

        // Get max words from parse buffer
        let max_words = self.read_byte(parse_buffer);

        // Tokenize using dictionary separators, not just whitespace
        let word_tokens = tokenize_with_separators(&text, &separators);
        debug!("Tokenized words: {:?}", word_tokens);
        let word_count = word_tokens.len().min(max_words as usize);

        // Write word count
        self.write_byte(parse_buffer + 1, word_count as u8)?;

        // For each word, write parse data
        for (i, (word, position)) in word_tokens.iter().take(word_count).enumerate() {
            // Look up word in dictionary
            let dict_addr = self.lookup_dictionary(word);

            // Log what we're storing in the parse buffer
            debug!(
                "Storing word '{}' at position {} in parse buffer: dict_addr={:04x}",
                word, position, dict_addr
            );

            // Write parse entry (V3 format)
            let entry_offset = parse_buffer + 2 + (i * 4) as u32;
            self.write_word(entry_offset, dict_addr)?; // Dictionary address

            // Special debug for leaves
            if word == "leaves" {
                debug!(
                    "Writing parse entry for 'leaves': len={}, pos={}",
                    word.len(),
                    position + 2
                );
            }

            self.write_byte(entry_offset + 2, word.len() as u8)?; // Word length
                                                                  // The position is the byte offset in the text buffer where the word starts
                                                                  // position is 0-based in the text string
                                                                  // The text starts at buffer+2, so buffer offset = position + 2
            self.write_byte(entry_offset + 3, (position + 2) as u8)?; // Byte offset in buffer
        }

        Ok(())
    }
}
