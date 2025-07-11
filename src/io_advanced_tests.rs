#[cfg(test)]
mod tests {
    use crate::test_utils::MockZMachine;

    #[test]
    fn test_read_char_keyboard() {
        let mut zmachine = MockZMachine::new();
        
        // Test READ_CHAR with keyboard input (default)
        zmachine.operands_buffer = vec![1]; // Device 1 (keyboard)
        
        let result = zmachine.op_read_char();
        assert!(result.is_ok());
        
        // Should return 'A' (65) as mock input
        assert_eq!(zmachine.stack.len(), 1);
        assert_eq!(zmachine.stack[0], 65);
    }
    
    #[test]
    fn test_read_char_default_device() {
        let mut zmachine = MockZMachine::new();
        
        // Test READ_CHAR without device specified (should default to keyboard)
        zmachine.operands_buffer = vec![]; // No operands
        
        let result = zmachine.op_read_char();
        assert!(result.is_ok());
        
        // Should return 'A' (65) as mock input
        assert_eq!(zmachine.stack.len(), 1);
        assert_eq!(zmachine.stack[0], 65);
    }
    
    #[test]
    fn test_read_char_invalid_device() {
        let mut zmachine = MockZMachine::new();
        
        // Test READ_CHAR with invalid device
        zmachine.operands_buffer = vec![3]; // Invalid device
        
        let result = zmachine.op_read_char();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid input device"));
    }
    
