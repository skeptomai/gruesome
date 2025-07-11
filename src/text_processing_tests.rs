#[cfg(test)]
mod tests {
    use crate::test_utils::MockZMachine;

    #[test]
    fn test_tokenise_basic_parsing() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        // Set up text buffer at 0x100
        let text_buffer = 0x100;
        let parse_buffer = 0x200;
        
        // Set up input text "go north"
        let input_text = "go north";
        zmachine.memory[text_buffer] = 20;  // max length
        zmachine.memory[text_buffer + 1] = input_text.len() as u8;  // actual length
        
        // Store the input text
        for (i, ch) in input_text.chars().enumerate() {
            zmachine.memory[text_buffer + 2 + i] = ch as u8;
        }
        
        // Set up parse buffer (max 5 words)
        zmachine.memory[parse_buffer] = 5;
        
        // Set up operands for TOKENISE
        zmachine.operands_buffer = vec![text_buffer as u16, parse_buffer as u16];
        
        // Call TOKENISE
        let result = zmachine.op_tokenise();
        assert!(result.is_ok());
        
        // Check results
        assert_eq!(zmachine.memory[parse_buffer + 1], 2); // 2 words found
        
        // Check first word "go"
        assert_eq!(zmachine.memory[parse_buffer + 2], 2);  // length
        assert_eq!(zmachine.memory[parse_buffer + 3], 2);  // position
        assert_eq!(zmachine.memory[parse_buffer + 4], 0);  // dict addr high (not found)
        assert_eq!(zmachine.memory[parse_buffer + 5], 0);  // dict addr low (not found)
        
