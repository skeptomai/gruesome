/// Clean dictionary lookup implementations for Z-Machine v3 and v4+
use crate::vm::VM;
use log::debug;

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
    /// Look up a word in the dictionary (version-aware dispatcher)
    pub fn lookup_dictionary_clean(&self, word: &str) -> u16 {
        if self.game.header.version <= 3 {
            self.lookup_dictionary_v3_clean(word)
        } else {
            self.lookup_dictionary_v4_plus_clean(word)
        }
    }

    /// Look up a word in the v3 dictionary (simple, clean implementation)
    pub fn lookup_dictionary_v3_clean(&self, word: &str) -> u16 {
        let dict_addr = self.game.header.dictionary as u32;

        // Read dictionary header
        let sep_count = self.read_byte(dict_addr);
        let entry_start = dict_addr + 1 + sep_count as u32;
        let entry_length = self.read_byte(entry_start);
        let entry_count = self.read_word(entry_start + 1);

        debug!("V3 dictionary lookup for '{}': {} entries, length {} bytes", 
               word, entry_count, entry_length);

        // Encode the search word (v3: 6 Z-characters in 2 words)
        let (search_word1, search_word2) = encode_word_v3(word);
        debug!("V3 encoded '{}' as: {:04x} {:04x}", word, search_word1, search_word2);

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
            if search_word1 < dict_word1 || (search_word1 == dict_word1 && search_word2 < dict_word2) {
                high = mid - 1;
            } else if search_word1 > dict_word1 || (search_word1 == dict_word1 && search_word2 > dict_word2) {
                low = mid + 1;
            } else {
                // Found!
                debug!("V3 dictionary found '{}' at {:04x}", word, addr);
                return addr as u16;
            }
        }

        // Not found
        debug!("V3 dictionary: '{}' not found", word);
        0
    }

    /// Look up a word in the v4+ dictionary (simple, clean implementation)
    pub fn lookup_dictionary_v4_plus_clean(&self, word: &str) -> u16 {
        let dict_addr = self.game.header.dictionary as u32;

        // Read dictionary header
        let sep_count = self.read_byte(dict_addr);
        let entry_start = dict_addr + 1 + sep_count as u32;
        let entry_length = self.read_byte(entry_start);
        let entry_count = self.read_word(entry_start + 1);

        debug!("V4+ dictionary lookup for '{}': {} entries, length {} bytes", 
               word, entry_count, entry_length);

        // Encode the search word (v4+: 9 Z-characters in 3 words)
        let (search_word1, search_word2, search_word3) = encode_word_v4_plus(word);
        debug!("V4+ encoded '{}' as: {:04x} {:04x} {:04x}", 
               word, search_word1, search_word2, search_word3);

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
                    return addr as u16;
                }
            }
        }

        // Not found
        debug!("V4+ dictionary: '{}' not found", word);
        0
    }
}