    #[test]
    fn test_read_char_file_device() {
        let mut zmachine = MockZMachine::new();
        
        // Test READ_CHAR with file input (not implemented)
        zmachine.operands_buffer = vec![2]; // Device 2 (file)
        
        let result = zmachine.op_read_char();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("file not implemented"));
    }
    
    #[test]
    fn test_output_stream_enable_screen() {
        let mut zmachine = MockZMachine::new();
        
        // Screen output is enabled by default
        assert!(zmachine.output_streams.contains(&1));
        
        // Test enabling screen output (should have no effect)
        zmachine.operands_buffer = vec![1]; // Enable stream 1
        
        let result = zmachine.op_output_stream();
        assert!(result.is_ok());
        assert!(zmachine.output_streams.contains(&1));
    }
    
    #[test]
    fn test_output_stream_disable_screen() {
        let mut zmachine = MockZMachine::new();
        
        // Test disabling screen output
        zmachine.operands_buffer = vec![-1_i16 as u16]; // Disable stream 1
        
        let result = zmachine.op_output_stream();
        assert!(result.is_ok());
        assert!(!zmachine.output_streams.contains(&1));
    }
    
    #[test]
    fn test_output_stream_enable_transcript() {
        let mut zmachine = MockZMachine::new();
        
        // Test enabling transcript output
        zmachine.operands_buffer = vec![2]; // Enable stream 2
        
        let result = zmachine.op_output_stream();
        assert!(result.is_ok());
        assert!(zmachine.output_streams.contains(&2));
    }
    
    #[test]
    fn test_output_stream_enable_memory() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        // Test enabling memory output
        let table_addr = 0x200;
        zmachine.operands_buffer = vec![3, table_addr]; // Enable stream 3 at address
        
        let result = zmachine.op_output_stream();
        assert!(result.is_ok());
        assert!(zmachine.output_streams.contains(&3));
        assert_eq!(zmachine.memory_stream_addr, Some(table_addr));
        assert!(zmachine.memory_stream_data.is_empty());
    }
    
    #[test]
    fn test_output_stream_memory_missing_address() {
        let mut zmachine = MockZMachine::new();
        
        // Test enabling memory output without address
        zmachine.operands_buffer = vec![3]; // Enable stream 3 but no address
        
        let result = zmachine.op_output_stream();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires table address"));
    }
    
    #[test]
    fn test_output_stream_disable_memory() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        let table_addr = 0x200;
        
        // First enable memory output
        zmachine.operands_buffer = vec![3, table_addr];
        zmachine.op_output_stream().unwrap();
        
        // Add some data to memory stream
        zmachine.memory_stream_data.extend_from_slice(b"test");
        
        // Now disable memory output
        zmachine.operands_buffer = vec![-3_i16 as u16]; // Disable stream 3
        
        let result = zmachine.op_output_stream();
        assert!(result.is_ok());
        assert!(!zmachine.output_streams.contains(&3));
        
        // Data should be flushed to memory
        assert_eq!(zmachine.memory[table_addr as usize], 0); // Length high byte
        assert_eq!(zmachine.memory[table_addr as usize + 1], 4); // Length low byte (4 bytes)
        assert_eq!(zmachine.memory[table_addr as usize + 2], b't');
        assert_eq!(zmachine.memory[table_addr as usize + 3], b'e');
        assert_eq!(zmachine.memory[table_addr as usize + 4], b's');
        assert_eq!(zmachine.memory[table_addr as usize + 5], b't');
    }
    
    #[test]
    fn test_output_stream_invalid_stream() {
        let mut zmachine = MockZMachine::new();
        
        // Test invalid stream number
        zmachine.operands_buffer = vec![5]; // Invalid stream
        
        let result = zmachine.op_output_stream();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid stream number"));
    }
    
    #[test]
    fn test_output_stream_missing_operand() {
        let mut zmachine = MockZMachine::new();
        
        // Test missing operand
        zmachine.operands_buffer = vec![]; // No operands
        
        let result = zmachine.op_output_stream();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing operand"));
    }
    
    #[test]
    fn test_input_stream_keyboard() {
        let mut zmachine = MockZMachine::new();
        
        // Test setting input stream to keyboard
        zmachine.operands_buffer = vec![0]; // Stream 0 (keyboard)
        
        let result = zmachine.op_input_stream();
        assert!(result.is_ok());
        assert_eq!(zmachine.input_stream, 0);
    }
    
    #[test]
    fn test_input_stream_file() {
        let mut zmachine = MockZMachine::new();
        
        // Test setting input stream to file (not implemented)
        zmachine.operands_buffer = vec![1]; // Stream 1 (file)
        
        let result = zmachine.op_input_stream();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("file not implemented"));
    }
    
    #[test]
    fn test_input_stream_invalid() {
        let mut zmachine = MockZMachine::new();
        
        // Test invalid stream number
        zmachine.operands_buffer = vec![2]; // Invalid stream
        
        let result = zmachine.op_input_stream();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid stream number"));
    }
    
    #[test]
    fn test_input_stream_missing_operand() {
        let mut zmachine = MockZMachine::new();
        
        // Test missing operand
        zmachine.operands_buffer = vec![]; // No operands
        
        let result = zmachine.op_input_stream();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing operand"));
    }
    
    #[test]
    fn test_buffer_mode_enable() {
        let mut zmachine = MockZMachine::new();
        
        // Test enabling buffer mode
        zmachine.operands_buffer = vec![1]; // Enable buffering
        
        let result = zmachine.op_buffer_mode();
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_buffer_mode_disable() {
        let mut zmachine = MockZMachine::new();
        
        // Test disabling buffer mode
        zmachine.operands_buffer = vec![0]; // Disable buffering
        
        let result = zmachine.op_buffer_mode();
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_buffer_mode_invalid_flag() {
        let mut zmachine = MockZMachine::new();
        
        // Test invalid flag
        zmachine.operands_buffer = vec![2]; // Invalid flag
        
        let result = zmachine.op_buffer_mode();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid flag"));
    }
    
    #[test]
    fn test_buffer_mode_missing_operand() {
        let mut zmachine = MockZMachine::new();
        
        // Test missing operand
        zmachine.operands_buffer = vec![]; // No operands
        
        let result = zmachine.op_buffer_mode();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing operand"));
    }
    
    #[test]
    fn test_write_output_screen() {
        let mut zmachine = MockZMachine::new();
        
        // Test writing to screen output
        let result = zmachine.write_output("Hello World!");
        assert!(result.is_ok());
        assert_eq!(zmachine.test_output, "Hello World!");
    }
    
    #[test]
    fn test_write_output_memory_stream() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        // Enable memory output
        let table_addr = 0x200;
        zmachine.operands_buffer = vec![3, table_addr];
        zmachine.op_output_stream().unwrap();
        
        // Write to memory stream
        let result = zmachine.write_output("Test");
        assert!(result.is_ok());
        
        // Check that data was buffered
        assert_eq!(zmachine.memory_stream_data, b"Test");
        
        // Disable memory stream to flush
        zmachine.operands_buffer = vec![-3_i16 as u16];
        zmachine.op_output_stream().unwrap();
        
        // Check that data was written to memory
        assert_eq!(zmachine.memory[table_addr as usize], 0); // Length high byte
        assert_eq!(zmachine.memory[table_addr as usize + 1], 4); // Length low byte
        assert_eq!(zmachine.memory[table_addr as usize + 2], b'T');
        assert_eq!(zmachine.memory[table_addr as usize + 3], b'e');
        assert_eq!(zmachine.memory[table_addr as usize + 4], b's');
        assert_eq!(zmachine.memory[table_addr as usize + 5], b't');
    }
    
    #[test]
    fn test_write_output_multiple_streams() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        // Enable both screen and memory output
        let table_addr = 0x200;
        zmachine.operands_buffer = vec![3, table_addr];
        zmachine.op_output_stream().unwrap(); // Memory stream
        
        // Screen stream is enabled by default
        assert!(zmachine.output_streams.contains(&1));
        assert!(zmachine.output_streams.contains(&3));
        
        // Write to both streams
        let result = zmachine.write_output("Multi");
        assert!(result.is_ok());
        
        // Check screen output
        assert_eq!(zmachine.test_output, "Multi");
        
        // Check memory stream buffer
        assert_eq!(zmachine.memory_stream_data, b"Multi");
    }
    
    #[test]
    fn test_io_operations_integration() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        // Test a sequence of I/O operations
        
        // 1. Disable screen output
        zmachine.operands_buffer = vec![-1_i16 as u16];
        zmachine.op_output_stream().unwrap();
        assert!(!zmachine.output_streams.contains(&1));
        
        // 2. Enable memory output
        let table_addr = 0x200;
        zmachine.operands_buffer = vec![3, table_addr];
        zmachine.op_output_stream().unwrap();
        
        // 3. Write some output (should only go to memory)
        zmachine.write_output("Memory Only").unwrap();
        assert!(zmachine.test_output.is_empty()); // No screen output
        assert_eq!(zmachine.memory_stream_data, b"Memory Only");
        
        // 4. Re-enable screen output
        zmachine.operands_buffer = vec![1];
        zmachine.op_output_stream().unwrap();
        
        // 5. Write more output (should go to both)
        zmachine.write_output(" + Screen").unwrap();
        assert_eq!(zmachine.test_output, " + Screen");
        assert_eq!(zmachine.memory_stream_data, b"Memory Only + Screen");
        
        // 6. Disable memory output to flush
        zmachine.operands_buffer = vec![-3_i16 as u16];
        zmachine.op_output_stream().unwrap();
        
        // 7. Check memory contents
        assert_eq!(zmachine.memory[table_addr as usize], 0); // Length high byte
        assert_eq!(zmachine.memory[table_addr as usize + 1], 20); // Length low byte
        // First few bytes should be "Memory Only + Screen"
        assert_eq!(zmachine.memory[table_addr as usize + 2], b'M');
        assert_eq!(zmachine.memory[table_addr as usize + 3], b'e');
        assert_eq!(zmachine.memory[table_addr as usize + 4], b'm');
        
        // 8. Test READ_CHAR
        zmachine.operands_buffer = vec![1];
        zmachine.op_read_char().unwrap();
        assert_eq!(zmachine.stack.len(), 1);
        assert_eq!(zmachine.stack[0], 65); // 'A'
        
        // 9. Test INPUT_STREAM
        zmachine.operands_buffer = vec![0];
        zmachine.op_input_stream().unwrap();
        assert_eq!(zmachine.input_stream, 0);
        
        // 10. Test BUFFER_MODE
        zmachine.operands_buffer = vec![1];
        zmachine.op_buffer_mode().unwrap();
    }
}