        // Check second word "north"
        assert_eq!(zmachine.memory[parse_buffer + 6], 5);  // length
        assert_eq!(zmachine.memory[parse_buffer + 7], 5);  // position
        assert_eq!(zmachine.memory[parse_buffer + 8], 0);  // dict addr high (not found)
        assert_eq!(zmachine.memory[parse_buffer + 9], 0);  // dict addr low (not found)
    }

    #[test]
    fn test_tokenise_empty_input() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let text_buffer = 0x100;
        let parse_buffer = 0x200;
        
        // Set up empty input
        zmachine.memory[text_buffer] = 20;  // max length
        zmachine.memory[text_buffer + 1] = 0;  // actual length (empty)
        
        zmachine.memory[parse_buffer] = 5;  // max words
        
        zmachine.operands_buffer = vec![text_buffer as u16, parse_buffer as u16];
        
        let result = zmachine.op_tokenise();
        assert!(result.is_ok());
        
        // Should find 0 words
        assert_eq!(zmachine.memory[parse_buffer + 1], 0);
    }

    #[test]
    fn test_tokenise_multiple_spaces() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let text_buffer = 0x100;
        let parse_buffer = 0x200;
        
        // Set up input with multiple spaces "  take   lamp  "
        let input_text = "  take   lamp  ";
        zmachine.memory[text_buffer] = 20;
        zmachine.memory[text_buffer + 1] = input_text.len() as u8;
        
        for (i, ch) in input_text.chars().enumerate() {
            zmachine.memory[text_buffer + 2 + i] = ch as u8;
        }
        
        zmachine.memory[parse_buffer] = 5;
        zmachine.operands_buffer = vec![text_buffer as u16, parse_buffer as u16];
        
        let result = zmachine.op_tokenise();
        assert!(result.is_ok());
        
        // Should find 2 words despite multiple spaces
        assert_eq!(zmachine.memory[parse_buffer + 1], 2);
        
        // Check word lengths
        assert_eq!(zmachine.memory[parse_buffer + 2], 4);  // "take"
        assert_eq!(zmachine.memory[parse_buffer + 6], 4);  // "lamp"
    }

    #[test]
    fn test_tokenise_max_words_limit() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let text_buffer = 0x100;
        let parse_buffer = 0x200;
        
        // Set up input with many words
        let input_text = "one two three four five six";
        zmachine.memory[text_buffer] = 30;
        zmachine.memory[text_buffer + 1] = input_text.len() as u8;
        
        for (i, ch) in input_text.chars().enumerate() {
            zmachine.memory[text_buffer + 2 + i] = ch as u8;
        }
        
        // Limit to 3 words max
        zmachine.memory[parse_buffer] = 3;
        zmachine.operands_buffer = vec![text_buffer as u16, parse_buffer as u16];
        
        let result = zmachine.op_tokenise();
        assert!(result.is_ok());
        
        // Should only store 3 words due to limit
        assert_eq!(zmachine.memory[parse_buffer + 1], 3);
    }

    #[test]
    fn test_tokenise_missing_operands() {
        let mut zmachine = MockZMachine::new();
        
        // Test with insufficient operands
        zmachine.operands_buffer = vec![0x100]; // Only 1 operand, need 2
        
        let result = zmachine.op_tokenise();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing operands"));
    }

    #[test]
    fn test_encode_text_basic() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let text_buffer = 0x100;
        let coded_text_addr = 0x200;
        
        // Set up text to encode "hello"
        let text = "hello";
        for (i, ch) in text.chars().enumerate() {
            zmachine.memory[text_buffer + i] = ch as u8;
        }
        
        // Set up operands for ENCODE_TEXT
        zmachine.operands_buffer = vec![
            text_buffer as u16,    // text buffer
            text.len() as u16,     // length
            0,                     // start position
            coded_text_addr as u16 // coded text address
        ];
        
        let result = zmachine.op_encode_text();
        assert!(result.is_ok());
        
        // Check that something was written to the coded text address
        // (The mock implementation stores 0x80 and length)
        assert_eq!(zmachine.memory[coded_text_addr], 0x80);
        assert_eq!(zmachine.memory[coded_text_addr + 1], text.len() as u8);
    }

    #[test]
    fn test_encode_text_with_offset() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let text_buffer = 0x100;
        let coded_text_addr = 0x200;
        
        // Set up text "hello world" but encode only "world" (offset 6, length 5)
        let full_text = "hello world";
        for (i, ch) in full_text.chars().enumerate() {
            zmachine.memory[text_buffer + i] = ch as u8;
        }
        
        zmachine.operands_buffer = vec![
            text_buffer as u16,    // text buffer
            5,                     // length ("world")
            6,                     // start position (after "hello ")
            coded_text_addr as u16 // coded text address
        ];
        
        let result = zmachine.op_encode_text();
        assert!(result.is_ok());
        
        // Check encoding result
        assert_eq!(zmachine.memory[coded_text_addr], 0x80);
        assert_eq!(zmachine.memory[coded_text_addr + 1], 5); // length of "world"
    }

    #[test]
    fn test_encode_text_missing_operands() {
        let mut zmachine = MockZMachine::new();
        
        // Test with insufficient operands
        zmachine.operands_buffer = vec![0x100, 5, 0]; // Only 3 operands, need 4
        
        let result = zmachine.op_encode_text();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing operands"));
    }

    #[test]
    fn test_encode_text_boundary_conditions() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let text_buffer = 0x100;
        let coded_text_addr = 0x200;
        
        // Test with zero length
        zmachine.operands_buffer = vec![
            text_buffer as u16,
            0,  // zero length
            0,
            coded_text_addr as u16
        ];
        
        let result = zmachine.op_encode_text();
        assert!(result.is_ok());
        
        // Should handle zero length gracefully
        assert_eq!(zmachine.memory[coded_text_addr], 0x80);
        assert_eq!(zmachine.memory[coded_text_addr + 1], 0);
    }

    #[test]
    fn test_tokenise_and_encode_text_integration() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let text_buffer = 0x100;
        let parse_buffer = 0x200;
        let coded_text_addr = 0x300;
        
        // Set up input text
        let input_text = "take key";
        zmachine.memory[text_buffer] = 20;
        zmachine.memory[text_buffer + 1] = input_text.len() as u8;
        
        for (i, ch) in input_text.chars().enumerate() {
            zmachine.memory[text_buffer + 2 + i] = ch as u8;
        }
        
        zmachine.memory[parse_buffer] = 5;
        
        // First tokenise the input
        zmachine.operands_buffer = vec![text_buffer as u16, parse_buffer as u16];
        let result = zmachine.op_tokenise();
        assert!(result.is_ok());
        
        // Verify tokenisation worked
        assert_eq!(zmachine.memory[parse_buffer + 1], 2); // 2 words
        
        // Now encode the first word "take"
        zmachine.operands_buffer = vec![
            text_buffer as u16,
            4,  // length of "take"
            2,  // position of "take" in buffer
            coded_text_addr as u16
        ];
        
        let result = zmachine.op_encode_text();
        assert!(result.is_ok());
        
        // Verify encoding worked
        assert_eq!(zmachine.memory[coded_text_addr], 0x80);
        assert_eq!(zmachine.memory[coded_text_addr + 1], 4);
    }

    #[test]
    fn test_text_processing_buffer_bounds() {
        let mut zmachine = MockZMachine::with_memory_size(512); // Smaller memory
        
        // Test TOKENISE with out-of-bounds addresses
        zmachine.operands_buffer = vec![600, 700]; // Beyond memory size
        let result = zmachine.op_tokenise();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
        
        // Test ENCODE_TEXT with out-of-bounds addresses
        zmachine.operands_buffer = vec![600, 5, 0, 700];
        let result = zmachine.op_encode_text();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
    }
}