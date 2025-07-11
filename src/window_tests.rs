#[cfg(test)]
mod tests {
    use crate::test_utils::MockZMachine;

    #[test]
    fn test_split_window_basic() {
        let mut zmachine = MockZMachine::new();
        
        // Initially, upper window should have no height
        assert_eq!(zmachine.windows[1].height, 0);
        assert_eq!(zmachine.windows[0].height, 24);
        
        // Split window with 3 lines for upper window
        zmachine.operands_buffer = vec![3];
        let result = zmachine.op_split_window();
        assert!(result.is_ok());
        
        // Check window dimensions
        assert_eq!(zmachine.windows[1].height, 3); // Upper window
        assert_eq!(zmachine.windows[0].height, 21); // Lower window (24 - 3)
        assert_eq!(zmachine.windows[0].top, 4); // Lower window starts at row 4
        
        // Check cursor positions reset in upper window
        assert_eq!(zmachine.windows[1].cursor_row, 1);
        assert_eq!(zmachine.windows[1].cursor_col, 1);
    }
    
    #[test]
    fn test_split_window_unsplit() {
        let mut zmachine = MockZMachine::new();
        
        // First split the window
        zmachine.operands_buffer = vec![5];
        zmachine.op_split_window().unwrap();
        
        // Verify it's split
        assert_eq!(zmachine.windows[1].height, 5);
        
        // Now unsplit (lines = 0)
        zmachine.operands_buffer = vec![0];
        let result = zmachine.op_split_window();
        assert!(result.is_ok());
        
        // Upper window should have no height
        assert_eq!(zmachine.windows[1].height, 0);
        // Lower window should take full screen
        assert_eq!(zmachine.windows[0].height, 24);
        assert_eq!(zmachine.windows[0].top, 1);
        // Should switch to lower window
        assert_eq!(zmachine.current_window, 0);
    }
    
    #[test]
    fn test_split_window_boundary_conditions() {
        let mut zmachine = MockZMachine::new();
        
        // Try to split with more lines than screen height
        zmachine.operands_buffer = vec![30]; // More than 24 lines
        let result = zmachine.op_split_window();
        assert!(result.is_ok());
        
        // Should be clamped to screen_height - 1
        assert_eq!(zmachine.windows[1].height, 23); // 24 - 1
        assert_eq!(zmachine.windows[0].height, 1);
    }
    
    #[test]
    fn test_split_window_missing_operand() {
        let mut zmachine = MockZMachine::new();
        
        // Test with no operands
        zmachine.operands_buffer = vec![];
        let result = zmachine.op_split_window();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing operand"));
    }
    
    #[test]
    fn test_set_window_lower() {
        let mut zmachine = MockZMachine::new();
        
        // Should start in lower window
        assert_eq!(zmachine.current_window, 0);
        
        // Explicitly set to lower window
        zmachine.operands_buffer = vec![0];
        let result = zmachine.op_set_window();
        assert!(result.is_ok());
        assert_eq!(zmachine.current_window, 0);
    }
    
    #[test]
    fn test_set_window_upper() {
        let mut zmachine = MockZMachine::new();
        
        // First split the window
        zmachine.operands_buffer = vec![3];
        zmachine.op_split_window().unwrap();
        
        // Now switch to upper window
        zmachine.operands_buffer = vec![1];
        let result = zmachine.op_set_window();
        assert!(result.is_ok());
        assert_eq!(zmachine.current_window, 1);
    }
    
    #[test]
    fn test_set_window_upper_not_split() {
        let mut zmachine = MockZMachine::new();
        
        // Try to switch to upper window without splitting
        zmachine.operands_buffer = vec![1];
        let result = zmachine.op_set_window();
        assert!(result.is_ok());
        
        // Should remain in lower window since upper window has no height
        assert_eq!(zmachine.current_window, 0);
    }
    
    #[test]
    fn test_set_window_invalid() {
        let mut zmachine = MockZMachine::new();
        
        // Try invalid window number
        zmachine.operands_buffer = vec![2];
        let result = zmachine.op_set_window();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid window number"));
    }
    
