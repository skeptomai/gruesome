#[cfg(test)]
mod tests {
    use crate::test_utils::MockZMachine;

    #[test]
    fn test_random_positive_range() {
        let mut zmachine = MockZMachine::new();
        
        // Test RANDOM with positive range (1-6, like a dice roll)
        zmachine.operands_buffer = vec![6];
        let result = zmachine.op_random();
        assert!(result.is_ok());
        
        // Check that result is stored on stack and within range
        assert_eq!(zmachine.stack.len(), 1);
        let random_value = zmachine.stack[0];
        assert!(random_value >= 1 && random_value <= 6);
    }

    #[test]
    fn test_random_zero_range() {
        let mut zmachine = MockZMachine::new();
        
        // Test RANDOM with zero range (should re-seed randomly)
        zmachine.operands_buffer = vec![0];
        let result = zmachine.op_random();
        assert!(result.is_ok());
        
        // Check that result is 0
        assert_eq!(zmachine.stack.len(), 1);
        assert_eq!(zmachine.stack[0], 0);
    }

    #[test]
    fn test_random_negative_range() {
        let mut zmachine = MockZMachine::new();
        
        // Test RANDOM with negative range (should set seed)
        zmachine.operands_buffer = vec![(-42i16) as u16];
        let result = zmachine.op_random();
        assert!(result.is_ok());
        
        // Check that result is 0 and seed is set to 42
        assert_eq!(zmachine.stack.len(), 1);
        assert_eq!(zmachine.stack[0], 0);
        assert_eq!(zmachine.random_seed, 42);
    }

    #[test]
    fn test_random_deterministic() {
        let mut zmachine = MockZMachine::new();
        
        // Set a known seed
        zmachine.random_seed = 54321;
        
        // Generate random number
        zmachine.operands_buffer = vec![100];
        zmachine.op_random().unwrap();
        let first_result = zmachine.stack[0];
        
        // Reset seed and generate again
        zmachine.random_seed = 54321;
        zmachine.stack.clear();
        zmachine.op_random().unwrap();
        let second_result = zmachine.stack[0];
        
        assert_eq!(first_result, second_result);
    }

    #[test]
    fn test_random_no_operand() {
        let mut zmachine = MockZMachine::new();
        
        // Test RANDOM with no operands (should fail)
        zmachine.operands_buffer = vec![];
        let result = zmachine.op_random();
        assert!(result.is_err());
    }

    #[test]
    fn test_random_boundary_values() {
        let mut zmachine = MockZMachine::new();
        
        // Test with range 1 (should always return 1)
        zmachine.operands_buffer = vec![1];
        zmachine.op_random().unwrap();
        assert_eq!(zmachine.stack[0], 1);
        
        // Test with large range
        zmachine.stack.clear();
        zmachine.operands_buffer = vec![1000];
        zmachine.op_random().unwrap();
        let result = zmachine.stack[0];
        assert!(result >= 1 && result <= 1000);
    }

    #[test]
    fn test_random_multiple_calls() {
        let mut zmachine = MockZMachine::new();
        
        // Generate multiple random numbers and check they're different
        zmachine.operands_buffer = vec![100];
        let mut results = Vec::new();
        
        for _ in 0..5 {
            zmachine.op_random().unwrap();
            results.push(zmachine.stack.pop().unwrap());
        }
        
        // Check that all results are in range
        for &result in &results {
            assert!(result >= 1 && result <= 100);
        }
        
        // Check that not all results are the same
        let all_same = results.windows(2).all(|w| w[0] == w[1]);
        assert!(!all_same, "Random numbers should not all be the same");
    }

