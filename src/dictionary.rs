/// Dictionary lookup for Z-Machine V3
use crate::vm::VM;

/// Encode a word for dictionary lookup (V3)
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

impl VM {
    /// Look up a word in the dictionary
    pub fn lookup_dictionary(&self, word: &str) -> u16 {
        let dict_addr = self.game.header.dictionary as u32;
        
        // Read dictionary header
        let sep_count = self.read_byte(dict_addr);
        
        // Skip separators
        let entry_start = dict_addr + 1 + sep_count as u32;
        let entry_length = self.read_byte(entry_start);
        let entry_count = self.read_word(entry_start + 1);
        
        // Encode the word
        let (search_word1, search_word2) = encode_word_v3(word);
        
        // Dictionary entries start here
        let entries_addr = entry_start + 3;
        
        // Binary search (dictionary is sorted)
        let mut low = 0i32;
        let mut high = entry_count as i32 - 1;
        
        while low <= high {
            let mid = (low + high) / 2;
            let addr = entries_addr + (mid as u32 * entry_length as u32);
            
            // Read dictionary entry
            let dict_word1 = self.read_word(addr);
            let dict_word2 = self.read_word(addr + 2);
            
            // Compare
            if search_word1 < dict_word1 || (search_word1 == dict_word1 && search_word2 < dict_word2) {
                high = mid - 1;
            } else if search_word1 > dict_word1 || (search_word1 == dict_word1 && search_word2 > dict_word2) {
                low = mid + 1;
            } else {
                // Found\!
                return addr as u16;
            }
        }
        
        // Not found
        0
    }
    
    /// Parse text buffer into parse buffer
    pub fn parse_text(&mut self, text_buffer: u32, parse_buffer: u32) -> Result<(), String> {
        // Read text from text buffer
        let text_len = self.read_byte(text_buffer + 1) as usize;
        let mut text = String::new();
        for i in 0..text_len {
            let ch = self.read_byte(text_buffer + 2 + i as u32);
            text.push(ch as char);
        }
        
        // Get max words from parse buffer
        let max_words = self.read_byte(parse_buffer);
        
        // Split into words (simple whitespace split)
        let words: Vec<&str> = text.split_whitespace().collect();
        let word_count = words.len().min(max_words as usize);
        
        // Write word count
        self.write_byte(parse_buffer + 1, word_count as u8)?;
        
        // For each word, write parse data
        let mut text_offset = 0;
        for (i, word) in words.iter().take(word_count).enumerate() {
            // Find word position in original text
            while text_offset < text.len() && text.chars().nth(text_offset).unwrap().is_whitespace() {
                text_offset += 1;
            }
            
            // Look up word in dictionary
            let dict_addr = self.lookup_dictionary(word);
            
            // Write parse entry (V3 format)
            let entry_offset = parse_buffer + 2 + (i * 4) as u32;
            self.write_word(entry_offset, dict_addr)?;                    // Dictionary address
            self.write_byte(entry_offset + 2, word.len() as u8)?;        // Word length
            self.write_byte(entry_offset + 3, (text_offset + 1) as u8)?; // Position (1-based)
            
            // Advance text offset
            text_offset += word.len();
        }
        
        Ok(())
    }
}
