#[cfg(test)]
mod tests {
    use crate::test_utils::MockZMachine;

    #[test]
    fn test_dictionary_lookup_basic() {
        let mut zmachine = MockZMachine::with_memory_size(2048);
        zmachine.setup_version_3();
        
        // Set up a simple dictionary at address 0x80
        let dict_addr = 0x80;
        zmachine.memory[8] = 0x00; zmachine.memory[9] = dict_addr; // Dictionary pointer in header
        
        // Dictionary structure:
        // - Number of separator characters (1 byte)
        // - Separator characters (variable)
        // - Entry length (1 byte)
        // - Number of entries (2 bytes)
        // - Entries (variable)
        
        let mut pos = dict_addr as usize;
        
        // No separator characters
        zmachine.memory[pos] = 0;
        pos += 1;
        
        // Entry length: 4 bytes for v1-3
        zmachine.memory[pos] = 4;
        pos += 1;
        
        // Number of entries: 2
        zmachine.memory[pos] = 0;
        zmachine.memory[pos + 1] = 2;
        pos += 2;
        
        // Entry 1: "look" encoded
        // l=17, o=20, o=20, k=16 (Z-chars: 'a'=6, 'k'=16, 'l'=17, 'o'=20)
        let word1 = (17 << 10) | (20 << 5) | 20;  // "loo"
        let word2 = (16 << 10) | 0x8000;          // "k" with end bit
        
        zmachine.memory[pos] = (word1 >> 8) as u8;
        zmachine.memory[pos + 1] = (word1 & 0xFF) as u8;
        zmachine.memory[pos + 2] = (word2 >> 8) as u8;
        zmachine.memory[pos + 3] = (word2 & 0xFF) as u8;
        pos += 4;
        
        // Entry 2: "take" encoded
        // t=25, a=6, k=16, e=10 (Z-chars: 'a'=6, 'e'=10, 'k'=16, 't'=25)
        let word3 = (25 << 10) | (6 << 5) | 16;   // "tak"
        let word4 = (10 << 10) | 0x8000;           // "e" with end bit
        
        zmachine.memory[pos] = (word3 >> 8) as u8;
        zmachine.memory[pos + 1] = (word3 & 0xFF) as u8;
        zmachine.memory[pos + 2] = (word4 >> 8) as u8;
        zmachine.memory[pos + 3] = (word4 & 0xFF) as u8;
        
        // Test lookup
        let result = zmachine.lookup_word_in_dictionary("look");
        assert!(result.is_ok());
        let addr = result.unwrap();
        assert_ne!(addr, 0); // Should find the word
        
        // Test not found
        let result = zmachine.lookup_word_in_dictionary("xyz");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // Should not find the word
    }
    
    #[test]
    fn test_dictionary_lookup_case_insensitive() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        zmachine.setup_version_3();
        
        // Set up simple dictionary with "hello"
        let dict_addr = 0x80;
        zmachine.memory[8] = 0x00; zmachine.memory[9] = dict_addr;
        
        let mut pos = dict_addr as usize;
        zmachine.memory[pos] = 0; pos += 1;     // No separators
        zmachine.memory[pos] = 4; pos += 1;     // Entry length
        zmachine.memory[pos] = 0; pos += 1;     // Number of entries (high)
        zmachine.memory[pos] = 1; pos += 1;     // Number of entries (low)
        
        // Encode "hello": h=13, e=10, l=17, l=17, o=20 (Z-chars: 'a'=6, 'h'=13, 'e'=10, etc.)
        let word1 = (13 << 10) | (10 << 5) | 17;   // "hel"
        let word2 = (17 << 10) | (20 << 5) | 0x8000; // "lo" with end bit
        
        zmachine.memory[pos] = (word1 >> 8) as u8;
        zmachine.memory[pos + 1] = (word1 & 0xFF) as u8;
        zmachine.memory[pos + 2] = (word2 >> 8) as u8;
        zmachine.memory[pos + 3] = (word2 & 0xFF) as u8;
        
