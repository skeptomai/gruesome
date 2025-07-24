//! V4+ input handling - complex character and line input
//!
//! V4+ games like AMFV use both read_char and sread instructions.
//! This requires careful terminal control and proper input sequencing.

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{self, DisableLineWrap, EnableLineWrap},
    execute,
};
use log::debug;
use std::io;
use std::time::{Duration, Instant};
use crate::display_trait::ZMachineDisplay;

pub struct V4Input {
    /// Whether we're currently in raw mode
    in_raw_mode: bool,
    /// Current line being built
    line_buffer: String,
    /// Cursor position in line buffer
    cursor_pos: usize,
}

impl V4Input {
    pub fn new() -> Self {
        V4Input {
            in_raw_mode: false,
            line_buffer: String::new(),
            cursor_pos: 0,
        }
    }

    /// Read a single character for V4+ games (read_char instruction)
    pub fn read_char<F>(
        &mut self,
        time_tenths: u16,
        _routine_addr: u16,
        timer_callback: Option<F>,
    ) -> Result<(char, bool), String>
    where
        F: FnMut() -> Result<bool, String>,
    {
        debug!(
            "V4 input: reading character (time={}s, routine=0x{:04x})",
            time_tenths as f32 / 10.0,
            _routine_addr
        );

        // For V4+ character input, we need proper terminal control
        if !atty::is(atty::Stream::Stdin) {
            // Piped input - read first character from line
            debug!("V4 input: piped mode, reading character from line");
            return self.read_char_from_line();
        }

        // Interactive terminal - use event-driven input
        debug!("V4 input: interactive mode, using terminal events");
        self.read_char_interactive(time_tenths, _routine_addr, timer_callback)
    }

    /// Read a line for V4+ games (sread instruction)
    pub fn read_line<F>(
        &mut self,
        time_tenths: u16,
        _routine_addr: u16,
        timer_callback: Option<F>,
        display: &mut dyn ZMachineDisplay,
    ) -> Result<(String, bool), String>
    where
        F: FnMut() -> Result<bool, String>,
    {
        debug!(
            "V4 input: reading line (time={}s, routine=0x{:04x})",
            time_tenths as f32 / 10.0,
            _routine_addr
        );

        if !atty::is(atty::Stream::Stdin) {
            // Piped input - simple line reading
            debug!("V4 input: piped mode, reading line from stdin");
            return self.read_line_from_stdin();
        }

        // Interactive terminal - use event-driven line input
        debug!("V4 input: interactive mode, using terminal line input");
        self.read_line_interactive(time_tenths, _routine_addr, timer_callback, display)
    }

