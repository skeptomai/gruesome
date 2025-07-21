//! Tests for V4+ display implementation

use gruesome::display_trait::ZMachineDisplay;
use gruesome::display_v4::V4Display;

#[test]
fn test_v4_deferred_refresh() {
    // V4 key behavior: upper window buffers content until window switch
    let mut display = V4Display::new().unwrap();
    
    // Create multi-line upper window
    display.split_window(3).unwrap();
    
    // Switch to upper window
    display.set_window(1).unwrap();
    
    // Print text - should NOT display immediately
    display.set_cursor(1, 1).unwrap();
    display.print("Mode: Communications Mode").unwrap();
    
    display.set_cursor(2, 1).unwrap();
    display.print("Time: 7:07pm").unwrap();
    
    // Content is buffered until we switch windows
    display.set_window(0).unwrap();
    // NOW the upper window should be refreshed
}

#[test]
fn test_v4_cursor_positioning() {
    let mut display = V4Display::new().unwrap();
    
    display.split_window(5).unwrap();
    display.set_window(1).unwrap();
    
    // Test precise cursor positioning
    display.set_cursor(3, 10).unwrap();
    display.print("Test").unwrap();
    
    // Cursor should advance after print
    display.set_cursor(3, 20).unwrap();
    display.print("More").unwrap();
}

#[test]
fn test_v4_window_buffering() {
    let mut display = V4Display::new().unwrap();
    
    display.split_window(2).unwrap();
    
    // Print to lower window - immediate
    display.set_window(0).unwrap();
    display.print("Lower window text").unwrap();
    
    // Print to upper window - buffered
    display.set_window(1).unwrap();
    display.print("Upper window text").unwrap();
    
    // Force refresh
    display.force_refresh().unwrap();
}

#[test]
fn test_v4_specific_operations() {
    let mut display = V4Display::new().unwrap();
    
    // V4+ operations should work
    display.erase_line().unwrap();
    
    // Get cursor position
    let (line, col) = display.get_cursor().unwrap();
    assert_eq!(line, 1);
    assert_eq!(col, 1);
    
    // Buffer mode
    display.set_buffer_mode(false).unwrap();
}

#[test]
fn test_v4_ignores_show_status() {
    let mut display = V4Display::new().unwrap();
    
    // V4 games don't use show_status - should be ignored
    display.show_status("Location", 100, 50).unwrap();
    // No error, but also no effect
}