//! Tests for V3 display implementation

use gruesome::display_trait::ZMachineDisplay;
use gruesome::display_v3::V3Display;

#[test]
fn test_v3_immediate_status_refresh() {
    // V3 key behavior: status line updates immediately
    let mut display = V3Display::new().unwrap();
    
    // Create status line
    display.split_window(1).unwrap();
    
    // Update status - should display immediately
    display.show_status("West of House", 0, 0).unwrap();
    
    // In a real test, we'd capture terminal output
    // For now, just verify no errors
}

#[test]
fn test_v3_simple_printing() {
    let mut display = V3Display::new().unwrap();
    
    // V3 printing is direct to terminal
    display.print("Hello, world!").unwrap();
    display.print_char('!').unwrap();
    
    // No buffering in v3
}

#[test]
fn test_v3_window_operations() {
    let mut display = V3Display::new().unwrap();
    
    // V3 only supports 1-line status window
    display.split_window(1).unwrap();
    
    // Set window to upper
    display.set_window(1).unwrap();
    
    // Set cursor (rarely used in v3)
    display.set_cursor(1, 1).unwrap();
    
    // Switch back to lower
    display.set_window(0).unwrap();
}

#[test]
fn test_v3_unsupported_operations() {
    let mut display = V3Display::new().unwrap();
    
    // V4+ operations should fail gracefully
    assert!(display.erase_line().is_err());
    assert!(display.get_cursor().is_err());
    
    // Buffer mode is ignored
    display.set_buffer_mode(true).unwrap();
}