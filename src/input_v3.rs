//! V3 input handling - simple line-based input only
//!
//! V3 games like Zork only use sread (line input) and don't have read_char.
//! This makes the input model much simpler and more reliable.

use log::debug;
use std::io;

pub struct V3Input {
    /// Input buffer for building lines
    buffer: String,
}

impl V3Input {
    pub fn new() -> Self {
        V3Input {
            buffer: String::new(),
        }
    }

    /// Read a line of input for V3 games (sread instruction)
    ///
    /// V3 games only use line input, no character input complications
    pub fn read_line(&mut self) -> Result<String, String> {
        debug!("V3 input: reading line");

        // Simple line reading - works reliably for V3 games
        self.buffer.clear();
        io::stdin()
            .read_line(&mut self.buffer)
            .map_err(|e| format!("Failed to read line: {e}"))?;

        // Remove trailing newline
        if self.buffer.ends_with('\n') {
            self.buffer.pop();
            if self.buffer.ends_with('\r') {
                self.buffer.pop();
            }
        }

        debug!("V3 input received: '{}'", self.buffer);
        Ok(self.buffer.clone())
    }

    /// Read line with timer support for V3 games
    ///
    /// In V3, timers are simpler - they just fire once after input for turn counting
    pub fn read_line_with_timer<F>(
        &mut self,
        time_tenths: u16,
        routine_addr: u16,
        timer_callback: Option<F>,
    ) -> Result<(String, bool), String>
    where
        F: FnOnce() -> Result<bool, String>,
    {
        debug!(
            "V3 input: reading line with timer ({}s, routine=0x{:04x})",
            time_tenths as f32 / 10.0,
            routine_addr
        );

        // For V3 games, we use a simplified approach:
        // 1. Get input normally (blocking is fine for turn-based games)
        let input = self.read_line()?;

        // 2. After input, fire timer callback if present (for turn counting)
        if time_tenths > 0 && routine_addr > 0 {
            if let Some(callback) = timer_callback {
                debug!("V3 input: calling timer callback after input");
                let _result = callback()?;
                // For V3 games, timer result doesn't affect input continuation
            }
        }

        Ok((input, false)) // V3 timers don't terminate input
    }
}

impl Default for V3Input {
    fn default() -> Self {
        Self::new()
    }
}