    #[test]
    fn test_set_window_missing_operand() {
        let mut zmachine = MockZMachine::new();
        
        // Test with no operands
        zmachine.operands_buffer = vec![];
        let result = zmachine.op_set_window();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing operand"));
    }
    
    #[test]
    fn test_erase_window_lower() {
        let mut zmachine = MockZMachine::new();
        
        // Move cursor away from origin
        if let Some(window) = zmachine.windows.get_mut(0) {
            window.cursor_row = 10;
            window.cursor_col = 20;
        }
        
        // Erase lower window
        zmachine.operands_buffer = vec![0];
        let result = zmachine.op_erase_window();
        assert!(result.is_ok());
        
        // Cursor should be reset to origin
        assert_eq!(zmachine.windows[0].cursor_row, 1);
        assert_eq!(zmachine.windows[0].cursor_col, 1);
    }
    
    #[test]
    fn test_erase_window_upper() {
        let mut zmachine = MockZMachine::new();
        
        // Split window first
        zmachine.operands_buffer = vec![3];
        zmachine.op_split_window().unwrap();
        
        // Move cursor in upper window
        if let Some(window) = zmachine.windows.get_mut(1) {
            window.cursor_row = 2;
            window.cursor_col = 10;
        }
        
        // Erase upper window
        zmachine.operands_buffer = vec![1];
        let result = zmachine.op_erase_window();
        assert!(result.is_ok());
        
        // Cursor should be reset
        assert_eq!(zmachine.windows[1].cursor_row, 1);
        assert_eq!(zmachine.windows[1].cursor_col, 1);
    }
    
    #[test]
    fn test_erase_window_entire_screen() {
        let mut zmachine = MockZMachine::new();
        
        // Split window and move cursors
        zmachine.operands_buffer = vec![3];
        zmachine.op_split_window().unwrap();
        
        zmachine.windows[0].cursor_row = 10;
        zmachine.windows[1].cursor_row = 2;
        
        // Erase entire screen
        zmachine.operands_buffer = vec![-1_i16 as u16];
        let result = zmachine.op_erase_window();
        assert!(result.is_ok());
        
        // Both windows should have cursors reset
        assert_eq!(zmachine.windows[0].cursor_row, 1);
        assert_eq!(zmachine.windows[1].cursor_row, 1);
    }
    
    #[test]
    fn test_erase_window_and_unsplit() {
        let mut zmachine = MockZMachine::new();
        
        // Split window first
        zmachine.operands_buffer = vec![5];
        zmachine.op_split_window().unwrap();
        assert_eq!(zmachine.windows[1].height, 5);
        
        // Erase entire screen and unsplit
        zmachine.operands_buffer = vec![-2_i16 as u16];
        let result = zmachine.op_erase_window();
        assert!(result.is_ok());
        
        // Window should be unsplit
        assert_eq!(zmachine.windows[1].height, 0);
        assert_eq!(zmachine.windows[0].height, 24);
    }
    
    #[test]
    fn test_erase_window_invalid() {
        let mut zmachine = MockZMachine::new();
        
        // Invalid window spec
        zmachine.operands_buffer = vec![-5_i16 as u16];
        let result = zmachine.op_erase_window();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid window specification"));
    }
    
    #[test]
    fn test_set_cursor_basic() {
        let mut zmachine = MockZMachine::new();
        
        // Set cursor to row 5, column 10
        zmachine.operands_buffer = vec![5, 10];
        let result = zmachine.op_set_cursor();
        assert!(result.is_ok());
        
        // Check cursor position in current window (lower window)
        assert_eq!(zmachine.windows[0].cursor_row, 5);
        assert_eq!(zmachine.windows[0].cursor_col, 10);
    }
    
    #[test]
    fn test_set_cursor_clamping() {
        let mut zmachine = MockZMachine::new();
        
        // Try to set cursor beyond window boundaries
        zmachine.operands_buffer = vec![100, 200]; // Way beyond 24x80
        let result = zmachine.op_set_cursor();
        assert!(result.is_ok());
        
        // Should be clamped to window size
        assert_eq!(zmachine.windows[0].cursor_row, 24); // Clamped to height
        assert_eq!(zmachine.windows[0].cursor_col, 80); // Clamped to width
    }
    
