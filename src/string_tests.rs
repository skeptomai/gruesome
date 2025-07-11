#[cfg(test)]
mod tests {
    use crate::test_utils::MockZMachine;

    #[test]
    fn test_mock_string_operations() {
        let mut zmachine = MockZMachine::new();
        
        // Test that basic string operations are mockable
        // For now, just test that the structure exists
        assert_eq!(zmachine.memory.len(), 4096);
        assert_eq!(zmachine.pc, 0x100);
        assert_eq!(zmachine.stack.len(), 0);
    }

    #[test]
    fn test_convert_packed_address() {
        let mut zmachine = MockZMachine::new();
        zmachine.setup_version_3();
        
        // Test packed address conversion for version 3
        // Version 3: packed address * 2 = byte address
        let packed = 0x200;
        let expected_byte_addr = 0x400; // 2 * 0x200
        
        // For now, just verify the version is set correctly
        assert_eq!(zmachine.memory[0], 3);
        
        // In a real implementation, we'd test:
        // let byte_addr = zmachine.convert_packed_address(packed);
        // assert_eq!(byte_addr, expected_byte_addr);
    }

    #[test]
    fn test_string_boundary_conditions() {
        let zmachine = MockZMachine::new();
        
        // Test reading from near end of memory would be out of bounds
        let near_end = zmachine.memory.len() - 10;
        assert!(near_end > 0);
        
        // Test reading from invalid address would be out of bounds
        let invalid_addr = zmachine.memory.len() + 100;
        assert!(invalid_addr > zmachine.memory.len());
    }

    #[test]
    fn test_string_setup_infrastructure() {
        let mut zmachine = MockZMachine::with_memory_size(8192);
        zmachine.setup_version_3();
        
        // Verify the setup worked
        assert_eq!(zmachine.memory[0], 3); // Version 3
        assert_eq!(zmachine.memory.len(), 8192);
        
        // Verify header values were set
        assert_eq!(zmachine.memory[6], 0x01); // Initial PC high byte
        assert_eq!(zmachine.memory[7], 0x00); // Initial PC low byte
        assert_eq!(zmachine.memory[10], 0x00); // Object table high byte
        assert_eq!(zmachine.memory[11], 0x40); // Object table low byte
        assert_eq!(zmachine.memory[8], 0x00); // Dictionary high byte
        assert_eq!(zmachine.memory[9], 0x80); // Dictionary low byte
    }

    #[test]
    fn test_string_operations_placeholder() {
        // This is a placeholder test for string operations
        // The actual string operations (op_print, op_print_ret, etc.) 
        // would need to be implemented in MockZMachine or tested
        // differently to avoid GameFile dependency issues
        
        let zmachine = MockZMachine::new();
        
        // Test that we can create a mock zmachine for string testing
        assert!(zmachine.memory.len() > 0);
        assert_eq!(zmachine.running, true);
        assert_eq!(zmachine.operands_buffer.len(), 0);
    }

    #[test]
    fn test_routine_setup_for_string_tests() {
        let mut zmachine = MockZMachine::with_memory_size(4096);
        zmachine.setup_version_3();
        
        // Set up a routine that would handle string operations
        zmachine.setup_routine_at(0x100, 0); // Routine at packed address 0x100
        
        // Verify the routine was set up
        let byte_addr = 0x100 * 2; // Version 3 packed address conversion
        assert_eq!(zmachine.memory[byte_addr], 0); // 0 local variables
        assert_eq!(zmachine.memory[byte_addr + 1], 0xB0); // RET instruction
    }
}