//! Logging wrapper for display implementations
//!
//! This wrapper logs every single display operation to help debug display issues.

use crate::display_trait::{DisplayError, ZMachineDisplay};
use log::{debug, info};

pub struct LoggingDisplay {
    inner: Box<dyn ZMachineDisplay>,
    op_count: usize,
}

impl LoggingDisplay {
    pub fn new(inner: Box<dyn ZMachineDisplay>) -> Self {
        info!("=== DISPLAY LOGGING STARTED ===");
        Self { inner, op_count: 0 }
    }

    fn log_op(&mut self, op: &str) {
        self.op_count += 1;
        info!("[OP {:04}] {}", self.op_count, op);
    }
}

impl ZMachineDisplay for LoggingDisplay {
    fn clear_screen(&mut self) -> Result<(), DisplayError> {
        self.log_op("clear_screen()");
        self.inner.clear_screen()
    }

    fn split_window(&mut self, lines: u16) -> Result<(), DisplayError> {
        self.log_op(&format!("split_window({})", lines));
        self.inner.split_window(lines)
    }

    fn set_window(&mut self, window: u8) -> Result<(), DisplayError> {
        self.log_op(&format!("set_window({})", window));
        self.inner.set_window(window)
    }

    fn set_cursor(&mut self, line: u16, column: u16) -> Result<(), DisplayError> {
        self.log_op(&format!("set_cursor({}, {})", line, column));
        self.inner.set_cursor(line, column)
    }

    fn print(&mut self, text: &str) -> Result<(), DisplayError> {
        let preview = text
            .chars()
            .take(50)
            .collect::<String>()
            .replace('\n', "\\n")
            .replace('\r', "\\r");
        self.log_op(&format!("print('{}')", preview));
        self.inner.print(text)
    }

    fn print_char(&mut self, ch: char) -> Result<(), DisplayError> {
        self.log_op(&format!("print_char('{}')", ch));
        self.inner.print_char(ch)
    }

    fn erase_window(&mut self, window: i16) -> Result<(), DisplayError> {
        self.log_op(&format!("erase_window({})", window));
        self.inner.erase_window(window)
    }

    fn handle_resize(&mut self, width: u16, height: u16) {
        self.log_op(&format!("handle_resize({}, {})", width, height));
        self.inner.handle_resize(width, height)
    }

    fn show_status(&mut self, location: &str, score: i16, moves: u16) -> Result<(), DisplayError> {
        self.log_op(&format!(
            "show_status('{}', {}, {})",
            location, score, moves
        ));
        self.inner.show_status(location, score, moves)
    }

    fn set_text_style(&mut self, style: u16) -> Result<(), DisplayError> {
        self.log_op(&format!("set_text_style({})", style));
        self.inner.set_text_style(style)
    }

    fn erase_line(&mut self) -> Result<(), DisplayError> {
        self.log_op("erase_line()");
        self.inner.erase_line()
    }

    fn get_cursor(&mut self) -> Result<(u16, u16), DisplayError> {
        self.log_op("get_cursor()");
        self.inner.get_cursor()
    }

    fn set_buffer_mode(&mut self, buffered: bool) -> Result<(), DisplayError> {
        self.log_op(&format!("set_buffer_mode({})", buffered));
        self.inner.set_buffer_mode(buffered)
    }

    fn get_terminal_size(&self) -> (u16, u16) {
        let size = self.inner.get_terminal_size();
        debug!("get_terminal_size() -> {:?}", size);
        size
    }

    fn force_refresh(&mut self) -> Result<(), DisplayError> {
        self.log_op("force_refresh()");
        self.inner.force_refresh()
    }
}

impl Drop for LoggingDisplay {
    fn drop(&mut self) {
        info!(
            "=== DISPLAY LOGGING ENDED ({} operations) ===",
            self.op_count
        );
    }
}