    #[test]
    fn test_set_cursor_upper_window() {
        let mut zmachine = MockZMachine::new();
        
        // Split window and switch to upper
        zmachine.operands_buffer = vec![3];
        zmachine.op_split_window().unwrap();
        zmachine.operands_buffer = vec![1];
        zmachine.op_set_window().unwrap();
        
        // Set cursor in upper window
        zmachine.operands_buffer = vec![2, 5];
        let result = zmachine.op_set_cursor();
        assert!(result.is_ok());
        
        assert_eq!(zmachine.windows[1].cursor_row, 2);
        assert_eq!(zmachine.windows[1].cursor_col, 5);
        
        // Lower window cursor should be unchanged
        assert_eq!(zmachine.windows[0].cursor_row, 1);
    }
    
    #[test]
    fn test_set_cursor_invalid_position() {
        let mut zmachine = MockZMachine::new();
        
        // Try zero-based coordinates (invalid)
        zmachine.operands_buffer = vec![0, 1];
        let result = zmachine.op_set_cursor();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid cursor position"));
        
        // Try with column zero
        zmachine.operands_buffer = vec![1, 0];
        let result = zmachine.op_set_cursor();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid cursor position"));
    }
    
    #[test]
    fn test_set_cursor_missing_operands() {
        let mut zmachine = MockZMachine::new();
        
        // Test with insufficient operands
        zmachine.operands_buffer = vec![5]; // Only 1 operand, need 2
        let result = zmachine.op_set_cursor();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing operands"));
    }
    
    #[test]
    fn test_get_cursor_basic() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        // Set cursor to known position
        zmachine.windows[0].cursor_row = 15;
        zmachine.windows[0].cursor_col = 25;
        
        // Get cursor position
        let table_addr = 0x200;
        zmachine.operands_buffer = vec![table_addr];
        let result = zmachine.op_get_cursor();
        assert!(result.is_ok());
        
        // Check stored values
        let stored_row = ((zmachine.memory[table_addr as usize] as u16) << 8) | 
                        (zmachine.memory[table_addr as usize + 1] as u16);
        let stored_col = ((zmachine.memory[table_addr as usize + 2] as u16) << 8) | 
                        (zmachine.memory[table_addr as usize + 3] as u16);
        
