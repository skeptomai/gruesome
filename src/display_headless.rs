//! Headless display implementation for testing and CI environments
//!
//! This implementation collects all output without displaying it,
//! useful for automated testing and non-interactive environments.

use crate::display_trait::{DisplayError, ZMachineDisplay};
use log::debug;

#[derive(Debug)]
pub struct HeadlessDisplay {
    buffer: Vec<String>,
    current_line: String,
    cursor: (u16, u16),
    upper_window_lines: u16,
    current_window: u8,
    terminal_width: u16,
    terminal_height: u16,
}

impl HeadlessDisplay {
    pub fn new() -> Result<Self, DisplayError> {
        Ok(Self {
            buffer: Vec::new(),
            current_line: String::new(),
            cursor: (1, 1),
            upper_window_lines: 0,
            current_window: 0,
            terminal_width: 80,
            terminal_height: 24,
        })
    }
    
    /// Get the current buffer content (for testing)
    pub fn get_buffer(&self) -> &[String] {
        &self.buffer
    }
    
    /// Get all output as a single string
    pub fn get_output(&self) -> String {
        let mut output = self.buffer.join("\n");
        if !self.current_line.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&self.current_line);
        }
        output
    }
    
    /// Flush the current line to the buffer
    fn flush_line(&mut self) {
        if !self.current_line.is_empty() || self.buffer.is_empty() {
            self.buffer.push(self.current_line.clone());
            self.current_line.clear();
        }
    }
}

impl ZMachineDisplay for HeadlessDisplay {
    fn clear_screen(&mut self) -> Result<(), DisplayError> {
        self.buffer.clear();
        self.current_line.clear();
        Ok(())
    }
    
    fn split_window(&mut self, lines: u16) -> Result<(), DisplayError> {
        debug!("Headless: split_window({})", lines);
        self.upper_window_lines = lines;
        Ok(())
    }
    
    fn set_window(&mut self, window: u8) -> Result<(), DisplayError> {
        debug!("Headless: set_window({})", window);
        self.current_window = window;
        Ok(())
    }
    
    fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), DisplayError> {
        debug!("Headless: set_cursor({}, {})", line, column);
        self.cursor = (line, column);
        Ok(())
    }
    
    fn print(&mut self, text: &str) -> Result<(), DisplayError> {
        // Handle newlines properly
        if !text.contains('\n') {
            self.current_line.push_str(text);
        } else {
            let lines: Vec<&str> = text.split('\n').collect();
            for (i, line) in lines.iter().enumerate() {
                self.current_line.push_str(line);
                if i < lines.len() - 1 {
                    self.flush_line();
                }
            }
        }
        Ok(())
    }
    
    fn print_char(&mut self, ch: char) -> Result<(), DisplayError> {
        if ch == '\n' {
            self.flush_line();
        } else {
            self.current_line.push(ch);
        }
        Ok(())
    }
    
    fn erase_window(&mut self, _window: i16) -> Result<(), DisplayError> {
        self.buffer.clear();
        self.current_line.clear();
        Ok(())
    }
    
    fn show_status(&mut self, location: &str, score: i16, moves: u16) -> Result<(), DisplayError> {
        let status = format!("{} Score: {} Moves: {}", location, score, moves);
        self.buffer.push(format!("[STATUS: {}]", status));
        Ok(())
    }
    
    fn handle_resize(&mut self, width: u16, height: u16) {
        self.terminal_width = width;
        self.terminal_height = height;
    }
    
    fn erase_line(&mut self) -> Result<(), DisplayError> {
        self.buffer.push("[ERASE_LINE]".to_string());
        Ok(())
    }
    
    fn get_cursor(&mut self) -> Result<(u16, u16), DisplayError> {
        Ok(self.cursor)
    }
    
    fn set_buffer_mode(&mut self, _buffered: bool) -> Result<(), DisplayError> {
        Ok(())
    }
    
    fn get_terminal_size(&self) -> (u16, u16) {
        (self.terminal_width, self.terminal_height)
    }
    
    fn force_refresh(&mut self) -> Result<(), DisplayError> {
        self.flush_line();
        Ok(())
    }
}

impl Drop for HeadlessDisplay {
    fn drop(&mut self) {
        // Flush any remaining content
        self.flush_line();
    }
}