        // Test different cases
        let result1 = zmachine.lookup_word_in_dictionary("hello");
        let result2 = zmachine.lookup_word_in_dictionary("HELLO");
        let result3 = zmachine.lookup_word_in_dictionary("Hello");
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert!(result3.is_ok());
        
        let addr1 = result1.unwrap();
        let addr2 = result2.unwrap();
        let addr3 = result3.unwrap();
        
        assert_ne!(addr1, 0);
        assert_ne!(addr2, 0);
        assert_ne!(addr3, 0);
        
        // All should return the same address
        assert_eq!(addr1, addr2);
        assert_eq!(addr2, addr3);
    }
    
    #[test]
    fn test_dictionary_lookup_truncation() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        zmachine.setup_version_3();
        
        // Set up dictionary
        let dict_addr = 0x80;
        zmachine.memory[8] = 0x00; zmachine.memory[9] = dict_addr;
        
        let mut pos = dict_addr as usize;
        zmachine.memory[pos] = 0; pos += 1;     // No separators
        zmachine.memory[pos] = 4; pos += 1;     // Entry length
        zmachine.memory[pos] = 0; pos += 1;     // Number of entries (high)
        zmachine.memory[pos] = 1; pos += 1;     // Number of entries (low)
        
        // Encode "invent" (6 chars): i=14, n=19, v=27, e=10, n=19, t=25 (Z-chars: 'a'=6, 'i'=14, etc.)
        let word1 = (14 << 10) | (19 << 5) | 27;  // "inv"
        let word2 = (10 << 10) | (19 << 5) | 25 | 0x8000; // "ent" with end bit
        
        zmachine.memory[pos] = (word1 >> 8) as u8;
        zmachine.memory[pos + 1] = (word1 & 0xFF) as u8;
        zmachine.memory[pos + 2] = (word2 >> 8) as u8;
        zmachine.memory[pos + 3] = (word2 & 0xFF) as u8;
        
        // Test that longer words are truncated to 6 characters
        let result1 = zmachine.lookup_word_in_dictionary("invent");
        let result2 = zmachine.lookup_word_in_dictionary("inventory"); // Should match "invent"
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        let addr1 = result1.unwrap();
        let addr2 = result2.unwrap();
        
        assert_ne!(addr1, 0);
        assert_ne!(addr2, 0);
        assert_eq!(addr1, addr2);
    }
    
    #[test]
    fn test_decode_dictionary_entry() {
        let zmachine = MockZMachine::new();
        
        // Test decoding "hello"
        // h=13, e=10, l=17, l=17, o=20 (Z-chars: 'a'=6, 'h'=13, 'e'=10, etc.)
        let word1 = (13 << 10) | (10 << 5) | 17;   // "hel"
        let word2 = (17 << 10) | (20 << 5) | 0x8000; // "lo" with end bit
        
        let encoded = [
            (word1 >> 8) as u8,
            (word1 & 0xFF) as u8,
            (word2 >> 8) as u8,
            (word2 & 0xFF) as u8,
        ];
        
        let result = zmachine.decode_dictionary_entry(&encoded);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello");
    }
    
    #[test]
    fn test_decode_dictionary_entry_with_spaces() {
        let zmachine = MockZMachine::new();
        
        // Test decoding "go" (padded with spaces)
        // g=12, o=20, space=0 (Z-chars: 'a'=6, 'g'=12, 'o'=20)
        let word1 = (12 << 10) | (20 << 5) | 0 | 0x8000; // "go " with end bit
        
        let encoded = [
            (word1 >> 8) as u8,
            (word1 & 0xFF) as u8,
            0,
            0,
        ];
        
        let result = zmachine.decode_dictionary_entry(&encoded);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "go"); // Should trim trailing spaces
    }
    
    #[test]
    fn test_tokenise_with_dictionary_lookup() {
        let mut zmachine = MockZMachine::with_memory_size(2048);
        zmachine.setup_version_3();
        
        // Set up dictionary with "look" and "table"
        let dict_addr = 0x80;
        zmachine.memory[8] = 0x00; zmachine.memory[9] = dict_addr;
        
        let mut pos = dict_addr as usize;
        zmachine.memory[pos] = 0; pos += 1;     // No separators
        zmachine.memory[pos] = 4; pos += 1;     // Entry length
        zmachine.memory[pos] = 0; pos += 1;     // Number of entries (high)
        zmachine.memory[pos] = 2; pos += 1;     // Number of entries (low)
        
        // Entry 1: "look" - l=17, o=20, o=20, k=16 (Z-chars: 'a'=6, 'k'=16, 'l'=17, 'o'=20)
        let word1 = (17 << 10) | (20 << 5) | 20;  // "loo"
        let word2 = (16 << 10) | 0x8000;          // "k" with end bit
        
        zmachine.memory[pos] = (word1 >> 8) as u8;
        zmachine.memory[pos + 1] = (word1 & 0xFF) as u8;
        zmachine.memory[pos + 2] = (word2 >> 8) as u8;
        zmachine.memory[pos + 3] = (word2 & 0xFF) as u8;
        pos += 4;
        
        // Entry 2: "table" - t=25, a=6, b=7, l=17, e=10 (Z-chars: 'a'=6, 'b'=7, 'e'=10, 'l'=17, 't'=25)
        let word3 = (25 << 10) | (6 << 5) | 7;    // "tab"
        let word4 = (17 << 10) | (10 << 5) | 0x8000; // "le" with end bit
        
        zmachine.memory[pos] = (word3 >> 8) as u8;
        zmachine.memory[pos + 1] = (word3 & 0xFF) as u8;
        zmachine.memory[pos + 2] = (word4 >> 8) as u8;
        zmachine.memory[pos + 3] = (word4 & 0xFF) as u8;
        
        // Set up text buffer with "look at table"
        let text_buffer = 0x200;
        zmachine.memory[text_buffer] = 20;      // Max length
        zmachine.memory[text_buffer + 1] = 13;  // Actual length
        let text = "look at table";
        for (i, ch) in text.bytes().enumerate() {
            zmachine.memory[text_buffer + 2 + i] = ch;
        }
        
        // Set up parse buffer
        let parse_buffer = 0x250;
        zmachine.memory[parse_buffer] = 10;     // Max words
        
        // Test TOKENISE
        zmachine.operands_buffer = vec![text_buffer as u16, parse_buffer as u16];
        let result = zmachine.op_tokenise();
        assert!(result.is_ok());
        
        // Check results
        let num_words = zmachine.memory[parse_buffer + 1];
        assert_eq!(num_words, 3); // "look", "at", "table"
        
        // Check first word "look"
        let word1_len = zmachine.memory[parse_buffer + 2];
        let word1_pos = zmachine.memory[parse_buffer + 3];
        let word1_dict_high = zmachine.memory[parse_buffer + 4];
        let word1_dict_low = zmachine.memory[parse_buffer + 5];
        
        assert_eq!(word1_len, 4);   // "look" length
        assert_eq!(word1_pos, 2);   // Position in buffer
        
        // Dictionary address should be non-zero for "look"
        let dict_addr_found = ((word1_dict_high as u16) << 8) | (word1_dict_low as u16);
        assert_ne!(dict_addr_found, 0);
        
        // Check third word "table"
        let word3_dict_high = zmachine.memory[parse_buffer + 10];
        let word3_dict_low = zmachine.memory[parse_buffer + 11];
        let dict_addr_table = ((word3_dict_high as u16) << 8) | (word3_dict_low as u16);
        assert_ne!(dict_addr_table, 0);
        
        // "at" should not be found (returns 0)
        let word2_dict_high = zmachine.memory[parse_buffer + 8];
        let word2_dict_low = zmachine.memory[parse_buffer + 9];
        let dict_addr_at = ((word2_dict_high as u16) << 8) | (word2_dict_low as u16);
        
        assert_eq!(dict_addr_at, 0);
    }
}