        assert_eq!(stored_row, 15);
        assert_eq!(stored_col, 25);
    }
    
    #[test]
    fn test_get_cursor_upper_window() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        // Split and switch to upper window
        zmachine.operands_buffer = vec![3];
        zmachine.op_split_window().unwrap();
        zmachine.operands_buffer = vec![1];
        zmachine.op_set_window().unwrap();
        
        // Set cursor in upper window
        zmachine.windows[1].cursor_row = 2;
        zmachine.windows[1].cursor_col = 10;
        
        // Get cursor position
        let table_addr = 0x300;
        zmachine.operands_buffer = vec![table_addr];
        let result = zmachine.op_get_cursor();
        assert!(result.is_ok());
        
        // Check stored values
        let stored_row = ((zmachine.memory[table_addr as usize] as u16) << 8) | 
                        (zmachine.memory[table_addr as usize + 1] as u16);
        let stored_col = ((zmachine.memory[table_addr as usize + 2] as u16) << 8) | 
                        (zmachine.memory[table_addr as usize + 3] as u16);
        
        assert_eq!(stored_row, 2);
        assert_eq!(stored_col, 10);
    }
    
    #[test]
    fn test_get_cursor_bounds_check() {
        let mut zmachine = MockZMachine::with_memory_size(512);
        
        // Try address that would overflow memory
        zmachine.operands_buffer = vec![510]; // 510 + 3 = 513 > 512
        let result = zmachine.op_get_cursor();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
    }
    
    #[test]
    fn test_get_cursor_missing_operand() {
        let mut zmachine = MockZMachine::new();
        
        // Test with no operands
        zmachine.operands_buffer = vec![];
        let result = zmachine.op_get_cursor();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing operand"));
    }
    
    #[test]
    fn test_set_text_style_basic() {
        let mut zmachine = MockZMachine::new();
        
        // Set text style
        let style = 0b0011; // Bold + reverse
        zmachine.operands_buffer = vec![style];
        let result = zmachine.op_set_text_style();
        assert!(result.is_ok());
        
        // Check that style was set in current window
        assert_eq!(zmachine.windows[0].text_style, style);
    }
    
    #[test]
    fn test_set_text_style_upper_window() {
        let mut zmachine = MockZMachine::new();
        
        // Split and switch to upper window
        zmachine.operands_buffer = vec![3];
        zmachine.op_split_window().unwrap();
        zmachine.operands_buffer = vec![1];
        zmachine.op_set_window().unwrap();
        
        // Set style in upper window
        let style = 0b1100; // Italic + fixed-width
        zmachine.operands_buffer = vec![style];
        let result = zmachine.op_set_text_style();
        assert!(result.is_ok());
        
        // Check upper window has style set
        assert_eq!(zmachine.windows[1].text_style, style);
        // Lower window should still have default style
        assert_eq!(zmachine.windows[0].text_style, 0);
    }
    
    #[test]
    fn test_set_text_style_missing_operand() {
        let mut zmachine = MockZMachine::new();
        
        // Test with no operands
        zmachine.operands_buffer = vec![];
        let result = zmachine.op_set_text_style();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("missing operand"));
    }
    
    #[test]
    fn test_erase_line_basic() {
        let mut zmachine = MockZMachine::new();
        
        // Test erase line (mock implementation)
        zmachine.operands_buffer = vec![1];
        let result = zmachine.op_erase_line();
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_window_operations_integration() {
        let mut zmachine = MockZMachine::with_memory_size(1024);
        
        // Complete workflow test
        
        // 1. Split window
        zmachine.operands_buffer = vec![4];
        zmachine.op_split_window().unwrap();
        assert_eq!(zmachine.windows[1].height, 4);
        assert_eq!(zmachine.windows[0].height, 20);
        
        // 2. Switch to upper window
        zmachine.operands_buffer = vec![1];
        zmachine.op_set_window().unwrap();
        assert_eq!(zmachine.current_window, 1);
        
        // 3. Set cursor in upper window
        zmachine.operands_buffer = vec![2, 10];
        zmachine.op_set_cursor().unwrap();
        assert_eq!(zmachine.windows[1].cursor_row, 2);
        assert_eq!(zmachine.windows[1].cursor_col, 10);
        
        // 4. Set text style in upper window
        zmachine.operands_buffer = vec![3]; // Bold + reverse
        zmachine.op_set_text_style().unwrap();
        assert_eq!(zmachine.windows[1].text_style, 3);
        
        // 5. Get cursor position
        let table_addr = 0x300; // Use a safe address
        zmachine.operands_buffer = vec![table_addr];
        zmachine.op_get_cursor().unwrap();
        
        let stored_row = ((zmachine.memory[table_addr as usize] as u16) << 8) | 
                        (zmachine.memory[table_addr as usize + 1] as u16);
        assert_eq!(stored_row, 2);
        
        // 6. Switch to lower window
        zmachine.operands_buffer = vec![0];
        zmachine.op_set_window().unwrap();
        assert_eq!(zmachine.current_window, 0);
        
        // 7. Clear upper window
        zmachine.operands_buffer = vec![1];
        zmachine.op_erase_window().unwrap();
        assert_eq!(zmachine.windows[1].cursor_row, 1); // Should be reset
        
        // 8. Unsplit window
        zmachine.operands_buffer = vec![0];
        zmachine.op_split_window().unwrap();
        assert_eq!(zmachine.windows[1].height, 0);
        assert_eq!(zmachine.windows[0].height, 24);
        assert_eq!(zmachine.current_window, 0);
    }
}