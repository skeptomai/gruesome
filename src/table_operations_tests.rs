#[cfg(test)]
mod tests {
    use crate::test_utils::MockZMachine;

    #[test]
    fn test_copy_table_basic_copy() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        // Set up source data at 0x100
        let source_addr = 0x100;
        let dest_addr = 0x200;
        let test_data = b"Hello World!";
        
        for (i, &byte) in test_data.iter().enumerate() {
            zmachine.memory[source_addr + i] = byte;
        }
        
        // Set up operands for COPY_TABLE
        zmachine.operands_buffer = vec![
            source_addr as u16,     // source
            dest_addr as u16,       // destination  
            test_data.len() as u16  // size
        ];
        
        let result = zmachine.op_copy_table();
        assert!(result.is_ok());
        
        // Verify the data was copied correctly
        for (i, &expected_byte) in test_data.iter().enumerate() {
            assert_eq!(zmachine.memory[dest_addr + i], expected_byte);
        }
    }

    #[test]
    fn test_copy_table_zero_fill() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let dest_addr = 0x200;
        let fill_size = 10;
        
        // Fill destination with non-zero values first
        for i in 0..fill_size {
            zmachine.memory[dest_addr + i] = 0xFF;
        }
        
        // Set up operands for zero fill (size = 0)
        zmachine.operands_buffer = vec![
            fill_size as u16,  // number of bytes to zero (when size=0)
            dest_addr as u16,  // destination
            0                  // size = 0 means zero fill
        ];
        
        let result = zmachine.op_copy_table();
        assert!(result.is_ok());
        
        // Verify the memory was zeroed
        for i in 0..fill_size {
            assert_eq!(zmachine.memory[dest_addr + i], 0);
        }
    }

    #[test]
    fn test_copy_table_overlapping_regions() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let base_addr = 0x100;
        let test_data = b"ABCDEFGHIJ";
        
        // Set up overlapping source data
        for (i, &byte) in test_data.iter().enumerate() {
            zmachine.memory[base_addr + i] = byte;
        }
        
        // Copy with overlap (should handle correctly with temp buffer)
        zmachine.operands_buffer = vec![
            base_addr as u16,           // source
            (base_addr + 3) as u16,     // destination (overlapping)
            5                           // size
        ];
        
        let result = zmachine.op_copy_table();
        assert!(result.is_ok());
        
        // Verify the overlapping copy worked correctly
        // Original: ABCDEFGHIJ
        // After copying 5 bytes from pos 0 to pos 3: ABCABCDEHIJ
        assert_eq!(zmachine.memory[base_addr + 3], b'A');
        assert_eq!(zmachine.memory[base_addr + 4], b'B');
        assert_eq!(zmachine.memory[base_addr + 5], b'C');
        assert_eq!(zmachine.memory[base_addr + 6], b'D');
        assert_eq!(zmachine.memory[base_addr + 7], b'E');
    }

    #[test]
    fn test_copy_table_boundary_conditions() {
        let mut zmachine = MockZMachine::with_memory_size(512);
        
        // Test copy that would exceed memory bounds
        zmachine.operands_buffer = vec![
            100,    // source
            600,    // destination (out of bounds)
            10      // size
        ];
        
        let result = zmachine.op_copy_table();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
    }

    #[test]
    fn test_copy_table_missing_operands() {
        let mut zmachine = MockZMachine::new();
        
        // Test with insufficient operands
        zmachine.operands_buffer = vec![100, 200]; // Only 2 operands, need 3
        
        let result = zmachine.op_copy_table();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing operands"));
    }

    #[test]
    fn test_scan_table_found_word() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let table_addr = 0x200;
        
        // Set up a table with 2-byte entries: [0x1234, 0x5678, 0xABCD]
        zmachine.memory[table_addr] = 0x12;     // First entry high byte
        zmachine.memory[table_addr + 1] = 0x34; // First entry low byte
        zmachine.memory[table_addr + 2] = 0x56; // Second entry high byte
        zmachine.memory[table_addr + 3] = 0x78; // Second entry low byte
        zmachine.memory[table_addr + 4] = 0xAB; // Third entry high byte
        zmachine.memory[table_addr + 5] = 0xCD; // Third entry low byte
        
        // Search for 0x5678 (should find it at index 1)
        zmachine.operands_buffer = vec![
            0x5678,                 // search value
            table_addr as u16,      // table address
            3,                      // table length (3 entries)
            0x02                    // form (2-byte entries, compare full word)
        ];
        
        let result = zmachine.op_scan_table();
        assert!(result.is_ok());
        
        
        // Should return address of found entry
        assert_eq!(zmachine.stack.len(), 1);
        assert_eq!(zmachine.stack[zmachine.stack.len() - 1], (table_addr + 2) as u16); // Address of second entry
    }

    #[test]
    fn test_scan_table_not_found() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let table_addr = 0x200;
        
        // Set up a table with entries that don't match our search
        zmachine.memory[table_addr] = 0x12;
        zmachine.memory[table_addr + 1] = 0x34;
        zmachine.memory[table_addr + 2] = 0x56;
        zmachine.memory[table_addr + 3] = 0x78;
        
        // Search for 0x9999 (not in table)
        zmachine.operands_buffer = vec![
            0x9999,                 // search value (not found)
            table_addr as u16,      // table address
            2,                      // table length (2 entries)
            0x82                    // form (2-byte entries)
        ];
        
        let result = zmachine.op_scan_table();
        assert!(result.is_ok());
        
        // Should return 0 (not found)
        assert_eq!(zmachine.stack.len(), 1);
        assert_eq!(zmachine.stack[zmachine.stack.len() - 1], 0);
    }

    #[test]
    fn test_scan_table_byte_comparison() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let table_addr = 0x200;
        
        // Set up a table with byte entries
        zmachine.memory[table_addr] = 0x12;
        zmachine.memory[table_addr + 1] = 0x34;
        zmachine.memory[table_addr + 2] = 0x56;
        
        // Search for 0x1234 with byte-only comparison (should match 0x34)
        zmachine.operands_buffer = vec![
            0x1234,                 // search value (low byte = 0x34)
            table_addr as u16,      // table address
            3,                      // table length (3 entries)
            0x81                    // form (1-byte entries, compare byte only - bit 7 set)
        ];
        
        let result = zmachine.op_scan_table();
        assert!(result.is_ok());
        
        // Should find the entry with byte value 0x34
        assert_eq!(zmachine.stack.len(), 1);
        assert_eq!(zmachine.stack[zmachine.stack.len() - 1], (table_addr + 1) as u16);
    }

    #[test]
    fn test_scan_table_default_form() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let table_addr = 0x200;
        
        // Set up a table
        zmachine.memory[table_addr] = 0x12;
        zmachine.memory[table_addr + 1] = 0x34;
        
        // Search without specifying form (should default to 0x02)
        zmachine.operands_buffer = vec![
            0x1234,                 // search value
            table_addr as u16,      // table address
            1                       // table length (1 entry)
            // No form operand - should default to 0x02
        ];
        
        let result = zmachine.op_scan_table();
        assert!(result.is_ok());
        
        // Should find the entry
        assert_eq!(zmachine.stack.len(), 1);
        assert_eq!(zmachine.stack[zmachine.stack.len() - 1], table_addr as u16);
    }

    #[test]
    fn test_scan_table_error_conditions() {
        let mut zmachine = MockZMachine::new();
        
        // Test with insufficient operands
        zmachine.operands_buffer = vec![0x1234, 0x200]; // Only 2 operands, need at least 3
        let result = zmachine.op_scan_table();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing operands"));
        
        // Test with zero entry length
        zmachine.operands_buffer = vec![0x1234, 0x200, 1, 0x00]; // form = 0x00 (zero length)
        let result = zmachine.op_scan_table();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("entry length cannot be zero"));
    }

    #[test]
    fn test_print_table_basic() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let text_addr = 0x200;
        let test_text = "Hello World!";
        
        // Set up text in memory
        for (i, ch) in test_text.chars().enumerate() {
            zmachine.memory[text_addr + i] = ch as u8;
        }
        
        // Set up operands for PRINT_TABLE
        zmachine.operands_buffer = vec![
            text_addr as u16,   // text address
            12,                 // width
            1,                  // height
            0                   // skip
        ];
        
        let result = zmachine.op_print_table();
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_table_multiple_rows() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let text_addr = 0x200;
        
        // Set up operands for PRINT_TABLE with multiple rows
        zmachine.operands_buffer = vec![
            text_addr as u16,   // text address
            5,                  // width
            3,                  // height (3 rows)
            2                   // skip 2 characters between rows
        ];
        
        let result = zmachine.op_print_table();
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_table_default_parameters() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let text_addr = 0x200;
        
        // Test with minimal operands (should use defaults)
        zmachine.operands_buffer = vec![
            text_addr as u16,   // text address
            10                  // width only (height defaults to 1, skip defaults to 0)
        ];
        
        let result = zmachine.op_print_table();
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_table_error_conditions() {
        let mut zmachine = MockZMachine::new();
        
        // Test with insufficient operands
        zmachine.operands_buffer = vec![0x200]; // Only 1 operand, need at least 2
        let result = zmachine.op_print_table();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing operands"));
        
        // Test with zero width
        zmachine.operands_buffer = vec![0x200, 0]; // width = 0
        let result = zmachine.op_print_table();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("width cannot be zero"));
    }

    #[test]
    fn test_table_operations_integration() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let source_addr = 0x100;
        let dest_addr = 0x200;
        let table_addr = 0x300;
        
        // Set up source data
        let data = [0x12, 0x34, 0x56, 0x78, 0xAB, 0xCD];
        for (i, &byte) in data.iter().enumerate() {
            zmachine.memory[source_addr + i] = byte;
        }
        
        // 1. Copy data using COPY_TABLE
        zmachine.operands_buffer = vec![source_addr as u16, dest_addr as u16, data.len() as u16];
        let result = zmachine.op_copy_table();
        assert!(result.is_ok());
        
        // 2. Copy to create a searchable table
        zmachine.operands_buffer = vec![dest_addr as u16, table_addr as u16, data.len() as u16];
        let result = zmachine.op_copy_table();
        assert!(result.is_ok());
        
        // 3. Search the table using SCAN_TABLE
        zmachine.operands_buffer = vec![0x5678, table_addr as u16, 3, 0x02];
        let result = zmachine.op_scan_table();
        assert!(result.is_ok());
        
        // Should find the value at the correct address
        assert_eq!(zmachine.stack.len(), 1);
        assert_eq!(zmachine.stack[zmachine.stack.len() - 1], (table_addr + 2) as u16);
        
        // 4. Print the table using PRINT_TABLE
        zmachine.operands_buffer = vec![table_addr as u16, 6, 1, 0];
        let result = zmachine.op_print_table();
        assert!(result.is_ok());
    }

    #[test]
    fn test_copy_table_large_data() {
        let mut zmachine = MockZMachine::with_memory_size(2048);
        
        let source_addr = 0x100;
        let dest_addr = 0x400;
        let size = 256;
        
        // Set up large source data with pattern
        for i in 0..size {
            zmachine.memory[source_addr + i] = (i % 256) as u8;
        }
        
        // Copy large block
        zmachine.operands_buffer = vec![source_addr as u16, dest_addr as u16, size as u16];
        let result = zmachine.op_copy_table();
        assert!(result.is_ok());
        
        // Verify the pattern was copied correctly
        for i in 0..size {
            assert_eq!(zmachine.memory[dest_addr + i], (i % 256) as u8);
        }
    }
}