//! WASM display implementation for browser-based Z-Machine
//!
//! This display buffers output and status updates for the JavaScript frontend
//! to poll and render. It does not directly manipulate the DOM.

use crate::interpreter::display::display_trait::{DisplayError, ZMachineDisplay};
use std::cell::RefCell;

/// Output message types for the JS frontend
#[derive(Debug, Clone)]
pub enum WasmDisplayMessage {
    /// Print text to the main output area
    Print(String),
    /// Update the status bar
    StatusUpdate {
        location: String,
        score: i16,
        moves: u16,
    },
    /// Clear the screen
    ClearScreen,
    /// Set the upper window size (for V4+ games)
    SplitWindow(u16),
    /// Set cursor position in upper window
    SetCursor { line: u16, column: u16 },
    /// Erase a window
    EraseWindow(i16),
    /// Set text style
    SetTextStyle(u16),
}

/// WASM display that buffers output for JavaScript consumption
#[derive(Debug)]
pub struct WasmDisplay {
    /// Buffered messages for the frontend
    messages: RefCell<Vec<WasmDisplayMessage>>,
    /// Current window (0 = lower, 1 = upper)
    current_window: u8,
    /// Upper window line count
    upper_window_lines: u16,
    /// Virtual cursor position
    cursor: (u16, u16),
    /// Virtual terminal size
    terminal_size: (u16, u16),
    /// Current text buffer for the lower window
    current_line: RefCell<String>,
}

impl WasmDisplay {
    pub fn new() -> Result<Self, DisplayError> {
        Ok(Self {
            messages: RefCell::new(Vec::new()),
            current_window: 0,
            upper_window_lines: 0,
            cursor: (1, 1),
            terminal_size: (80, 24), // Standard terminal size
            current_line: RefCell::new(String::new()),
        })
    }

    /// Take all buffered messages (clears the buffer)
    pub fn take_messages(&self) -> Vec<WasmDisplayMessage> {
        self.messages.borrow_mut().drain(..).collect()
    }

    /// Get pending output as a single string (for simple text output)
    pub fn take_output(&self) -> String {
        let messages = self.take_messages();
        let mut output = String::new();

        for msg in messages {
            if let WasmDisplayMessage::Print(text) = msg {
                output.push_str(&text);
            }
        }

        output
    }

    /// Check if there are pending messages
    pub fn has_messages(&self) -> bool {
        !self.messages.borrow().is_empty()
    }

    /// Flush current line buffer to messages
    fn flush_line(&self) {
        let mut line = self.current_line.borrow_mut();
        if !line.is_empty() {
            self.messages
                .borrow_mut()
                .push(WasmDisplayMessage::Print(line.clone()));
            line.clear();
        }
    }
}

impl Default for WasmDisplay {
    fn default() -> Self {
        Self::new().expect("WasmDisplay::new should not fail")
    }
}

impl ZMachineDisplay for WasmDisplay {
    fn clear_screen(&mut self) -> Result<(), DisplayError> {
        self.flush_line();
        self.messages
            .borrow_mut()
            .push(WasmDisplayMessage::ClearScreen);
        Ok(())
    }

    fn split_window(&mut self, lines: u16) -> Result<(), DisplayError> {
        self.upper_window_lines = lines;
        self.messages
            .borrow_mut()
            .push(WasmDisplayMessage::SplitWindow(lines));
        Ok(())
    }

    fn set_window(&mut self, window: u8) -> Result<(), DisplayError> {
        if self.current_window == 0 && window == 1 {
            // Switching from lower to upper - flush any pending output
            self.flush_line();
        }
        self.current_window = window;
        Ok(())
    }

    fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), DisplayError> {
        self.cursor = (line, column);
        self.messages
            .borrow_mut()
            .push(WasmDisplayMessage::SetCursor { line, column });
        Ok(())
    }

    fn print(&mut self, text: &str) -> Result<(), DisplayError> {
        // Handle newlines
        if text.contains('\n') {
            let lines: Vec<&str> = text.split('\n').collect();
            for (i, line) in lines.iter().enumerate() {
                self.current_line.borrow_mut().push_str(line);
                if i < lines.len() - 1 {
                    self.flush_line();
                    // Add a newline message
                    self.messages
                        .borrow_mut()
                        .push(WasmDisplayMessage::Print("\n".to_string()));
                }
            }
        } else {
            self.current_line.borrow_mut().push_str(text);
        }
        Ok(())
    }

    fn print_char(&mut self, ch: char) -> Result<(), DisplayError> {
        if ch == '\n' {
            self.flush_line();
            self.messages
                .borrow_mut()
                .push(WasmDisplayMessage::Print("\n".to_string()));
        } else {
            self.current_line.borrow_mut().push(ch);
        }
        Ok(())
    }

    fn erase_window(&mut self, window: i16) -> Result<(), DisplayError> {
        self.flush_line();
        self.messages
            .borrow_mut()
            .push(WasmDisplayMessage::EraseWindow(window));
        Ok(())
    }

    fn show_status(&mut self, location: &str, score: i16, moves: u16) -> Result<(), DisplayError> {
        self.messages
            .borrow_mut()
            .push(WasmDisplayMessage::StatusUpdate {
                location: location.to_string(),
                score,
                moves,
            });
        Ok(())
    }

    fn handle_resize(&mut self, width: u16, height: u16) {
        self.terminal_size = (width, height);
    }

    fn erase_line(&mut self) -> Result<(), DisplayError> {
        // For WASM, we just clear the current line buffer
        self.current_line.borrow_mut().clear();
        Ok(())
    }

    fn get_cursor(&mut self) -> Result<(u16, u16), DisplayError> {
        Ok(self.cursor)
    }

    fn set_buffer_mode(&mut self, _buffered: bool) -> Result<(), DisplayError> {
        // WASM is always buffered
        Ok(())
    }

    fn set_text_style(&mut self, style: u16) -> Result<(), DisplayError> {
        self.messages
            .borrow_mut()
            .push(WasmDisplayMessage::SetTextStyle(style));
        Ok(())
    }

    fn get_terminal_size(&self) -> (u16, u16) {
        self.terminal_size
    }

    fn force_refresh(&mut self) -> Result<(), DisplayError> {
        self.flush_line();
        Ok(())
    }
}
