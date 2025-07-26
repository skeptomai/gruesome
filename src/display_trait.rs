//! Core display trait for Z-Machine display implementations
//!
//! This trait defines the interface that all display implementations must follow,
//! whether they're for v3, v4+, or headless testing.

use std::fmt;

/// Core trait for Z-Machine display operations
pub trait ZMachineDisplay {
    /// Clear the entire screen
    fn clear_screen(&mut self) -> Result<(), DisplayError>;

    /// Split the screen into upper and lower windows
    /// In v3: creates a status line
    /// In v4+: creates a multi-line upper window
    fn split_window(&mut self, lines: u16) -> Result<(), DisplayError>;

    /// Set the current window (0 = lower/main, 1 = upper)
    fn set_window(&mut self, window: u8) -> Result<(), DisplayError>;

    /// Set cursor position (1-based coordinates)
    /// In v3: typically only used for upper window
    /// In v4+: can be used for any window
    fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), DisplayError>;

    /// Print text to the current window
    fn print(&mut self, text: &str) -> Result<(), DisplayError>;

    /// Print a single character to the current window
    fn print_char(&mut self, ch: char) -> Result<(), DisplayError>;

    /// Erase a window (-1 = whole screen, 0 = lower, 1 = upper)
    fn erase_window(&mut self, window: i16) -> Result<(), DisplayError>;

    /// Handle terminal resize
    fn handle_resize(&mut self, width: u16, height: u16);

    // V3-specific operations (no-op for v4+)

    /// Show status line (v3 only)
    /// For v4+, this should be a no-op as games manage their own status
    fn show_status(&mut self, location: &str, score: i16, moves: u16) -> Result<(), DisplayError>;

    // V4+ specific operations (no-op or error for v3)

    /// Erase from cursor to end of line (v4+)
    fn erase_line(&mut self) -> Result<(), DisplayError>;

    /// Get current cursor position (v4+)
    /// Returns (line, column) with 1-based indexing
    fn get_cursor(&mut self) -> Result<(u16, u16), DisplayError>;

    /// Set buffer mode (v4+)
    fn set_buffer_mode(&mut self, buffered: bool) -> Result<(), DisplayError>;

    /// Set text style (0 = normal, 1 = reverse, 2 = bold, 4 = italic, 8 = fixed)
    /// Multiple styles can be combined with bitwise OR
    fn set_text_style(&mut self, style: u16) -> Result<(), DisplayError>;

    /// Print input echo immediately (for real-time feedback during input)
    fn print_input_echo(&mut self, text: &str) -> Result<(), DisplayError> {
        // Default implementation just calls print
        self.print(text)
    }

    // Utility methods

    /// Get the current terminal dimensions
    fn get_terminal_size(&self) -> (u16, u16);

    /// Force a display refresh (mainly for debugging)
    fn force_refresh(&mut self) -> Result<(), DisplayError>;
}

/// Display error type
#[derive(Debug, Clone)]
pub struct DisplayError {
    pub message: String,
}

impl DisplayError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for DisplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Display error: {}", self.message)
    }
}

impl std::error::Error for DisplayError {}

impl From<std::io::Error> for DisplayError {
    fn from(error: std::io::Error) -> Self {
        Self::new(format!("I/O error: {}", error))
    }
}

impl From<DisplayError> for String {
    fn from(error: DisplayError) -> String {
        error.message
    }
}