    #[test]
    fn test_save_instruction() {
        let mut zmachine = MockZMachine::new();
        
        // Test SAVE instruction (mock implementation)
        let result = zmachine.op_save();
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_file_io_error_handling() {
        let mut zmachine = MockZMachine::new();
        
        // Test restoring from non-existent file
        let result = zmachine.op_restore();
        assert!(result.is_ok()); // Should handle error gracefully
        
        // Test saving (mock implementation)
        let result = zmachine.op_save();
        assert!(result.is_ok()); // Should handle error gracefully
    }

    #[test]
    fn test_restore_instruction() {
        let mut zmachine = MockZMachine::new();
        
        // Test RESTORE instruction (mock implementation)
        let result = zmachine.op_restore();
        assert!(result.is_ok());
    }

    #[test]
    fn test_restart_instruction() {
        let mut zmachine = MockZMachine::new();
        
        // Modify machine state
        zmachine.pc = 0x200;
        zmachine.stack.push(42);
        zmachine.global_vars.insert(1, 100);
        zmachine.local_vars[0] = 50;
        zmachine.random_seed = 99999;
        
        // Test RESTART instruction
        let result = zmachine.op_restart();
        assert!(result.is_ok());
        
        // Check that state was reset
        assert_eq!(zmachine.stack.len(), 0);
        assert_eq!(zmachine.global_vars.len(), 0);
        assert_eq!(zmachine.local_vars[0], 0);
        assert_eq!(zmachine.random_seed, 12345);
    }

    #[test]
    fn test_verify_instruction() {
        let mut zmachine = MockZMachine::new();
        
        // Test VERIFY instruction (mock implementation)
        let result = zmachine.op_verify();
        assert!(result.is_ok());
    }

    #[test]
    fn test_game_flow_simple() {
        let mut zmachine = MockZMachine::new();
        
        // Test basic random functionality
        zmachine.operands_buffer = vec![10];
        zmachine.op_random().unwrap();
        let random1 = zmachine.stack[0];
        assert!(random1 >= 1 && random1 <= 10);
        
        // Test seeding
        zmachine.operands_buffer = vec![(-1000i16) as u16];
        zmachine.op_random().unwrap();
        assert_eq!(zmachine.random_seed, 1000);
        
        // Test other operations don't crash
        zmachine.op_verify().unwrap();
        zmachine.op_save().unwrap();
        zmachine.op_restore().unwrap();
    }

    #[test]
    fn test_call_1s_instruction() {
        let mut zmachine = MockZMachine::new();
        
        // Test CALL_1S with routine address 0 (special case - should return 0 immediately)
        let result = zmachine.op_call_1s(0);
        assert!(result.is_ok());
        assert_eq!(zmachine.stack.len(), 1);
        assert_eq!(zmachine.stack[0], 0);
        
        // Test CALL_1S with actual routine address
        zmachine.stack.clear();
        let result = zmachine.op_call_1s(0x100);
        assert!(result.is_ok());
        assert_eq!(zmachine.stack.len(), 1);
        assert_eq!(zmachine.stack[0], 42); // Mock return value
    }

    #[test]
    fn test_call_2s_instruction() {
        let mut zmachine = MockZMachine::new();
        
        // Test CALL_2S with routine address 0
        let result = zmachine.op_call_2s(0, 42);
        assert!(result.is_ok());
        assert_eq!(zmachine.stack.len(), 1);
        assert_eq!(zmachine.stack[0], 0);
        
        // Test CALL_2S with actual routine
        zmachine.stack.clear();
        let result = zmachine.op_call_2s(0x100, 42);
        assert!(result.is_ok());
        assert_eq!(zmachine.stack.len(), 1);
        assert_eq!(zmachine.stack[0], 42); // Mock returns the argument
    }

    #[test]
    fn test_call_2n_instruction() {
        let mut zmachine = MockZMachine::new();
        
        // Test CALL_2N with routine address 0 (special case)
        let result = zmachine.op_call_2n(0, 42);
        assert!(result.is_ok());
        assert_eq!(zmachine.stack.len(), 0); // No result stored
        
        // Test CALL_2N with actual routine
        let result = zmachine.op_call_2n(0x100, 42);
        assert!(result.is_ok());
        assert_eq!(zmachine.stack.len(), 0); // No result stored
    }

    #[test]
    fn test_advanced_calling_variants() {
        let mut zmachine = MockZMachine::new();
        
        // Test that all advanced calling variants exist and don't crash
        let _ = zmachine.op_call_1s(0); // Should handle routine 0
        let _ = zmachine.op_call_2s(0, 0); // Should handle routine 0
        let _ = zmachine.op_call_2n(0, 0); // Should handle routine 0
        
        // All should complete without panicking
        assert!(true);
    }

    #[test]
    fn test_call_variants_with_mock_routines() {
        let mut zmachine = MockZMachine::with_memory_size(4096);
        zmachine.setup_version_3();
        
        // Set up routines in memory
        zmachine.setup_routine_at(0x100, 0); // Routine at packed address 0x100 (byte 0x200)
        zmachine.setup_routine_at(0x200, 1); // Routine at packed address 0x200 (byte 0x400)
        
        // Test calling routines
        let result = zmachine.op_call_1s(0x100);
        assert!(result.is_ok());
        
        let result = zmachine.op_call_2s(0x200, 123);
        assert!(result.is_ok());
        
        let result = zmachine.op_call_2n(0x100, 456);
        assert!(result.is_ok());
    }
}