    /// Read character from piped input
    fn read_char_from_line(&mut self) -> Result<(char, bool), String> {
        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => {
                // EOF - return newline as default
                debug!("V4 input: EOF on character input, returning newline");
                Ok(('\n', false))
            }
            Ok(_) => {
                let ch = line.chars().next().unwrap_or('\n');
                debug!("V4 input: got character '{}' from piped input", ch);
                Ok((ch, false))
            }
            Err(e) => Err(format!("Failed to read character from pipe: {e}"))
        }
    }

    /// Read line from piped input
    fn read_line_from_stdin(&mut self) -> Result<(String, bool), String> {
        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => {
                // EOF
                debug!("V4 input: EOF on line input, returning empty");
                Ok((String::new(), false))
            }
            Ok(_) => {
                // Remove trailing newline
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }
                debug!("V4 input: got line '{}' from piped input", line);
                Ok((line, false))
            }
            Err(e) => Err(format!("Failed to read line from pipe: {e}"))
        }
    }

    /// Interactive character input using terminal events
    fn read_char_interactive<F>(
        &mut self,
        time_tenths: u16,
        _routine_addr: u16,
        mut timer_callback: Option<F>,
    ) -> Result<(char, bool), String>
    where
        F: FnMut() -> Result<bool, String>,
    {
        debug!("V4 input: starting interactive character input");

        // Enable raw mode for single character input
        terminal::enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {e}"))?;
        self.in_raw_mode = true;

        let timeout = if time_tenths > 0 {
            Some(Duration::from_millis((time_tenths as u64) * 100))
        } else {
            None
        };
        let start_time = Instant::now();

        let result = loop {
            // Check for timeout
            if let Some(timeout_duration) = timeout {
                if start_time.elapsed() >= timeout_duration {
                    debug!("V4 input: character input timeout");
                    if let Some(ref mut callback) = timer_callback {
                        match callback() {
                            Ok(terminate) => {
                                if terminate {
                                    debug!("V4 input: timer callback terminated character input");
                                    break Ok(('\0', true)); // Special value for timeout
                                }
                                // Continue waiting for input
                            }
                            Err(e) => {
                                break Err(format!("Timer callback error: {e}"));
                            }
                        }
                    }
                }
            }

            // Poll for events with timeout
            let poll_timeout = timeout.map(|_| Duration::from_millis(100))
                .unwrap_or(Duration::from_secs(3600));

            if event::poll(poll_timeout).map_err(|e| format!("Event poll error: {e}"))? {
                match event::read().map_err(|e| format!("Event read error: {e}"))? {
                    Event::Key(key_event) => {
                        if let Some(ch) = self.handle_char_key_event(key_event)? {
                            debug!("V4 input: got character '{}' from keyboard", ch);
                            break Ok((ch, false));
                        }
                    }
                    _ => {
                        // Ignore other events for character input
                    }
                }
            }
        };

        // Cleanup
        self.cleanup_raw_mode();
        result
    }

    /// Interactive line input using terminal events
    fn read_line_interactive<F>(
        &mut self,
        time_tenths: u16,
        _routine_addr: u16,
        mut timer_callback: Option<F>,
        display: &mut dyn ZMachineDisplay,
    ) -> Result<(String, bool), String>
    where
        F: FnMut() -> Result<bool, String>,
    {
        debug!("V4 input: starting interactive line input");

        // Enable raw mode for character-by-character control
        terminal::enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {e}"))?;
        self.in_raw_mode = true;
        execute!(io::stdout(), DisableLineWrap).ok();

        // Clear line buffer
        self.line_buffer.clear();
        self.cursor_pos = 0;

        let timeout = if time_tenths > 0 {
            Some(Duration::from_millis((time_tenths as u64) * 100))
        } else {
            None
        };
        let mut start_time = Instant::now();

        let result = loop {
            // Check for timeout
            if let Some(timeout_duration) = timeout {
                if start_time.elapsed() >= timeout_duration {
                    debug!("V4 input: line input timeout");
                    if let Some(ref mut callback) = timer_callback {
                        match callback() {
                            Ok(terminate) => {
                                if terminate {
                                    debug!("V4 input: timer callback terminated line input");
                                    break Ok((self.line_buffer.clone(), true));
                                } else {
                                    debug!("V4 input: timer callback continuing");
                                    start_time = Instant::now(); // Reset timer
                                }
                            }
                            Err(e) => {
                                break Err(format!("Timer callback error: {e}"));
                            }
                        }
                    }
                }
            }

            // Poll for events
            let poll_timeout = timeout.map(|_| Duration::from_millis(100))
                .unwrap_or(Duration::from_secs(3600));

            if event::poll(poll_timeout).map_err(|e| format!("Event poll error: {e}"))? {
                match event::read().map_err(|e| format!("Event read error: {e}"))? {
                    Event::Key(key_event) => {
                        if let Some(line) = self.handle_line_key_event(key_event, display)? {
                            debug!("V4 input: got line '{}' from keyboard", line);
                            break Ok((line, false));
                        }
                    }
                    Event::Paste(text) => {
                        // Handle pasted text with echo
                        for ch in text.chars() {
                            self.line_buffer.insert(self.cursor_pos, ch);
                            self.cursor_pos += 1;
                        }
                        // Echo pasted text to display immediately
                        display.print_input_echo(&text).ok();
                    }
                    _ => {
                        // Ignore other events
                    }
                }
            }
        };

        // Cleanup
        self.cleanup_raw_mode();
        execute!(io::stdout(), EnableLineWrap).ok();
        
        // Z-Machine spec 15.4 (read): "If input was terminated in the usual way, by the player 
        // typing a carriage return, then a carriage return is printed (so the cursor moves to the next line)"
        if let Ok((_, false)) = &result {
            display.print("\n").ok();
        }
        
        result
    }

    /// Handle key event for character input
    fn handle_char_key_event(&self, key: KeyEvent) -> Result<Option<char>, String> {
        match key.code {
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                    return Err("Interrupted by Ctrl+C".to_string());
                }
                Ok(Some(c))
            }
            KeyCode::Enter => Ok(Some('\n')),
            KeyCode::Esc => Ok(Some('\x1b')),
            KeyCode::Backspace => Ok(Some('\x08')),
            KeyCode::Tab => Ok(Some('\t')),
            _ => Ok(None) // Ignore other keys for character input
        }
    }

    /// Handle key event for line input
    fn handle_line_key_event(&mut self, key: KeyEvent, display: &mut dyn ZMachineDisplay) -> Result<Option<String>, String> {
        match key.code {
            KeyCode::Enter => {
                // Line complete
                Ok(Some(self.line_buffer.clone()))
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                    return Err("Interrupted by Ctrl+C".to_string());
                }

                // Add character to buffer
                self.line_buffer.insert(self.cursor_pos, c);
                self.cursor_pos += 1;

                // Echo character to display immediately (Z-Machine spec 7.1.1.1: input should be echoed)
                display.print_input_echo(&c.to_string()).ok();
                
                Ok(None)
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.line_buffer.remove(self.cursor_pos);

                    // Echo backspace to display immediately (backspace, space, backspace)
                    display.print_input_echo("\x08 \x08").ok();
                }
                Ok(None)
            }
            _ => Ok(None) // Ignore other keys
        }
    }

    /// Clean up raw mode
    fn cleanup_raw_mode(&mut self) {
        if self.in_raw_mode {
            let _ = terminal::disable_raw_mode();
            self.in_raw_mode = false;
        }
    }
}

impl Default for V4Input {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for V4Input {
    fn drop(&mut self) {
        self.cleanup_raw_mode();
    }
}